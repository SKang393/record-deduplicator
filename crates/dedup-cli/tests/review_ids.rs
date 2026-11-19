use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

fn run(input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_dedup"))
        .args(["review-ids", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn canonical_output_retains_every_occurrence_and_has_no_raw_identifiers() {
    let input = b"doi\tpmid\n10.0000/b\t91\n10.0000/a\t\n10.0000/b\t92\n10.0000/a\t\nwrong\n";
    let expected=b"type\tsignal\trows\tdetail\nsummary\tall\t5\tretained=5;suppressed=0;candidate_records=4;groups=2\ncandidate\tdoi\t1,3\tother_id_conflict\ncandidate\tdoi\t2,4\tother_id_incomplete\nissue\tinput\t5\twrong_column_count\n";
    let first = run(input);
    assert!(first.status.success());
    assert!(first.stderr.is_empty());
    assert_eq!(first.stdout, expected);
    assert_eq!(run(input).stdout, expected);
    let crlf = std::str::from_utf8(input).unwrap().replace('\n', "\r\n");
    assert_eq!(run(crlf.as_bytes()).stdout, expected);
}

#[test]
fn invalid_header_and_non_utf8_do_not_emit_partial_reports() {
    for input in [b"pmid\tdoi\n".as_slice(), b"doi\tpmid\n\xff\t9\n"] {
        let out = run(input);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
}
