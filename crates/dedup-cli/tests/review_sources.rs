use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Inputs(PathBuf);

impl Inputs {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("review-{name}-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }

    fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_dedup"))
            .current_dir(&self.0)
            .arg("review-ids")
            .args(args)
            .output()
            .unwrap()
    }

    fn export(&self, directory: &str, sources: &[&str]) -> Output {
        self.run(
            &[
                &["--export-reviewed", "choices.tsv", directory][..],
                sources,
            ]
            .concat(),
        )
    }
}

impl Drop for Inputs {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn named_sources_preserve_identical_exports_local_ordinals_and_empty_inputs() {
    let inputs = Inputs::new("provenance");
    let duplicate = b"TY  - JOUR\rDO  - 10.0000/example\rUR  - https://pubmed.ncbi.nlm.nih.gov/fixture/\rER  -\r";
    let other = b"TY  - BOOK\nER  -\nTY  - JOUR\nDO  - 10.0000/single\nER  -\n";
    inputs.write("first/export.ris", duplicate);
    inputs.write("second folder/export.ris", duplicate);
    inputs.write("other.ris", other);
    inputs.write("empty.ris", b"");
    let mut entries = [
        ["--ris-source", "right", "second folder/export.ris"],
        ["--ris-source", "empty", "empty.ris"],
        ["--ris-source", "left", "first/export.ris"],
        ["--ris-source", "other", "other.ris"],
    ];
    let output = inputs.run(&entries.concat());
    assert!(output.status.success(), "{:?}", output.stderr);
    entries.reverse();
    let reversed = inputs.run(&entries.concat());
    assert!(reversed.status.success(), "{:?}", reversed.stderr);
    assert_eq!(output.stdout, reversed.stdout);

