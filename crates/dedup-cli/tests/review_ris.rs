use std::{
    io::Write,
    process::{Command, Output, Stdio},
};
fn run(input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_dedup"))
        .args(["review-ids", "--ris", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn native_report_exposes_source_ranges_and_damage_without_leaking_metadata() {
    let input=b"TY  - JOUR\nDO  - 10.0000/example\nQ9  - illustrative text\nER  -\nTY  - JOUR\nDO  - 10.0000/example\nER  -\noutside record\n";
    let a = run(input);
    assert!(a.status.success());
    assert_eq!(run(input).stdout, a.stdout);
    let text = String::from_utf8(a.stdout).unwrap();
    assert!(text.contains("retained=3;suppressed=0;candidate_records=2;groups=1"));
    assert!(text.contains("candidate\tdoi\t1,2\tother_id_incomplete"));
    let ends: Vec<_> = input
        .split_inclusive(|b| *b == b'\n')
        .map(|l| l.len())
        .collect();
    let first: usize = ends[..4].iter().sum();
    let second: usize = ends[..7].iter().sum();
    assert!(text.contains(&format!("source\tris\t1\tbytes=0..{first};status=complete;closed=true;malformed_lines=0;fields=4;unmapped_fields=1")));
    assert!(text.contains(&format!(
        "source\tris\t3\tbytes={second}..{};status=fragment",
        input.len()
    )));
    assert!(!text.contains("illustrative") && !text.contains("10.0000"));
}
#[test]
fn native_input_rejects_non_utf8_without_replacement_or_partial_report() {
    let out = run(b"TY  - JOUR\nTI  - \xff\nER  -\n");
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
}
