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
    let other = b"TY  - BOOK\nER  -\nTY  - JOUR\nDO  - 10.0000/single\nER  -\noutside record";
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
        lines.contains(&"summary\tall\t5\tretained=5;suppressed=0;candidate_records=2;groups=1")
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
    assert_eq!(dispositions, ["review_input", "retained", "review_input"]);
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
fn decisions_reopen_against_named_bytes_and_can_be_reversed() {
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
    assert!(String::from_utf8_lossy(&stale.stderr).contains("inputs differ"));
    assert_failed_before_report(stale);
    assert_eq!(
        fs::read(inputs.0.join("review.tsv")).unwrap(),
        reversed.as_bytes()
    );
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
    fs::remove_file(inputs.0.join("choices.tsv")).unwrap();
    assert!(init().status.success());
    let failed = inputs.export("out", &sources);
    assert!(String::from_utf8_lossy(&failed.stderr).contains("damaged occurrence s:3"));
    assert_failed_before_report(failed);
    assert!(!inputs.0.join("out").exists());
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
