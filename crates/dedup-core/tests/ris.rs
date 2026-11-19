use dedup_core::identifier_review::{
    Identifier::*, InputError, Issue, MAX_RECORDS, OtherEvidence::*,
};
use dedup_core::ris::{MAX_FIELDS, parse};

#[test]
fn all_bytes_and_interleaved_fields_remain_accessible_with_unicode_crlf_and_no_final_newline() {
    let text = "\u{feff}TY  - JOUR\r\nAU  - Example, A\r\nQ9  - caf\u{e9}\r\n  second line\r\nAU  - Example, B\r\nDO  - 10.0000/a\r\nER  -\r\n\r\nTY  - BOOK\r\nER  -";
    let source = parse(text).unwrap();
    assert_eq!(source.original.as_bytes(), text.as_bytes());
    let [a, b] = source.occurrences.as_slice() else {
        panic!("expected two occurrences")
    };
    assert_eq!(
        a.fields.iter().map(|f| f.tag).collect::<Vec<_>>(),
        ["TY", "AU", "Q9", "AU", "DO", "ER"]
    );
    assert_eq!(a.fields[2].first_line_value, "caf\u{e9}");
    assert_eq!(
        &source.original[a.fields[2].bytes.clone()],
        "Q9  - caf\u{e9}\r\n  second line\r\n"
    );
    assert_eq!(a.unmapped_fields(), 3);
    assert!(a.usable() && b.usable());
    assert_eq!(a.bytes.start, 0);
    assert_eq!(b.bytes.end, text.len());
    assert_eq!(&text[a.bytes.end..b.bytes.start], "\r\n");
    let field_bytes: String = a.fields.iter().map(|f| &text[f.bytes.clone()]).collect();
    assert_eq!(field_bytes, &text[a.bytes.clone()]);
}

#[test]
fn conflicting_repeats_abstain_for_that_namespace_and_mark_other_group() {
    let text = "TY  - JOUR\nDO  - 10.0000/a\nDO  - 10.0000/b\nUR  - https://pubmed.ncbi.nlm.nih.gov/12/\nER  -\nTY  - JOUR\nDO  - 10.0000/a\nUR  - https://pubmed.ncbi.nlm.nih.gov/12\nER  -\nTY  - JOUR\nDO  - doi:10.0000/A\nUR  - https://doi.org/10.0000/a\nER  -\n";
    let report = parse(text).unwrap().review();
    assert_eq!(report.retained_records, 3);
    assert_eq!(report.groups.len(), 2);
    assert_eq!(
        (
            report.groups[0].shared,
            &report.groups[0].rows,
            report.groups[0].other_evidence
        ),
        (Doi, &vec![2, 3], Incomplete)
    );
    assert_eq!(
        (
            report.groups[1].shared,
            &report.groups[1].rows,
            report.groups[1].other_evidence
        ),
        (Pmid, &vec![1, 2], Conflict)
    );
    assert_eq!(report.issues, [(1, Issue::ConflictingDoi)]);
}

#[test]
fn do_not_guess_accession_namespace_or_follow_arbitrary_web_links() {
    let text =
        "TY  - JOUR\nAN  - 123\nID  - 456\nUR  - https://example.invalid/123\nER  -\n".repeat(2);
    let source = parse(&text).unwrap();
    let r = source.review();
    assert_eq!(r.retained_records, 2);
    assert!(r.groups.is_empty());
    assert_eq!(
        r.issues,
        [(1, Issue::NoIdentifiers), (2, Issue::NoIdentifiers)]
    );
    assert_eq!(source.occurrences[0].unmapped_fields(), 3);
}

#[test]
fn neither_partial_records_nor_stray_fragments_contaminate_adjacent_candidates() {
    let text = "outside record\nDO  - 10.0000/x\nER  -\nTY  - JOUR\nDO  - 10.0000/x\nTY  - JOUR\nDO  - 10.0000/x\nER  -\nTY  - JOUR\nDO  - 10.0000/x\nER  -\nTY  - JOUR\nDO  - 10.0000/x";
    let s = parse(text).unwrap();
    assert_eq!(s.occurrences.len(), 5);
    assert!(!s.occurrences[0].has_type && s.occurrences[0].has_end);
    assert!(s.occurrences[1].has_type && !s.occurrences[1].has_end);
    assert!(s.occurrences[4].has_type && !s.occurrences[4].has_end);
    let r = s.review();
    assert_eq!(r.retained_records, 5);
    assert_eq!(r.groups[0].rows, [3, 4]);
}

#[test]
fn damaged_and_folded_id_fields_are_not_trusted() {
    let text = "TY  - JOUR\nDO  - 10.0000/x\nBad untagged line\nER  -\nTY  - JOUR\nDO  - 10.0000/x\n continuation\nER  -\nTY  - JOUR\nDO  - 10.0000/x\nDO  - not-valid\nER  -\nTY  - JOUR\nDO  - 10.0000/x\n  TY  - JOUR\nER  -\nTY  - JOUR\nDO  - 10.0000/x\nTI  - bad\0text\nER  -\n";
    let s = parse(text).unwrap();
    let r = s.review();
    assert_eq!(r.retained_records, 5);
    assert!(r.groups.is_empty());
    assert_eq!(
        r.issues,
        [(2, Issue::UnsupportedDoi), (3, Issue::UnsupportedDoi)]
    );
    for i in [0, 3, 4] {
        assert_eq!(s.occurrences[i].malformed_lines, 1);
    }
}

#[test]
fn record_and_field_limits_stop_the_whole_review_without_a_partial_success() {
    let many = "ER  -\n".repeat(MAX_RECORDS + 1);
    assert_eq!(parse(&many).unwrap_err(), InputError::RecordLimit);
    let many = String::from("TY  - JOUR\n") + &"TI  - x\n".repeat(MAX_FIELDS) + "ER  -\n";
    assert_eq!(parse(&many).unwrap_err(), InputError::FieldLimit);
    assert!(parse(" \r\n\t\n").unwrap().occurrences.is_empty());
}
