use dedup_core::identifier_review::{
    Group, Identifier::*, InputError, Issue, MAX_INPUT_BYTES, MAX_RECORDS, OtherEvidence::*,
    review_tsv,
};

#[test]
fn shared_keys_keep_conflicting_rows_and_never_collapse_chains() {
    let review =
        review_tsv("doi\tpmid\n10.0000/a\t11\nHTTPS://doi.org/10.0000/A\t22\n10.0000/b\t22\n")
            .unwrap();
    assert_eq!((review.retained_records, review.candidate_records), (3, 3));
    assert_eq!(
        review.groups,
        vec![
            Group {
                shared: Doi,
                rows: vec![1, 2],
                other_evidence: Conflict
            },
            Group {
                shared: Pmid,
                rows: vec![2, 3],
                other_evidence: Conflict
            },
        ]
    );
}

#[test]
fn suffixes_punctuation_and_literal_percent_escapes_remain_significant() {
    let r=review_tsv("doi\tpmid\ndoi:10.0000/X.v1\t\nhttps://dx.doi.org/10.0000%2Fx.v1\t\n10.0000/x\t\n10.0000/x.v1.\t\n10.0000/x%2Fz\t\nhttps://doi.org/10.0000/x%252Fz\t\n10.0000/x/z\t\n").unwrap();
    assert_eq!(r.retained_records, 7);
    assert_eq!(
        r.groups
            .iter()
            .map(|g| (&g.rows, g.other_evidence))
            .collect::<Vec<_>>(),
        vec![(&vec![1, 2], Incomplete), (&vec![5, 6], Incomplete)]
    );
}

#[test]
fn missing_and_unsupported_ids_are_not_generic_keys() {
    let r=review_tsv("doi\tpmid\n\t\nnot-a-doi\t01\n\t\n\t0\nhttps://doi.org/10.0000/x?link=1\t-7\nhttps://doi.org/10.0000/%GG\t\n").unwrap();
    assert!(r.groups.is_empty());
    assert_eq!(r.retained_records, 6);
    assert_eq!(
        r.issues,
        vec![
            (1, Issue::NoIdentifiers),
            (2, Issue::UnsupportedDoi),
            (2, Issue::UnsupportedPmid),
            (3, Issue::NoIdentifiers),
            (4, Issue::UnsupportedPmid),
            (5, Issue::UnsupportedDoi),
            (5, Issue::UnsupportedPmid),
            (6, Issue::UnsupportedDoi)
        ]
    );
}

#[test]
fn every_malformed_line_is_retained_and_diagnosed_with_its_ordinal() {
    let r=review_tsv("\u{feff}doi\tpmid\r\nwrong\r\n\r\n10.0000/a\t12\textra\r\n10.0000/a\t12\r\n10.0000/a\twrong\r\n").unwrap();
    assert_eq!(r.retained_records, 5);
    assert_eq!(
        r.groups,
        vec![Group {
            shared: Doi,
            rows: vec![4, 5],
            other_evidence: Incomplete
        }]
    );
    assert_eq!(
        r.issues,
        vec![
            (1, Issue::ColumnCount),
            (2, Issue::ColumnCount),
            (3, Issue::ColumnCount),
            (5, Issue::UnsupportedPmid)
        ]
    );
}

#[test]
fn agreement_is_only_a_review_group_and_conflicts_outrank_missing_values() {
    let r = review_tsv(
        "doi\tpmid\n10.0000/x\t123456789012\n10.0000/x\t123456789012\n10.0000/x\t\n10.0000/x\t9\n",
    )
    .unwrap();
    assert_eq!((r.retained_records, r.candidate_records), (4, 4));
    assert_eq!(r.groups[0].other_evidence, Conflict);
    assert_eq!(r.groups[1].other_evidence, Agreement);
}

#[test]
fn strict_header_and_empty_input_have_distinct_outcomes() {
    for text in [
        "",
        "pmid\tdoi\n",
        "doi,pmid\n",
        "DOI\tPMID\n",
        "doi\tpmid\textra\n",
    ] {
        assert_eq!(review_tsv(text), Err(InputError::Header));
    }
    assert_eq!(review_tsv("doi\tpmid").unwrap().retained_records, 0);
}

#[test]
fn large_shared_blocks_stay_linear_in_memberships_and_fail_closed_at_limits() {
    let header = "doi\tpmid\n";
    let line = "10.0000/x\t7\n";
    let mut text = String::from(header) + &line.repeat(MAX_RECORDS);
    let r = review_tsv(&text).unwrap();
    assert_eq!(r.retained_records, MAX_RECORDS);
    assert_eq!(r.groups.len(), 2);
    assert_eq!(
        r.groups.iter().map(|g| g.rows.len()).sum::<usize>(),
        2 * MAX_RECORDS
    );
    text.push_str(line);
    assert_eq!(review_tsv(&text), Err(InputError::RecordLimit));
    assert_eq!(
        review_tsv(&"x".repeat(MAX_INPUT_BYTES + 1)),
        Err(InputError::ByteLimit)
    );
}