    let text = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert!(
        lines.contains(&"summary\tall\t4\tretained=4;suppressed=0;candidate_records=2;groups=1")
    );
    assert!(lines.contains(&"candidate\tdoi\tleft:1,right:1\tother_id_incomplete"));
    assert!(lines.contains(&"issue\tinput\tother:1\tmissing_identifiers"));
    assert!(lines.contains(&"input\tris\tempty\tbytes=0;occurrences=0;sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
    for label in ["left", "right"] {
        let manifest = format!(
            "input\tris\t{label}\tbytes=86;occurrences=1;sha256=843148056495410a879838089edee836e63ca1c3c8c7522317c2a740ed937512"
        );
        let record = format!(
            "source\tris\t{label}:1\tbytes=0..86;status=complete;closed=true;malformed_lines=0;fields=4;unmapped_fields=0;disposition=review_candidate"
        );
        assert!(lines.contains(&format!("issue\tinput\t{label}:1\tunsupported_pmid").as_str()));
        assert!(lines.contains(&manifest.as_str()));
        assert!(lines.contains(&record.as_str()));
    }
    let dispositions = lines
        .iter()
        .filter(|line| line.starts_with("source\tris\tother:"))
        .map(|line| line.rsplit_once("disposition=").unwrap().1)
        .collect::<Vec<_>>();
    assert_eq!(dispositions, ["review_input", "retained"]);
    assert!(!text.contains("export.ris") && !text.contains("10.0000/"));
    for path in ["first/export.ris", "second folder/export.ris"] {
        assert_eq!(fs::read(inputs.0.join(path)).unwrap(), duplicate);
    }
    assert_eq!(fs::read(inputs.0.join("other.ris")).unwrap(), other);
}

fn assert_failed_before_report(output: Output) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn damaged_named_sources_fail_before_queue_or_sheet() {
    let inputs = Inputs::new("structure");
    let good = "TY  - JOUR\r\nER  -\r\n";
    inputs.write("good.ris", good.as_bytes());
    let sources = [
        "--ris-source",
        "a",
        "good.ris",
        "--ris-source",
        "b",
        "renamed.ris",
    ];
    let bib = "@article{fixture1, title={Canopy}}\n@article{fixture2, title={Shade}}\n@book{fixture3, title={Sun}}\n";
    for (prefix, damaged, suffix, ordinal, status) in [
        ("", bib, "", 1, "fragment"),
        (good, "outside records\n", good, 2, "fragment"),
        (good, "TY  - JOUR\nTI  - Unclosed\n", good, 2, "partial"),
        (
            good,
            "TY  - JOUR\nUnexpected line\nER  -\n",
            good,
            2,
            "malformed",
        ),
    ] {
        let data = format!("{prefix}{damaged}{suffix}");
        inputs.write("renamed.ris", data.as_bytes());
        for action in [
            &[][..],
            &["--init-decisions", "new.tsv"],
            &["--init-metadata-decisions", "new.tsv"],
        ] {
            let failed = inputs.run(&[action, &sources].concat());
            let error = String::from_utf8_lossy(&failed.stderr);
            assert!(
                error.contains(&format!(
                    "b:{ordinal} (bytes {}..{}, {status})",
                    prefix.len(),
                    prefix.len() + damaged.len()
                )),
                "{error}"
            );
            assert!(
                error.contains("--ris <file>")
                    && error.contains("Bring a BibTeX export into a named RIS review")
            );
            assert!(
                !error.contains("renamed.ris") && !damaged.lines().any(|line| error.contains(line))
            );
            assert_failed_before_report(failed);
            assert!(!inputs.0.join("new.tsv").exists());
        }
        assert_eq!(
            fs::read(inputs.0.join("renamed.ris")).unwrap(),
            data.as_bytes()
        );
    }
}

#[test]
fn interrupted_field_joined_to_a_bom_export_fails_before_sheet_or_export() {
    let inputs = Inputs::new("interrupted-join");
    let good =
        "\u{feff}TY  - JOUR\r\nTI  - Literal TY  - JOUR\r\nN1  - Pre\u{feff}face\r\nER  -\r\n";
    inputs.write("catalog.ris", good.as_bytes());
    inputs.write("index.ris", good.as_bytes());
    let sources = [
        "--ris-source",
        "catalog",
        "catalog.ris",
        "--ris-source",
        "index",
        "index.ris",
    ];
    let init = |sheet| inputs.run(&[&["--init-decisions", sheet][..], &sources].concat());
    assert!(init("choices.tsv").status.success());
    let sheet = fs::read(inputs.0.join("choices.tsv")).unwrap();

    let cut = "TY  - JOUR\nTI  - Enquêt";
    let joined = format!("{good}{cut}\u{feff}TY  - JOUR\nTI  - Follow-up\nER  -\n");
    inputs.write("index.ris", joined.as_bytes());
    for failed in [init("new.tsv"), inputs.export("out", &sources)] {
        let error = String::from_utf8_lossy(&failed.stderr);
        assert!(
            error.contains(&format!(
                "index:2 (bytes {}..{}, malformed)",
                good.len(),
                joined.len()
            )),
            "{error}"
        );
        assert!(error.contains(&format!(
            "TY header inside the file at byte {}",
            good.len() + cut.len()
        )));
        assert!(!error.contains("Enquêt") && !error.contains("Follow-up"));
        assert_failed_before_report(failed);
    }
    assert!(!inputs.0.join("new.tsv").exists() && !inputs.0.join("out").exists());
    assert_eq!(fs::read(inputs.0.join("choices.tsv")).unwrap(), sheet);
    assert_eq!(
        fs::read(inputs.0.join("index.ris")).unwrap(),
        joined.as_bytes()
    );
    assert_eq!(
        fs::read(inputs.0.join("catalog.ris")).unwrap(),
        good.as_bytes()
    );
}

#[test]
fn converted_ris_and_blank_sources_can_be_reviewed_together() {
    let inputs = Inputs::new("converted");
    let legacy = b"@article{CanopyNotes, title={Canop\xe9e, light and shade}, author={Fen, Uma}, year={2022}, journal={Fictional Shore Botany}}\n";
    inputs.write("original.bib", legacy);
    // A synthetic bibutils 7.2 conversion, with the file BOM and mixed line endings.
    let converted = "\u{feff}TY  - JOUR\r\nAU  - Fen, Uma\rPY  - 2022\nDA  - 2022//\nTI  - Canopée, light and shade\nJO  - Fictional Shore Botany\nID  - CanopyNotes\nER  - \n";
    inputs.write("converted.data", converted.as_bytes());
    inputs.write("empty.ris", b"");
    inputs.write("blank.ris", "\u{feff}\t \r\n\n".as_bytes());
    let sources = |file| {
        [
            "--ris-source",
            "bib",
            file,
            "--ris-source",
            "empty",
            "empty.ris",
            "--ris-source",
            "blank",
            "blank.ris",
        ]
    };
    let init_sheet =
        |file| inputs.run(&[&["--init-decisions", "choices.tsv"][..], &sources(file)].concat());
    let failed = init_sheet("original.bib");
    let message = String::from_utf8_lossy(&failed.stderr);
    assert!(message.starts_with("bib: Input is not UTF-8"));
    assert!(message.contains("separate UTF-8 RIS export") && message.contains("exact UTF-8 RIS"));
    assert!(message.contains("Bring a BibTeX export into a named RIS review"));
    assert!(!message.contains("original.bib") && !message.contains("CanopyNotes"));
    assert_eq!(failed.status.code(), Some(2));
    assert_failed_before_report(failed);
    assert!(!inputs.0.join("choices.tsv").exists());
    let init = init_sheet("converted.data");
    assert!(init.status.success(), "{:?}", init.stderr);
    let text = String::from_utf8(init.stdout).unwrap();
    assert!(text.contains("summary\tall\t1\tretained=1;suppressed=0;"));
    for (label, bytes) in [("empty", 0), ("blank", 8)] {
        assert!(text.contains(&format!(
            "input\tris\t{label}\tbytes={bytes};occurrences=0;"
        )));
    }
    assert!(
        inputs
            .export("out", &sources("converted.data"))
            .status
            .success()
    );
    let expected = converted
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    assert_eq!(
        fs::read(inputs.0.join("out/reviewed.ris")).unwrap(),
        expected.as_bytes()
    );
    assert_eq!(
        fs::read(inputs.0.join("converted.data")).unwrap(),
        converted.as_bytes()
    );
    assert_eq!(fs::read(inputs.0.join("original.bib")).unwrap(), legacy);
}

#[test]
fn decisions_reopen_against_named_bytes_and_can_be_reversed() {
    use sha2::{Digest, Sha256};
    let inputs = Inputs::new("reopen");
    let catalog =
        b"TY  - JOUR\r\nTI  - Lantern study results\r\nDO  - 10.0000/lantern\r\nER  -\r\n";
    // The protocol of this synthetic study has a copied DOI. It is a separate report.
    let index = b"TY  - JOUR\nTI  - Lantern study results\nDO  - 10.0000/lantern\nER  -\nTY  - JOUR\nTI  - Lantern study protocol\nDO  - 10.0000/lantern\nER  -\n";
    inputs.write("catalog.ris", catalog);
    inputs.write("index.ris", index);
    let sources = [
        "--ris-source",
        "catalog",
        "catalog.ris",
        "--ris-source",
        "index",
        "index.ris",
    ];
    let run = |flag, path| inputs.run(&[&[flag, path][..], &sources].concat());
    let init = run("--init-decisions", "review.tsv");
    assert!(init.status.success(), "{:?}", init.stderr);
    assert_eq!(init.stdout, inputs.run(&sources).stdout);
    let template = fs::read_to_string(inputs.0.join("review.tsv")).unwrap();
    assert!(template.ends_with("catalog:1\tpending\nindex:1\tpending\nindex:2\tpending\nend\n"));
    let reviewed = template
        .replace("catalog:1\tpending", "catalog:1\tkeep")
        .replace("index:1\tpending", "index:1\tduplicate-of=catalog:1")
        .replace("index:2\tpending", "index:2\tkeep");
    inputs.write("review.tsv", reviewed.replace('\n', "\r\n").as_bytes());
    let initial = &sources[..3];
    assert!(
        inputs
            .run(&[&["--init-decisions", "initial.tsv"][..], initial].concat())
            .status
            .success()
    );
    for (sheet, selected, diagnostic) in [
        (
            "review.tsv",
            initial,
            "Saved source 'index' is missing from the selection.",
        ),
        (
            "initial.tsv",
            &sources[..],
            "Selected source 'index' is absent from the sheet.",
        ),
    ] {
        let refused =
            inputs.run(&[&["--export-reviewed", sheet, "wrong-wave"][..], selected].concat());
        assert!(String::from_utf8_lossy(&refused.stderr).contains(diagnostic));
        assert_failed_before_report(refused);
        assert!(!inputs.0.join("wrong-wave").exists());
    }
    assert_failed_before_report(run("--init-decisions", "catalog.ris"));
    assert_failed_before_report(run("--init-decisions", "review.tsv"));
    let reopened = run("--resume-decisions", "review.tsv");
    assert!(reopened.status.success(), "{:?}", reopened.stderr);
    let report = String::from_utf8(reopened.stdout).unwrap();
    assert!(report.starts_with(std::str::from_utf8(&init.stdout).unwrap()));
    assert!(report.ends_with("decision\thuman\tcatalog:1\tkeep\ndecision\thuman\tindex:1\tduplicate-of=catalog:1\ndecision\thuman\tindex:2\tkeep\n"));
    inputs.write("moved.ris", catalog);
    let moved = inputs.run(&[
        "--resume-decisions",
        "review.tsv",
        "--ris-source",
        "index",
        "index.ris",
        "--ris-source",
        "catalog",
        "moved.ris",
    ]);
    assert!(moved.status.success());
    assert_eq!(moved.stdout, report.as_bytes());

    let reversed = reviewed.replace("duplicate-of=catalog:1", "pending");
    inputs.write("review.tsv", reversed.as_bytes());
    let reopened = run("--resume-decisions", "review.tsv");
    assert!(reopened.status.success());
    assert_eq!(
        reopened.stdout,
        report
            .replace("duplicate-of=catalog:1", "pending")
            .as_bytes()
    );
    assert_eq!(fs::read(inputs.0.join("index.ris")).unwrap(), index);
    assert_eq!(fs::read(inputs.0.join("catalog.ris")).unwrap(), catalog);

    // Same length, identifiers, occurrence count and candidate queue, different bytes.
    inputs.write(
        "catalog.ris",
        std::str::from_utf8(catalog)
            .unwrap()
            .replace("Lantern", "Inkdrop")
            .as_bytes(),
    );
    let stale = run("--resume-decisions", "review.tsv");
    let saved_hash = format!("{:x}", Sha256::digest(catalog));
    assert!(String::from_utf8_lossy(&stale.stderr).contains(&format!(
        "Source 'catalog' has a different SHA-256 from the saved value {saved_hash}."
    )));
    assert_failed_before_report(stale);
    assert_eq!(
        fs::read(inputs.0.join("review.tsv")).unwrap(),
        reversed.as_bytes()
    );
    inputs.write(
        "review.tsv",
        reversed.replace(&saved_hash, "damaged-cell").as_bytes(),
    );
    let damaged = run("--resume-decisions", "review.tsv");
    let error = String::from_utf8_lossy(&damaged.stderr);
    assert!(error.contains("Check all RIS labels") && !error.contains("damaged-cell"));
    assert_failed_before_report(damaged);
}

#[test]
fn invalid_decisions_and_indirect_targets_emit_no_report() {
    let inputs = Inputs::new("invalid-sheet");
    // A-B share a DOI and B-C share a PMID. A-C share no direct group.
    inputs.write("bridge.ris", b"TY  - JOUR\nDO  - 10.0000/bridge\nER  -\nTY  - JOUR\nDO  - 10.0000/bridge\nUR  - https://pubmed.ncbi.nlm.nih.gov/777/\nER  -\nTY  - JOUR\nUR  - https://pubmed.ncbi.nlm.nih.gov/777/\nER  -\n");
    let run = |flag, label| inputs.run(&[flag, "sheet.tsv", "--ris-source", label, "bridge.ris"]);
    assert!(run("--init-decisions", "s").status.success());
    let sheet = fs::read_to_string(inputs.0.join("sheet.tsv")).unwrap();
    assert_failed_before_report(run("--resume-decisions", "renamed"));
    for invalid in [
        sheet.replace("decisions\t1", "decisions\t9"),
        sheet.replace("end\n", ""),
        sheet.replace("end\n", "s:1\tkeep\nend\n"),
        sheet.replace("s:2\tpending\n", ""),
        sheet.replace("s:2\tpending", "s:2\tunknown"),
        sheet.replace("s:2\tpending", "s:2\tduplicate-of=s:1"),
        sheet
            .replace("s:1\tpending", "s:1\tduplicate-of=s:3")
            .replace("s:3\tpending", "s:3\tkeep"),
    ] {
        inputs.write("sheet.tsv", invalid.as_bytes());
        assert_failed_before_report(run("--resume-decisions", "s"));
    }
}

#[test]
fn reviewed_export_reconciles_every_occurrence_and_replays_byte_for_byte() {
    use sha2::{Digest, Sha256};
    let inputs = Inputs::new("export");
    let duplicate = "TY  - JOUR\r\nTI  - Violet results\r\nDO  - 10.0000/violet\r\nER  -\r\n";
    let protocol = "TY  - JOUR\rTI  - Violet protocol\rDO  - 10.0000/violet\rER  -\r";
    let pending = "TY  - JOUR\nTI  - Unresolved entry\nDO  - 10.0000/violet\nER  -";
    let kept = "\u{feff}TY  - JOUR\r\nTI  - Violet results\r\nDO  - 10.0000/violet\r\nZ9  - été\r\n  continued\r\nZ9  - repeated\r\nER  -\r\n";
    let other = "TY  - BOOK\nTI  - A separate work\nER  -";
    let alpha = format!("{duplicate}{protocol}{pending}");
    let beta = format!("{kept}{other}");
    inputs.write("a.ris", alpha.as_bytes());
    inputs.write("b.ris", beta.as_bytes());
    inputs.write("empty.ris", b"");
    let a = ["--ris-source", "a", "a.ris"];
    let b = ["--ris-source", "b", "b.ris"];
    let empty = ["--ris-source", "total", "empty.ris"];
    let sources = [a, b, empty].concat();
    let init = inputs.run(&[&["--init-decisions", "choices.tsv"][..], &sources].concat());
    assert!(init.status.success(), "{:?}", init.stderr);
    let choices = fs::read_to_string(inputs.0.join("choices.tsv"))
        .unwrap()
        .replace("a:1\tpending", "a:1\tduplicate-of=b:1")
        .replace("a:2\tpending", "a:2\tkeep")
        .replace("b:1\tpending", "b:1\tkeep");
    inputs.write("choices.tsv", choices.as_bytes());
    for (dir, args) in [
        ("first run", sources.clone()),
        ("second run", [empty, b, a].concat()),
    ] {
        let run = inputs.export(dir, &args);
        assert!(run.status.success(), "{:?}", run.stderr);
        assert!(run.stdout.is_empty());
    }
    let read = |dir: &str, file: &str| fs::read_to_string(inputs.0.join(dir).join(file)).unwrap();
    let ris = read("first run", "reviewed.ris");
    let map = read("first run", "mapping.tsv");
    assert_eq!(ris, read("second run", "reviewed.ris"));
    assert_eq!(map, read("second run", "mapping.tsv"));
    let tally = read("first run", "reconciliation.tsv");
    assert_eq!(tally, read("second run", "reconciliation.tsv"));
    assert_eq!(
        tally.lines().skip(1).collect::<Vec<_>>(),
        [
            "source\ta\t3\t1\t0\t2\t1\t1\t0\t0\t0",
            "source\tb\t2\t0\t0\t2\t1\t0\t0\t1\t0",
            "source\ttotal\t0\t0\t0\t0\t0\t0\t0\t0\t0",
            "total\t\t5\t1\t0\t4\t2\t1\t0\t1\t0",
        ]
    );
    let expected_ris = format!(
        "{}{pending}\n{}{other}\n",
        protocol.replace('\r', "\n"),
        kept.trim_start_matches('\u{feff}').replace("\r\n", "\n")
    );
    assert_eq!(ris, expected_ris);
    let reopened = inputs.run(&["--ris", "first run/reviewed.ris"]);
    assert!(reopened.status.success(), "{:?}", reopened.stderr);
    assert!(std::str::from_utf8(&reopened.stdout).unwrap().lines().any(
        |line| line == "summary\tall\t4\tretained=4;suppressed=0;candidate_records=3;groups=1"
    ));
    let rows: Vec<_> = map
        .lines()
        .filter(|line| line.starts_with("map\t"))
        .collect();
    let boundary = duplicate.len() + protocol.len();
    assert_eq!(
        rows,
        [
            format!(
                "map\tduplicate-of=b:1\ta:1\tbytes=0..{};output=3",
                duplicate.len()
            ),
            format!(
                "map\tkeep\ta:2\tbytes={}..{boundary};output=1",
                duplicate.len()
            ),
            format!(
                "map\tpending\ta:3\tbytes={boundary}..{};output=2",
                alpha.len()
            ),
            format!("map\tkeep\tb:1\tbytes=0..{};output=3", kept.len()),
            format!(
                "map\tnot-candidate\tb:2\tbytes={}..{};output=4",
                kept.len(),
                beta.len()
            ),
        ]
    );
    assert!(
        map.lines()
            .any(|line| line == "summary\tall\t5\tretained=4;suppressed=1")
    );
    assert!(map.contains(&format!(
        "output\tris\treviewed.ris\tbytes={};sha256={:x}\n",
        ris.len(),
        Sha256::digest(&ris)
    )));
    for line in choices.lines().filter(|line| line.starts_with("input\t")) {
        assert!(map.lines().any(|mapped| mapped == line));
    }
    assert_eq!(read("", "choices.tsv"), choices);
    inputs.write(
        "choices.tsv",
        choices.replace("duplicate-of=b:1", "pending").as_bytes(),
    );
    assert!(inputs.export("reversed", &sources).status.success());
    assert_eq!(
        read("reversed", "reviewed.ris"),
        format!("{}{ris}", duplicate.replace("\r\n", "\n"))
    );
    assert_eq!(read("", "a.ris"), alpha);
    assert_eq!(read("", "b.ris"), beta);
    assert_eq!(read("first run", "reviewed.ris"), ris);
}

#[test]
fn selected_pair_compares_complete_abstracts_and_leaves_ambiguous_fields_uncompared() {
    let inputs = Inputs::new("inspect-abstract");
    let abstracts = [
        "AB  - We surveyed the inlet.\n      Initial census.",
        "N2  - We surveyed\n      the inlet. Initial census.",
        "N2  - We surveyed the inlet.\n      Return census.",
        "AB  - We surveyed the inlet.",
        "KW  - Inlet",
        "AB  -",
        "AB  - We surveyed the inlet.\nN2  - Another abstract.",
    ];
    let ris: String = abstracts
        .iter()
        .map(|ab| format!("TY  - CONF\nTI  - Inlet study\nDO  - 10.0000/inlet\n{ab}\nER  -\n"))
        .collect();
    inputs.write("posters.ris", ris.replace('\n', "\r\n").as_bytes());
    let sources = ["--ris-source", "p", "posters.ris"];
    assert!(
        inputs
            .run(&[&["--init-decisions", "choices.tsv"][..], &sources].concat())
            .status
            .success()
    );
    for (right, expected) in [
        ("p:2", "same wording (spacing ignored)"),
        (
            "p:3",
            "first difference at word 5  \"Initial\" | \"Return\"",
        ),
        (
            "p:4",
            "first difference at word 5  \"Initial\" | (end of abstract)",
        ),
        ("p:5", "present | absent"),
        ("p:6", "present | empty"),
        ("p:7", "present | multiple fields (see original)"),
    ] {
        let comparison = inputs.run(
            &[
                &["--inspect-pair", "choices.tsv", "p:1", right][..],
                &sources,
            ]
            .concat(),
        );
        assert!(comparison.status.success(), "{:?}", comparison.stderr);
        let text = String::from_utf8(comparison.stdout).unwrap();
        let summary = text.split_once("[p:1]").unwrap().0;
        assert!(
            summary
                .lines()
                .any(|line| line == format!("  Abstract text (AB/N2)  {expected}")),
            "{summary}"
        );
    }
}

#[test]
fn asreview_companion_preserves_the_selected_occurrences_and_field_values() {
    use sha2::{Digest, Sha256};
    let inputs = Inputs::new("asreview-handoff");
    let catalog = "TY  - JOUR\nTI  - Clock enamel\nM3  - Research Article\nAN  - C-0007\nDO  - 10.0000/enamel\nER  -\n";
    let follow = "TY  - CONF\nTI  - Second winter\nAB  - Follow-up survey.\nAN  - C-0009\nDO  - 10.0000/enamel\nER  -\n";
    let register = "TY  -\tJOUR\nT1  - Clock enamel\nN2  -\tPremière visite.\n      The tag text N2  - stays here.\nN1  -\nID  - R-0012\nDO  - 10.0000/enamel\nER  -\n";
    inputs.write(
        "cat.ris",
        format!("{catalog}{follow}")
            .replace('\n', "\r\n")
            .as_bytes(),
    );
    inputs.write(
        "reg.ris",
        format!("\u{feff}{register}")
            .trim_end()
            .replace('\n', "\r")
            .as_bytes(),
    );
    let sources = [
        "--ris-source",
        "cat",
        "cat.ris",
        "--ris-source",
        "reg",
        "reg.ris",
    ];
    let option = [&["--asreview-ris"][..], &sources].concat();
    let init = inputs.run(&[&["--init-decisions", "choices.tsv"][..], &sources].concat());
    assert!(init.status.success());
    let choices = fs::read_to_string(inputs.0.join("choices.tsv"))
        .unwrap()
        .replace("reg:1\tpending", "reg:1\tkeep")
        .replace("cat:1\tpending", "cat:1\tduplicate-of=reg:1");
    inputs.write("choices.tsv", choices.as_bytes());
    assert!(inputs.export("ordinary", &sources).status.success());
    assert!(inputs.export("screen", &option).status.success());
    let read =
        |folder: &str, name: &str| fs::read_to_string(inputs.0.join(folder).join(name)).unwrap();
    for name in ["reviewed.ris", "mapping.tsv", "reconciliation.tsv"] {
        assert_eq!(read("ordinary", name), read("screen", name));
    }
    let original = format!("{follow}{register}");
    let expected = original
        .replace("\nT1  -", "\nTI  -")
        .replace("\nN2  -\t", "\nAB  - ")
        .replace("TY  -\t", "TY  - ")
        .replace("N1  -\n", "N1  - \n");
    assert_eq!(read("screen", "reviewed.ris"), original);
    assert_eq!(read("screen", "asreview.ris"), expected);
    assert_eq!(
        read("screen", "asreview.sha256"),
        format!(
            "{:x} *reviewed.ris\n{:x} *asreview.ris\n",
            Sha256::digest(original.as_bytes()),
            Sha256::digest(expected.as_bytes())
        )
    );
    assert!(read("screen", "mapping.tsv").contains("map\tpending\tcat:2\t"));
    assert_eq!(fs::read_dir(inputs.0.join("ordinary")).unwrap().count(), 3);
}

#[test]
fn asreview_companion_refuses_ambiguous_retained_fields_before_publication() {
    let inputs = Inputs::new("asreview-ambiguous");
    let source = ["--ris-source", "cat", "cat.ris"];
    for (i, fields) in [
        "TI  - First\nT1  - Second",
        "N2  - Part one\nN2  - Part two",
    ]
    .iter()
    .enumerate()
    {
        let record = format!("TY  - JOUR\n{fields}\nER  -\n");
        inputs.write("cat.ris", record.as_bytes());
        let sheet = format!("choices-{i}.tsv");
        assert!(
            inputs
                .run(&[&["--init-decisions", &sheet][..], &source].concat())
                .status
                .success()
        );
        let failed = inputs.run(
            &[
                &["--export-reviewed", &sheet, "screen", "--asreview-ris"][..],
                &source,
            ]
            .concat(),
        );
        assert!(
            String::from_utf8_lossy(&failed.stderr)
                .contains("Retained occurrence cat:1 has multiple")
        );
        assert_failed_before_report(failed);
        assert!(!inputs.0.join("screen").exists());
        let folder = format!("ordinary-{i}");
        assert!(
            inputs
                .run(&[&["--export-reviewed", &sheet, &folder][..], &source].concat())
                .status
                .success()
        );
        assert_eq!(
            fs::read(inputs.0.join(folder).join("reviewed.ris")).unwrap(),
            record.as_bytes()
        );
    }
}

#[test]
fn reviewed_export_rejects_invalid_inputs_and_preserves_existing_destinations() {
    let inputs = Inputs::new("export-errors");
    let record = "TY  - JOUR\nTI  - Tin\nDO  - 10.0000/tin\nER  -\n";
    let ris = record.repeat(2);
    inputs.write("src.ris", ris.as_bytes());
    let sources = ["--ris-source", "s", "src.ris"];
    let init = || inputs.run(&[&["--init-decisions", "choices.tsv"][..], &sources].concat());
    assert!(init().status.success());
    let sheet = fs::read_to_string(inputs.0.join("choices.tsv")).unwrap();
    for invalid in [
        sheet.replace("s:1\tpending", "s:1\tduplicate-of=s:2"),
        sheet.replace("s:1\tpending", "s:1\tduplicate-of=missing:1"),
        sheet.replace("end\n", ""),
    ] {
        inputs.write("choices.tsv", invalid.as_bytes());
        assert_failed_before_report(inputs.export("out", &sources));
        assert!(!inputs.0.join("out").exists());
    }
    inputs.write("choices.tsv", sheet.as_bytes());
    inputs.write("src.ris", ris.replace("Tin", "Tan").as_bytes());
    assert_failed_before_report(inputs.export("out", &sources));
    assert!(!inputs.0.join("out").exists());
    inputs.write("src.ris", ris.as_bytes());
    fs::create_dir(inputs.0.join("empty")).unwrap();
    inputs.write("existing/reviewed.ris", b"prior export");
    for dest in ["src.ris", "choices.tsv", "empty", "existing"] {
        assert_failed_before_report(inputs.export(dest, &sources));
    }
    assert_eq!(fs::read(inputs.0.join("src.ris")).unwrap(), ris.as_bytes());
    assert_eq!(
        fs::read(inputs.0.join("choices.tsv")).unwrap(),
        sheet.as_bytes()
    );
    assert_eq!(fs::read_dir(inputs.0.join("empty")).unwrap().count(), 0);
    assert_eq!(
        fs::read(inputs.0.join("existing/reviewed.ris")).unwrap(),
        b"prior export"
    );
    assert!(!inputs.0.join("existing/mapping.tsv").exists());

    inputs.write(
        "src.ris",
        format!("{ris}TY  - JOUR\nTI  - Unterminated").as_bytes(),
    );
    let failed = inputs.export("out", &sources);
    assert!(String::from_utf8_lossy(&failed.stderr).contains("Invalid RIS at s:3"));
    assert_failed_before_report(failed);
    assert!(!inputs.0.join("out").exists());
    assert_eq!(
        fs::read(inputs.0.join("choices.tsv")).unwrap(),
        sheet.as_bytes()
    );
}

#[test]
fn reviewed_export_checks_all_members_of_each_proposed_set() {
    for (kind, shared, first, second) in [
        (
            "DOI",
            "UR  - https://pubmed.ncbi.nlm.nih.gov/111/",
            "DO  - 10.0000/a",
            "DO  - 10.0000/b\nDO  - missing",
        ),
        (
            "PMID",
            "DO  - 10.0000/shared",
            "UR  - https://pubmed.ncbi.nlm.nih.gov/111/",
            "UR  - https://pubmed.ncbi.nlm.nih.gov/222/",
        ),
    ] {
        let inputs = Inputs::new(kind);
        // The keep target lacks the secondary ID. Only its two proposed copies disagree.
        let ris = format!(
            "TY  - JOUR\n{shared}\nER  -\nTY  - JOUR\n{shared}\n{first}\nER  -\nTY  - JOUR\n{shared}\n{second}\nER  -\n"
        );
        inputs.write("src.ris", ris.as_bytes());
        let sources = ["--ris-source", "s", "src.ris"];
        assert!(
            inputs
                .run(&[&["--init-decisions", "choices.tsv"][..], &sources].concat())
                .status
                .success()
        );
        let inspect = |left, right| {
            let args = ["--inspect-pair", "choices.tsv", left, right];
            let view = inputs.run(&[&args[..], &sources].concat());
            assert!(view.status.success());
            String::from_utf8(view.stdout).unwrap()
        };
        let view = inspect("s:2", "s:3");
        let summary = view.split_once("\n[").unwrap().0;
        assert!(summary.contains(&format!(
            "Conflicting supported {kind} values prevent combining"
        )));
        if kind == "DOI" {
            assert!(
                summary
                    .contains(r#"DOI  "10.0000/a" | "10.0000/b" (additional unsupported field)"#)
            );
        }
        assert!(!inspect("s:1", "s:3").contains("prevent combining"));
        let sheet = fs::read_to_string(inputs.0.join("choices.tsv"))
            .unwrap()
            .replace("s:1\tpending", "s:1\tkeep")
            .replace("\tpending", "\tduplicate-of=s:1");
        inputs.write("choices.tsv", sheet.as_bytes());
        let failed = inputs.export("out", &sources);
        let message = String::from_utf8_lossy(&failed.stderr);
        assert!(
            message.contains(kind) && message.contains("s:1") && message.contains("s:3"),
            "{message}"
        );
        assert_failed_before_report(failed);
        assert!(!inputs.0.join("out").exists());
        assert!(
            inputs
                .run(&[&["--resume-decisions", "choices.tsv"][..], &sources].concat())
                .status
                .success()
        );
        inputs.write(
            "choices.tsv",
            sheet
                .replace("s:3\tduplicate-of=s:1", "s:3\tpending")
                .as_bytes(),
        );
        assert!(inputs.export("out", &sources).status.success());
    }
}

#[test]
fn late_file_and_argument_errors_emit_no_partial_queue() {
    let inputs = Inputs::new("failure");
    inputs.write("good.ris", b"TY  - BOOK\nER  -\n");
    inputs.write("invalid.ris", b"\xff");
    for (label, path) in [
        ("last", "invalid.ris"),
        ("last", "absent.ris"),
        ("last", "."),
        ("last", "-"),
        ("first", "good.ris"),
        ("ambiguous:label", "good.ris"),
    ] {
        assert_failed_before_report(inputs.run(&[
            "--ris-source",
            "first",
            "good.ris",
            "--ris-source",
            label,
            path,
        ]));
    }
    assert_failed_before_report(inputs.run(&["--ris-source", "first", "good.ris", "--ris-source"]));
}

#[test]
fn file_count_limit_accepts_the_boundary_and_rejects_the_next_source() {
    let inputs = Inputs::new("source-limit");
    let labels: Vec<_> = (0..129).map(|n| format!("source-{n}")).collect();
    let mut args = Vec::new();
    for label in &labels {
        inputs.write(label, b"");
        args.extend(["--ris-source", label.as_str(), label.as_str()]);
    }
    let boundary = inputs.run(&args[..128 * 3]);
    assert!(boundary.status.success(), "{:?}", boundary.stderr);
    let report = String::from_utf8(boundary.stdout).unwrap();
    assert_eq!(
        report
            .lines()
            .filter(|line| line.starts_with("input\tris\t"))
            .count(),
        128
    );
    assert_failed_before_report(inputs.run(&args));
}

#[test]
fn metadata_hints_cannot_authorize_a_duplicate_decision() {
    let inputs = Inputs::new("triage");
    let mut source = String::new();
    for id in [
        "",
        "DO  - 10.0000/a\n",
        "DO  - 10.0000/a\n",
        "DO  - 10.0000/b\n",
        "DO  - 10.0000/b\n",
    ] {
        source.push_str(&format!(
            "TY  - JOUR\nTI  - Offshore survey\nAU  - Fable, X.\nPY  - 2020\n{id}ER  -\n"
        ));
    }
    inputs.write("src.ris", source.as_bytes());
    let sources = ["--ris-source", "s", "src.ris"];
    let init = inputs.run(&[&["--init-decisions", "choices.tsv"][..], &sources].concat());
    assert!(init.status.success());
    let report = String::from_utf8(init.stdout).unwrap();
    assert!(
        report.lines().any(|line| line
            == "triage\ttitle_author_year_type\ts:1,s:2,s:3,s:4,s:5\tinspect_metadata_only")
    );
    assert!(!report.contains("Offshore survey"));
    let sheet = fs::read_to_string(inputs.0.join("choices.tsv")).unwrap();
    assert!(!sheet.contains("s:1\t"));
    assert!(inputs.export("all", &sources).status.success());
    assert_eq!(
        fs::read(inputs.0.join("all/reviewed.ris")).unwrap(),
        source.as_bytes()
    );
    for invalid in [
        sheet.replace("s:2\tpending", "s:1\tduplicate-of=s:2\ns:2\tkeep"),
        sheet
            .replace("s:2\tpending", "s:2\tduplicate-of=s:4")
            .replace("s:4\tpending", "s:4\tkeep"),
    ] {
        inputs.write("choices.tsv", invalid.as_bytes());
        assert_failed_before_report(inputs.export("bad", &sources));
        assert!(!inputs.0.join("bad").exists());
    }
}

#[test]
fn pending_view_shows_direct_context_in_conflict_order_without_changing_decisions() {
    let inputs = Inputs::new("pending");
    let simple = "TY  - JOUR\nDO  - 10.0000/early\nER  -\n";
    let marsh = "TY  - JOUR\nTI  - Marsh results\nDO  - 10.0000/marsh\nUR  - https://pubmed.ncbi.nlm.nih.gov/111/\nER  -\n";
    // A protocol shares the results DOI and its own PMID connects it to another
    // occurrence. Its two direct groups have different kept members.
    let protocol = marsh.replace("results", "protocol").replace("111/", "222/");
    let pmid_only = protocol.replace("DO  - 10.0000/marsh\n", "");
    let meta = "TY  - JOUR\nTI  - Tidal survey\nAU  - Reed, A.\nPY  - 2020\nER  -\n";
    let ambiguous = meta.replace("ER  -", "DO  - 10.0000/east\nDO  - 10.0000/west\nER  -");
    let cat = [simple, marsh, meta].concat();
    let idx = [simple, &protocol, &pmid_only, &ambiguous, meta, marsh].concat();
    inputs.write("cat.ris", cat.as_bytes());
    inputs.write("idx.ris", idx.as_bytes());
    let a = ["--ris-source", "cat", "cat.ris"];
    let b = ["--ris-source", "idx", "idx.ris"];
    let sources = [a, b].concat();
    let run = |flag, sheet| inputs.run(&[&[flag, sheet][..], &sources].concat());
    let init = run("--init-metadata-decisions", "choices.tsv");
    assert!(init.status.success());
    let template = fs::read_to_string(inputs.0.join("choices.tsv")).unwrap();
    let edited = template
        .replace("cat:2\tpending", "cat:2\tkeep")
        .replace("cat:3\tpending", "cat:3\tkeep")
        .replace("idx:3\tpending", "idx:3\tkeep")
        .replace("idx:5\tpending", "idx:5\tkeep")
        .replace("idx:6\tpending", "idx:6\tduplicate-of=cat:2");
    inputs.write("choices.tsv", edited.as_bytes());
    assert!(inputs.export("before", &sources).status.success());
    let view = run("--pending-decisions", "choices.tsv");
    assert!(view.status.success(), "{:?}", view.stderr);
    let text = std::str::from_utf8(&view.stdout).unwrap();
    let rows: Vec<_> = text.lines().collect();
    assert_eq!(rows[0], "type\tsignal\trows\tdetail");
    assert_eq!(rows[1], "summary\tpending\t4\tgroups=4;flagged_groups=2");
    assert_eq!(
        &rows[2..4],
        template
            .lines()
            .filter(|l| l.starts_with("input\t"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        &rows[4..],
        [
            "pending\tdoi\tidx:2\tother_id_conflict;keep=cat:2;assigned=idx:6",
            "pending\ttitle_author_year_type\tidx:4\tinspect_metadata_only;keep=cat:3,idx:5;assigned=;conflicting_ids_in=idx:4",
            "pending\tdoi\tcat:1,idx:1\tother_id_incomplete;keep=;assigned=",
            "pending\tpmid\tidx:2\tother_id_incomplete;keep=idx:3;assigned=",
        ]
    );
    let reversed = inputs.run(&[&["--pending-decisions", "choices.tsv"][..], &b, &a].concat());
    assert!(reversed.status.success());
    assert_eq!(view.stdout, reversed.stdout);
    inputs.write(
        "guide.txt",
        b"choices.tsv\ncat\ncat.ris\nidx\nidx.ris\n\nyes\n:pending metadata\n:inspect cat:1,idx:1\n:inspect idx:2,cat:2\n:cancel\n",
    );
    let guided = Command::new(env!("CARGO_BIN_EXE_dedup"))
        .current_dir(&inputs.0)
        .arg("start")
        .stdin(fs::File::open(inputs.0.join("guide.txt")).unwrap())
        .output()
        .unwrap();
    assert!(guided.status.success(), "{:?}", guided.stderr);
    let prompts = String::from_utf8_lossy(&guided.stderr);
    assert!(prompts.contains(&format!(
        "First unresolved group from the pending list:\n  {}\nTo compare two keys",
        rows[4]
    )));
    let mut expected = view.stdout.clone();
    let metadata_view = [
        rows[0],
        "summary\tpending_in_sheet\t4\tshown_metadata_groups=1;flagged_metadata_groups=1",
        rows[2],
        rows[3],
        rows[5],
    ];
    expected.extend_from_slice(metadata_view.join("\n").as_bytes());
    expected.push(b'\n');
    for keys in [["cat:1", "idx:1"], ["idx:2", "cat:2"]] {
        let pair = inputs.run(&[&["--inspect-pair", "choices.tsv"][..], &keys, &sources].concat());
        assert!(pair.status.success());
        expected.extend(pair.stdout);
    }
    assert_eq!(guided.stdout, expected);
    assert!(inputs.export("after", &sources).status.success());
    for file in ["mapping.tsv", "reconciliation.tsv", "reviewed.ris"] {
        assert_eq!(
            fs::read(inputs.0.join("before").join(file)).unwrap(),
            fs::read(inputs.0.join("after").join(file)).unwrap()
        );
    }
    assert_eq!(
        fs::read(inputs.0.join("choices.tsv")).unwrap(),
        edited.as_bytes()
    );
    assert_eq!(fs::read(inputs.0.join("cat.ris")).unwrap(), cat.as_bytes());
    assert_eq!(fs::read(inputs.0.join("idx.ris")).unwrap(), idx.as_bytes());

    // V1 displays ID groups only. A fully resolved sheet still has its manifest.
    assert!(run("--init-decisions", "ids.tsv").status.success());
    let legacy = run("--pending-decisions", "ids.tsv");
    assert!(legacy.status.success());
    assert!(
        !String::from_utf8(legacy.stdout)
            .unwrap()
            .contains("title_author_year_type")
    );
    inputs.write(
        "choices.tsv",
        edited.replace("\tpending", "\tkeep").as_bytes(),
    );
    let done = run("--pending-decisions", "choices.tsv");
    assert!(done.status.success());
    let done = String::from_utf8(done.stdout).unwrap();
    assert!(done.contains("summary\tpending\t0\tgroups=0;flagged_groups=0\n"));
    assert_eq!(done.lines().count(), 4);

    inputs.write("report.tsv", &view.stdout);
    assert_failed_before_report(run("--pending-decisions", "report.tsv"));
    inputs.write(
        "choices.tsv",
        edited
            .replace("duplicate-of=cat:2", "duplicate-of=idx:1")
            .as_bytes(),
    );
    assert_failed_before_report(run("--pending-decisions", "choices.tsv"));
    inputs.write("choices.tsv", edited.as_bytes());
    inputs.write("idx.ris", idx.replace("Marsh", "Marls").as_bytes());
    let stale = run("--pending-decisions", "choices.tsv");
    assert!(String::from_utf8_lossy(&stale.stderr).contains("inputs differ"));
    assert_failed_before_report(stale);
}

#[test]
fn coverage_checks_one_snapshot_against_independent_counts_and_ranges() {
    let inputs = Inputs::new("coverage");
    let record = |title| format!("TY  - JOUR\nTI  - {title}\nER  -\n");
    let files = [
        ("first.ris", record("Estuary") + &record("Inlet")),
        ("second.ris", record("Inlet") + &record("Delta")),
        ("short.ris", record("Inlet")),
        ("update.ris", record("Soundings")),
    ];
    for (name, text) in &files {
        inputs.write(name, text.as_bytes());
    }

    let total = ["--expected-total", "4"];
    let ranges = [
        "--batch-range",
        "first",
        "1-2",
        "--batch-range",
        "second",
        "3-4",
    ];
    let first = ["--ris-source", "first", "first.ris"];
    let both = [&first[..], &["--ris-source", "second", "second.ris"]].concat();
    let check = |snapshot: &str, args: &[&str], expected: &str, issues: &[&str]| {
        let output = inputs.run(&[&["--check-coverage", snapshot][..], args].concat());
        let report = String::from_utf8(output.stdout).unwrap();
        let matched = issues.is_empty();
        assert_eq!(
            output.status.code(),
            Some(if matched { 0 } else { 2 }),
            "{report}"
        );
        assert_eq!(
            report.lines().next(),
            Some(format!("snapshot\t{snapshot}\t{expected}").as_str())
        );
        let actual: Vec<_> = report
            .lines()
            .filter_map(|line| line.strip_prefix("issue\t"))
            .collect();
        assert_eq!(actual, issues, "{report}");
        let result = if matched {
            "matched_expectations"
        } else {
            "discrepancies"
        };
        assert!(report.ends_with(&format!("result\t{snapshot}\t{result}\n")));
        println!("{report}");
    };
    check(
        "coast-day1",
        &[&total[..], &ranges, &both].concat(),
        "parsed=4\texpected_total=4",
        &[],
    );
    check(
        "coast-update",
        &[
            "--expected-total",
            "1",
            "--ris-source",
            "first",
            "update.ris",
        ],
        "parsed=1\texpected_total=1",
        &[],
    );
    check(
        "coast-day1",
        &[&total[..], &ranges, &first].concat(),
        "parsed=2\texpected_total=4",
        &[
            "total_count_mismatch\texpected=4\tparsed=2",
            "missing_batch\tsecond\tpositions=3-4",
        ],
    );
    check(
        "coast-day1",
        &[
            &total[..],
            &ranges,
            &first,
            &["--ris-source", "second", "short.ris"],
        ]
        .concat(),
        "parsed=3\texpected_total=4",
        &[
            "total_count_mismatch\texpected=4\tparsed=3",
            "batch_count_mismatch\tsecond\texpected=2\tparsed=1",
        ],
    );
    check(
        "coast-day1",
        &[
            &total[..],
            &[
                "--batch-range",
                "first",
                "1-2",
                "--batch-range",
                "second",
                "2-3",
            ],
            &both,
        ]
        .concat(),
        "parsed=4\texpected_total=4",
        &[
            "range_overlap\tsecond\tpositions=2-2",
            "range_gap\tpositions=4-4",
        ],
    );

    // With ranges only, validate the declared window. No inference about its ends.
    check(
        "window",
        &[&["--batch-range", "first", "11-12"][..], &first].concat(),
        "parsed=2\texpected_total=unspecified",
        &[],
    );
    check(
        "absent",
        &["--batch-range", "first", "11-12"],
        "parsed=0\texpected_total=unspecified",
        &["missing_batch\tfirst\tpositions=11-12"],
    );
    // Multiple snapshots or an invalid plan must never yield a pooled report.
    for invalid in [
        &[
            "--expected-total",
            "1",
            "--check-coverage",
            "other",
            "--expected-total",
            "1",
        ][..],
        &["--batch-range", "first", "0-1"],
        &["--batch-range", "first", "3-2"],
    ] {
        assert_failed_before_report(
            inputs.run(&[&["--check-coverage", "one"][..], invalid, &first].concat()),
        );
    }
    assert_eq!(fs::read_dir(&inputs.0).unwrap().count(), files.len());
    for (name, text) in files {
        assert_eq!(fs::read(inputs.0.join(name)).unwrap(), text.as_bytes());
    }
}

#[test]
fn coverage_flags_repeated_file_bytes_within_a_snapshot() {
    let inputs = Inputs::new("repeated-batch");
    let record = "TY  - JOUR\nTI  - Harbor survey\nER  -\n";
    inputs.write("batch.ris", record.as_bytes());
    inputs.write("renamed.ris", record.as_bytes());
    let crlf = record.replace('\n', "\r\n");
    inputs.write("crlf.ris", crlf.as_bytes());

    // Reverse the arguments so the flag must still identify the labels in canonical order.
    let check = |path: &str, total: &str| {
        inputs.run(&[
            "--check-coverage",
            "harbor",
            "--expected-total",
            total,
            "--batch-range",
            "a",
            "1-1",
            "--batch-range",
            "b",
            "2-2",
            "--ris-source",
            "b",
            path,
            "--ris-source",
            "a",
            "batch.ris",
        ])
    };
    for path in ["batch.ris", "renamed.ris"] {
        let output = check(path, "2");
        assert_eq!(output.status.code(), Some(2));
        let report = String::from_utf8(output.stdout).unwrap();
        assert!(report.starts_with("snapshot\tharbor\tparsed=2\texpected_total=2\n"));
        assert!(!report.lines().any(|row| row.starts_with("issue\t")));
        assert!(report.ends_with(concat!(
            "counts_and_ranges\tharbor\tmatched_expectations\n",
            "source_flag\trepeated_source_bytes\ta,b\n",
            "result\tharbor\tdiscrepancies\n"
        )));
    }
    let combined = check("renamed.ris", "3");
    assert_eq!(combined.status.code(), Some(2));
    let report = String::from_utf8(combined.stdout).unwrap();
    assert!(report.contains("issue\ttotal_count_mismatch\texpected=3\tparsed=2\n"));
    assert!(report.contains(
        "counts_and_ranges\tharbor\tdiscrepancies\nsource_flag\trepeated_source_bytes\ta,b\n"
    ));

    // Equivalent fields with different bytes and identical bytes in a separate run.
    let update = inputs.run(&[
        "--check-coverage",
        "harbor-update",
        "--expected-total",
        "1",
        "--ris-source",
        "a",
        "renamed.ris",
    ]);
    for (snapshot, output) in [
        ("harbor", check("crlf.ris", "2")),
        ("harbor-update", update),
    ] {
        assert!(output.status.success());
        let report = String::from_utf8(output.stdout).unwrap();
        assert!(!report.contains("source_flag\t"));
        assert!(report.ends_with(&format!("result\t{snapshot}\tmatched_expectations\n")));
    }
    assert_eq!(fs::read_dir(&inputs.0).unwrap().count(), 3);
    for (path, data) in [
        ("batch.ris", record),
        ("renamed.ris", record),
        ("crlf.ris", crlf.as_str()),
    ] {
        assert_eq!(fs::read(inputs.0.join(path)).unwrap(), data.as_bytes());
    }
}

#[test]
fn verify_export_detects_a_swapped_equal_count_and_length_ris() {
    use sha2::{Digest, Sha256};
    let inputs = Inputs::new("verify-export");
    let first = "TY  - JOUR\nTI  - Tidal survey results\nAU  - Reed, A.\nPY  - 2020\nJO  - Model Journal\nVL  - 1\nSP  - 1-9\nAN  - SYN-A\nER  -\n";
    let second = first.replace("SYN-A", "SYN-B");
    let protocol = "TY  - JOUR\nTI  - Tidal survey protocol\nER  -\n";
    inputs.write(
        "records.ris",
        format!("{first}{second}{protocol}").as_bytes(),
    );
    let sources = ["--ris-source", "batch", "records.ris"];
    assert!(
        inputs
            .run(&[&["--init-metadata-decisions", "choices.tsv"][..], &sources].concat())
            .status
            .success()
    );
    let pending = fs::read_to_string(inputs.0.join("choices.tsv")).unwrap();
    assert!(inputs.export("pending", &sources).status.success());
    let verify = |mapping: &str, ris: &str| {
        Command::new(env!("CARGO_BIN_EXE_dedup"))
            .current_dir(&inputs.0)
            .args(["verify-export", mapping, ris])
            .output()
            .unwrap()
    };
    let read = |path: &str| fs::read(inputs.0.join(path)).unwrap();
    // The all-pending export uses the original v1 mapping layout.
    assert!(
        read("pending/mapping.tsv").starts_with(b"type\tsignal\trows\tdetail\nschema\texport\t1\t")
    );
    assert!(
        verify("pending/mapping.tsv", "pending/reviewed.ris")
            .status
            .success()
    );
    for (dir, keep, drop, representative) in
        [("run A", 1, 2, first), ("run B", 2, 1, second.as_str())]
    {
        let choices = pending
            .replace(
                &format!("batch:{keep}\tpending"),
                &format!("batch:{keep}\tkeep"),
            )
            .replace(
                &format!("batch:{drop}\tpending"),
                &format!(
                    "batch:{drop}\tverified-duplicate-of=batch:{keep}\tCompared full citation."
                ),
            );
        inputs.write("choices.tsv", choices.as_bytes());
        assert!(inputs.export(dir, &sources).status.success());
        let mapping = format!("{dir}/mapping.tsv");
        let ris = format!("{dir}/reviewed.ris");
        let map_bytes = read(&mapping);
        let ris_bytes = read(&ris);
        assert_eq!(ris_bytes, format!("{representative}{protocol}").as_bytes());
        let verified = verify(&mapping, &ris);
        assert!(verified.status.success(), "{:?}", verified.stderr);
        let row = format!(
            "matched_output_fingerprint\t{mapping:?}\t{:x}\t{ris:?}\t{}\t{:x}",
            Sha256::digest(&map_bytes),
            ris_bytes.len(),
            Sha256::digest(&ris_bytes)
        );
        let report = String::from_utf8(verified.stdout).unwrap();
        assert_eq!(report.lines().count(), 2);
        assert_eq!(report.lines().nth(1), Some(row.as_str()));
        assert_eq!(read(&mapping), map_bytes);
        assert_eq!(read(&ris), ris_bytes);
    }
    assert_eq!(
        read("run A/reconciliation.tsv"),
        read("run B/reconciliation.tsv")
    );
    let swapped = verify("run A/mapping.tsv", "run B/reviewed.ris");
    assert_eq!(swapped.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&swapped.stderr).contains("SHA-256 differs"));
    assert_failed_before_report(swapped);

    let map = fs::read_to_string(inputs.0.join("run A/mapping.tsv")).unwrap();
    let (without_output, fingerprint) = map.rsplit_once("output\t").unwrap();
    for invalid in [
        without_output.to_owned(),
        format!("{map}output\t{fingerprint}"),
        format!(
            "{without_output}output\t{}",
            fingerprint.replace(";sha256=", ";missing=")
        ),
    ] {
        inputs.write("incomplete.tsv", invalid.as_bytes());
        let result = verify("incomplete.tsv", "run A/reviewed.ris");
        assert_eq!(result.status.code(), Some(2));
        assert_failed_before_report(result);
    }
}

#[test]
fn optional_keep_reason_follows_the_selected_kept_report() {
    let inputs = Inputs::new("keep-reason");
    let results = "TY  - JOUR\nTI  - Ceramic flexure: results\nDO  - 10.0000/flex\nER  -\n";
    let protocol = results.replace("results", "protocol");
    let ris = format!("{results}{results}{protocol}{results}");
    inputs.write("originals/records.ris", ris.as_bytes());
    let sources = ["--ris-source", "s", "originals/records.ris"];
    let run = |flag| inputs.run(&[&[flag, "choices.tsv"][..], &sources].concat());
    assert!(run("--init-metadata-decisions").status.success());
    let sheet = inputs.0.join("choices.tsv");
    let plain = fs::read_to_string(&sheet)
        .unwrap()
        .replace("s:1\tpending", "s:1\tkeep")
        .replace("s:2\tpending", "s:2\tduplicate-of=s:1")
        .replace("s:3\tpending", "s:3\tkeep");
    inputs.write("choices.tsv", plain.as_bytes());
    assert!(inputs.export("plain", &sources).status.success());

    let reason = "Distinct protocol — planned measurements only.";
    let annotated = plain
        .replace("s:1\tkeep", "s:1\tkeep-with-reason\tComplete results.")
        .replace("s:3\tkeep", &format!("s:3\tkeep-with-reason\t{reason}"));
    inputs.write("choices.tsv", annotated.as_bytes());
    for flag in ["--resume-decisions", "--pending-decisions"] {
        let result = run(flag);
        assert!(result.status.success());
        assert!(!String::from_utf8_lossy(&result.stdout).contains(reason));
    }
    assert!(inputs.export("noted", &sources).status.success());
    assert_eq!(fs::read_to_string(&sheet).unwrap(), annotated);
    let read = |path| fs::read_to_string(inputs.0.join(path)).unwrap();
    assert_eq!(
        read("noted/reviewed.ris"),
        format!("{results}{protocol}{results}")
    );
    assert_eq!(read("plain/reviewed.ris"), read("noted/reviewed.ris"));
    assert_eq!(
        read("plain/reconciliation.tsv"),
        read("noted/reconciliation.tsv")
    );
    let mapping = read("noted/mapping.tsv");
    let audit = format!("rationale\tkeep\ts:3\t{reason}\n");
    assert_eq!(mapping.matches(&audit).count(), 1);
    assert!(mapping.contains("map\tpending\ts:4\t"));
    assert_eq!(read("originals/records.ris"), ris);

    let trace = |show, mapping: &str, key: &str| {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_dedup"));
        cmd.current_dir(&inputs.0).arg("trace");
        if show {
            cmd.arg("--show-keep-reason");
        }
        cmd.args([mapping, key, "originals"]).output().unwrap()
    };
    inputs.write("choices.tsv", plain.as_bytes()); // Later edits do not change a completed export.
    let regular = trace(false, "noted/mapping.tsv", "s:3");
    assert!(regular.status.success());
    assert_eq!(
        regular.stdout,
        trace(false, "plain/mapping.tsv", "s:3").stdout
    );
    let selected = trace(true, "noted/mapping.tsv", "s:3");
    assert!(selected.status.success());
    let report = String::from_utf8(selected.stdout).unwrap();
    assert!(report.lines().next().unwrap().ends_with("\tkeep_reason"));
    assert!(report.ends_with(&format!("\t{reason}\n")));
    assert!(!report.contains("Complete results."));
    let absent = trace(true, "plain/mapping.tsv", "s:3");
    assert!(absent.status.success() && absent.stdout.ends_with(b"\t\n"));
    assert_failed_before_report(trace(true, "noted/mapping.tsv", "s:2"));
    assert_failed_before_report(trace(true, "noted/mapping.tsv", "s:4"));

    for bad in [
        mapping.replace(&audit, &audit.repeat(2)),
        mapping.replace(&audit, "rationale\tmetadata\ts:3\twrong type\n"),
        mapping.replace(&audit, "rationale\tkeep\ts:3\textra\tcolumn\n"),
    ] {
        inputs.write("bad.tsv", bad.as_bytes());
        assert_failed_before_report(trace(true, "bad.tsv", "s:3"));
    }
    for (key, bad) in [
        ("s:3\tkeep", "s:3\tkeep\tstray"),
        ("s:3\tkeep", "s:3\tkeep-with-reason\t "),
        ("s:3\tkeep", "s:3\tkeep-with-reason\tfirst\tsecond"),
        ("s:3\tkeep", "s:3\tkeep-with-reason\tline\u{2028}break"),
        (
            "s:3\tkeep",
            &format!("s:3\tkeep-with-reason\t{}", "é".repeat(257)),
        ),
        ("s:4\tpending", "s:4\tpending\treason"),
        ("s:2\tduplicate-of=s:1", "s:2\tduplicate-of=s:1\treason"),
    ] {
        inputs.write("choices.tsv", plain.replace(key, bad).as_bytes());
        assert_failed_before_report(inputs.export("invalid", &sources));
        assert!(!inputs.0.join("invalid").exists());
    }
    inputs.write(
        "originals/records.ris",
        ris.replace("results", "changed").as_bytes(),
    );
    assert_failed_before_report(trace(true, "noted/mapping.tsv", "s:3"));
}

#[test]
fn guided_preview_names_removal_with_opposing_work_type_wording() {
    let inputs = Inputs::new("work-type-wording");
    let ris = ["RESULTS article", "Study protocol", "Journal article"]
        .map(|kind| {
            format!(
                "TY  - JOUR\nTI  - Lattice survey\nDO  - 10.0000/lattice\nM3  - {kind}\nER  -\n"
            )
        })
        .concat();
    inputs.write("records.ris", ris.as_bytes());
    let init = inputs.run(&[
        "--init-decisions",
        "choices.tsv",
        "--ris-source",
        "s",
        "records.ris",
    ]);
    assert!(init.status.success(), "{:?}", init.stderr);
    let sheet = fs::read_to_string(inputs.0.join("choices.tsv"))
        .unwrap()
        .replace("s:1\tpending", "s:1\tkeep")
        .replace("s:2\tpending", "s:2\tduplicate-of=s:1")
        .replace("s:3\tpending", "s:3\tduplicate-of=s:1");
    inputs.write("choices.tsv", sheet.as_bytes());
    inputs.write(
        "answers.txt",
        b"choices.tsv\ns\nrecords.ris\n\nyes\nreviewed\n\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_dedup"))
        .arg("start")
        .current_dir(&inputs.0)
        .stdin(fs::File::open(inputs.0.join("answers.txt")).unwrap())
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    let prompts = String::from_utf8(output.stderr).unwrap();
    let (preview, _) = prompts.split_once("Type write").unwrap();
    assert!(
        preview
            .contains("s:2 (remove) has 'protocol' wording and s:1 (keep) has 'results' wording")
    );
    assert!(!preview.contains("s:3 (remove)"));
    assert!(!inputs.0.join("reviewed").exists());
}

#[test]
fn guided_review_flags_whole_file_matches_and_keeps_each_occurrence() {
    let inputs = Inputs::new("repeated-download");
    let first = "TY  - JOUR\nTI  - Inlet survey\nAN  - A1\nER  -\n";
    let second = "TY  - JOUR\nTI  - Inlet survey\nAN  - A2\nER  -\n";
    let batch = format!("{first}{second}");
    let other = format!("{first}{}", second.replace("A2", "B2"));
    inputs.write("first.ris", batch.as_bytes());
    inputs.write("copy.ris", batch.as_bytes());
    inputs.write("index.ris", other.as_bytes());
    inputs.write(
        "answers.txt",
        b"choices.tsv\nfirst\nfirst.ris\ncopy\ncopy.ris\nindex\nindex.ris\n\nreview\nreviewed\nwrite\n",
    );
    let guide = Command::new(env!("CARGO_BIN_EXE_dedup"))
        .arg("start")
        .current_dir(&inputs.0)
        .stdin(fs::File::open(inputs.0.join("answers.txt")).unwrap())
        .output()
        .unwrap();
    assert!(guide.status.success(), "{:?}", guide.stderr);
    let stderr = String::from_utf8(guide.stderr).unwrap();
    let (before_write, _) = stderr.split_once("Type write").unwrap();
    assert!(before_write.contains("Source labels copy, first have the same whole-file SHA-256."));
    assert_eq!(stderr.matches("same whole-file SHA-256").count(), 1);
    assert_eq!(
        fs::read_to_string(inputs.0.join("reviewed/reviewed.ris")).unwrap(),
        format!("{batch}{batch}{other}")
    );
}

#[test]
fn guided_setup_can_edit_a_path_and_add_a_missing_source() {
    let inputs = Inputs::new("edit-add");
    inputs.write(
        "east.ris",
        b"TY  - JOUR\nTI  - Reed beds\nDO  - 10.0000/reeds\nER  -\n",
    );
    inputs.write(
        "west.ris",
        b"TY  - JOUR\nT1  - Reed beds\nDO  - 10.0000/reeds\nER  -\n",
    );
    let init = inputs.run(&[
        "--init-decisions",
        "choices.tsv",
        "--ris-source",
        "east",
        "east.ris",
        "--ris-source",
        "west",
        "west.ris",
    ]);
    assert!(init.status.success(), "{:?}", init.stderr);

    inputs.write(
        "answers.txt",
        b"choices.tsv\neast\nwest.ris\n\nedit east\neast.ris\nadd west\nwest.ris\nyes\n\n",
    );
    let guide = Command::new(env!("CARGO_BIN_EXE_dedup"))
        .current_dir(&inputs.0)
        .arg("start")
        .stdin(fs::File::open(inputs.0.join("answers.txt")).unwrap())
        .output()
        .unwrap();
    assert!(guide.status.success(), "{:?}", guide.stderr);
    assert!(String::from_utf8_lossy(&guide.stderr).contains("Missing saved source 'west'"));
    assert!(String::from_utf8_lossy(&guide.stdout).contains("pending\tdoi\teast:1,west:1\t"));
}

#[test]
fn guided_find_shows_named_authors_in_either_order_and_filters_by_title() {
    let inputs = Inputs::new("find-author");
    inputs.write("folio.ris", b"TY  - JOUR\nTI  - Paper lattices\nAU  - Pell, Ora\nAU  - de Orrel, Nessa\nER  -\n\
        TY  - JOUR\nT1  - Paper lattices\nA1  - Nessa de Orrel\nER  -\n\
        TY  - JOUR\nTI  - Paper lattices\nAU  - de Orrel, N.\nED  - de Orrel, Nessa\nER  -\n\
        TY  - JOUR\nTI  - Copper braids\nAU  - de Orrel, Nessa\nER  -\n\
        TY  - JOUR\nTI  - Paper lattices\nAU  - Nessa Pell\nAU  - Ivo de Orrel\nA2  - de Orrel, Nessa\nER  -\n");
    inputs.write(
        "answers.txt",
        b"choices.tsv\nfolio\nfolio.ris\n\nreview\n\
        :find --author Nessa de Orrel | paper lattices\n\
        :find de Orrel, Nessa\n\
        :find --author de Orrel | paper lattices\n\
        :cancel\n",
    );
    let guide = Command::new(env!("CARGO_BIN_EXE_dedup"))
        .current_dir(&inputs.0)
        .arg("start")
        .stdin(fs::File::open(inputs.0.join("answers.txt")).unwrap())
        .output()
        .unwrap();
    assert!(guide.status.success(), "{:?}", guide.stderr);
    let text = String::from_utf8(guide.stdout).unwrap();
    let rows: Vec<_> = text.lines().filter(|l| l.starts_with("folio:")).collect();
    let keys: Vec<_> = rows.iter().map(|r| r.split('\t').next().unwrap()).collect();
    assert_eq!(
        keys,
        [
            "folio:1", "folio:2", "folio:1", "folio:2", "folio:4", "folio:1", "folio:2", "folio:3",
            "folio:5"
        ]
    );
    assert!(rows[0].ends_with("AU=\"de Orrel, Nessa\""));
    assert!(rows[1].ends_with("A1=\"Nessa de Orrel\""));
    assert!(rows[7].ends_with("AU=\"de Orrel, N.\""));
    assert!(rows[8].ends_with("AU=\"Ivo de Orrel\""));
}

#[test]
fn guided_find_requires_venue_and_title_on_the_same_occurrence() {
    let inputs = Inputs::new("find-venue");
    inputs.write("folio.ris", b"TY  - JOUR\nTI  - Field notes\nJO  - Slate Inlet Bulletin\nER  -\n\
        TY  - JOUR\nT1  - Field notes\nJF  - Slate\n      Inlet Bulletin\nER  -\n\
        TY  - JOUR\nTI  - Field notes\nT2  - Slate Inlet Bulletin\nAB  - Separate follow-up.\nER  -\n\
        TY  - JOUR\nTI  - Field notes\nAU  - Slate Inlet Bulletin\nER  -\n\
        TY  - JOUR\nTI  - Field notes\nJO  - Inlet, Slate\nER  -\n\
        TY  - JOUR\nTI  - Census\nJO  - Slate Inlet Bulletin\nER  -\n");
    inputs.write(
        "answers.txt",
        b"choices.tsv\nfolio\nfolio.ris\n\nreview\n\
        :find --venue | field notes\n\
        :find --venue SLATE inlet | field notes\n\
        :cancel\n",
    );
    let guide = Command::new(env!("CARGO_BIN_EXE_dedup"))
        .current_dir(&inputs.0)
        .arg("start")
        .stdin(fs::File::open(inputs.0.join("answers.txt")).unwrap())
        .output()
        .unwrap();
    assert!(guide.status.success(), "{:?}", guide.stderr);
    let prompts = String::from_utf8(guide.stderr).unwrap();
    assert!(prompts.contains("Enter :find --venue VENUE | TITLE with text on both sides"));
    let output = String::from_utf8(guide.stdout).unwrap();
    let hits: Vec<_> = output
        .lines()
        .filter(|line| line.starts_with("folio:"))
        .map(|line| {
            let columns: Vec<_> = line.split('\t').collect();
            (columns[0], columns[3])
        })
        .collect();
    assert_eq!(
        hits,
        [
            ("folio:1", "JO=\"Slate Inlet Bulletin\""),
            ("folio:2", "JF=\"Slate Inlet Bulletin\""),
            ("folio:3", "T2=\"Slate Inlet Bulletin\"")
        ]
    );
}
