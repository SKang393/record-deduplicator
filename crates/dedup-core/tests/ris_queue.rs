use dedup_core::identifier_review::{InputError, Issue, MAX_INPUT_BYTES, MAX_RECORDS};
use dedup_core::ris::{
    Disposition::*, MAX_FIELDS, MAX_SOURCES, QueueError, review_sources, valid_source_label,
};

#[test]
fn identical_and_empty_exports_keep_their_declared_source_identity() {
    let input = "TY  - JOUR\rDO  - 10.0000/example\rER  -\rstray";
    let queue = review_sources(&[("z", input), ("empty", ""), ("a", input)]).unwrap();
    let labels: Vec<_> = queue.sources.iter().map(|source| source.label).collect();
    assert_eq!(labels, ["a", "empty", "z"]);
    assert!(queue.sources[1].source.occurrences.is_empty());
    assert_eq!(queue.review.retained_records, 4);
    assert_eq!(queue.review.groups.len(), 1);
    assert_eq!(queue.review.groups[0].rows, [1, 3]);
    let entries: Vec<_> = queue
        .entries
        .iter()
        .map(|entry| (entry.label, entry.ordinal, entry.disposition))
        .collect();
    assert_eq!(
        entries,
        [
            ("a", 1, ReviewCandidate),
            ("a", 2, ReviewInput),
            ("z", 1, ReviewCandidate),
            ("z", 2, ReviewInput),
        ]
    );
}

#[test]
fn issue_ordinals_follow_file_boundaries_and_candidate_status_preserves_conflicts() {
    let a = "TY  - JOUR\nDO  - 10.0000/a\nER  -\nTY  - JOUR\nDO  - 10.0000/b\nER  -";
    let b = "TY  - JOUR\nDO  - 10.0000/b\nUR  - https://pubmed.ncbi.nlm.nih.gov/123456789012345678901/\nUR  - https://pubmed.ncbi.nlm.nih.gov/123456789012345678902/\nER  -\nTY  - JOUR\nER  -";
    let queue = review_sources(&[("first", a), ("second", b)]).unwrap();
    assert_eq!(queue.review.groups[0].rows, [2, 3]);
    assert_eq!(
        queue.review.groups[0].other_evidence,
        dedup_core::identifier_review::OtherEvidence::Conflict
    );
    assert_eq!(
        queue.review.issues,
        [(3, Issue::ConflictingPmid), (4, Issue::NoIdentifiers)]
    );
    let dispositions: Vec<_> = queue
        .entries
        .iter()
        .map(|entry| entry.disposition)
        .collect();
    assert_eq!(
        dispositions,
        [Retained, ReviewCandidate, ReviewCandidate, ReviewInput]
    );
    let source = &queue.sources[1].source;
    let first = &source.occurrences[0];
    assert_eq!(
        &source.original[first.bytes.clone()],
        &b[..b.rfind("TY  -").unwrap()]
    );
}

#[test]
fn source_labels_are_unique_bounded_and_safe_in_review_references() {
    for label in ["a", "A.b-c_2", "1", &"A".repeat(64)] {
        assert!(valid_source_label(label));
    }
    for label in [
        "",
        ".",
        "_a",
        "a:b",
        "a b",
        "a/b",
        "a\tb",
        "é",
        &"a".repeat(65),
    ] {
        assert!(!valid_source_label(label));
        assert_eq!(
            review_sources(&[(label, "")]).unwrap_err(),
            QueueError::InvalidLabel
        );
    }
    assert_eq!(review_sources(&[]).unwrap_err(), QueueError::SourceCount);
    assert_eq!(
        review_sources(&[("same", ""), ("same", "")]).unwrap_err(),
        QueueError::DuplicateLabel
    );
    let labels: Vec<_> = (0..=MAX_SOURCES).map(|i| format!("s{i}")).collect();
    let sources: Vec<_> = labels.iter().map(|s| (s.as_str(), "")).collect();
    assert_eq!(
        review_sources(&sources[..MAX_SOURCES])
            .unwrap()
            .sources
            .len(),
        MAX_SOURCES
    );
    assert_eq!(
        review_sources(&sources).unwrap_err(),
        QueueError::SourceCount
    );
}

#[test]
fn limits_apply_to_the_entire_queue_including_later_sources() {
    let fields = "TI  - x\n".repeat(MAX_FIELDS - 2);
    let records = "ER  -\n".repeat(MAX_RECORDS - 1);
    let bytes = " ".repeat(MAX_INPUT_BYTES - 1);
    for (first, last, extra, error) in [
        (
            fields.as_str(),
            "TY  - JOUR\nER  -",
            "Q1  -",
            InputError::FieldLimit,
        ),
        (records.as_str(), "ER  -", "stray", InputError::RecordLimit),
        (bytes.as_str(), " ", " ", InputError::ByteLimit),
    ] {
        let exact = [("a", first), ("b", last)];
        assert!(review_sources(&exact).is_ok());
        let over = [exact[0], exact[1], ("c", extra)];
        assert_eq!(review_sources(&over).unwrap_err(), QueueError::Input(error));
    }
}
