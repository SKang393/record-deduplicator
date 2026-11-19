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
