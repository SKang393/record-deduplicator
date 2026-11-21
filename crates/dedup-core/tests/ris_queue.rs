use dedup_core::identifier_review::{InputError, Issue, MAX_INPUT_BYTES, MAX_RECORDS};
use dedup_core::ris::{
    Disposition::*, MAX_FIELDS, MAX_SOURCES, QueueError, review_sources, valid_source_label,
};

#[test]
fn scopus_article_numbers_supply_only_direct_cross_field_inspection_leads() {
    let record = |fields: &str| {
        format!(
            "TY  - JOUR\nTI  - A census of painted wooden wheels\nAU  - Reed, Q\n\
             PY  - 2024\nJO  - Fictional Cart Journal\nVL  - 7\n{fields}\nER  -\n"
        )
    };
    let scopus = record("DB  - Scopus\nC7  - 073021\nDO  - 10.0000/wheels");
    let peer = record("SP  - Article number 073021\nUR  - https://pubmed.ncbi.nlm.nih.gov/980001/");
    let pairs = |left: &str, right: &str| {
        review_sources(&[("one", left), ("two", right)])
            .unwrap()
            .locator_pairs()
            .complementary_ids
    };
    assert_eq!(pairs(&scopus, &peer), [[1, 2]]);
    let queue = review_sources(&[("one", &scopus), ("two", &peer)]).unwrap();
    assert!(queue.review.groups.is_empty() && queue.metadata_hints().is_empty());
    assert!(
        queue.sources[0].source.occurrences[0]
            .publication_details()
            .is_none()
    );

    for field in ["SP  - 073021", "C7  - 073021", "SP  - Article number 73021"] {
        assert!(pairs(&scopus, &peer.replace("SP  - Article number 073021", field)).is_empty());
    }
    for db in [
        "DB  - Other",
        "DB  - Scopus\nSP  - 1",
        "DB  - Scopus\nC7  - 73022",
    ] {
        assert!(pairs(&scopus.replace("DB  - Scopus", db), &peer).is_empty());
    }
    let bystander = scopus.replace("10.0000/wheels", "10.0000/bystander");
    assert!(pairs(&format!("{scopus}{bystander}"), &peer).is_empty());
    let standard = record("SP  - Article number 073021\nDO  - 10.0000/ordinary");
    assert_eq!(pairs(&format!("{standard}{bystander}"), &peer), [[1, 3]]);
}

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

#[test]
fn metadata_hints_report_large_blocks_without_expanding_pairs() {
    use dedup_core::ris::{MAX_HINT_MEMBERS, MetadataHint};
    let rec = "TY  - JOUR\nTI  - Reading the tides\nAU  - Rowan, A.\nPY  - 2020\nER  -\n";
    let input = rec.repeat(MAX_HINT_MEMBERS);
    let queue = review_sources(&[("a", &input)]).unwrap();
    assert_eq!(
        queue.metadata_hints(),
        [MetadataHint::Inspect((1..=MAX_HINT_MEMBERS).collect())]
    );
    let queue = review_sources(&[("b", rec), ("a", &input)]).unwrap();
    assert_eq!(
        queue.metadata_hints(),
        [MetadataHint::Overflow {
            first: 1,
            count: MAX_HINT_MEMBERS + 1
        }]
    );
    assert!(queue.review.groups.is_empty());
}

#[test]
fn incomplete_or_contradictory_metadata_does_not_supply_a_key() {
    use dedup_core::ris::MetadataHint;
    let base = "TY  - JOUR\nTI  - Reading the tides\nAU  - Rowan, A.\nPY  - 2020\nER  -\n";
    let mut input = base.to_owned();
    for replacement in [
        base.replace("ER  -", "T1  - Reading the tides: protocol\nER  -"),
        base.replace("ER  -", "A1  - Rowan, B.\nER  -"),
        base.replace("ER  -", "Y1  - 2021\nER  -"),
        base.replace("Reading the tides", "Reading the tides\n      in winter"),
        base.replace("PY  - 2020", "PY  - 2020/2021"),
        base.replace("PY  - 2020", "PY  - 2020/13/01"),
        base.replace("AU  - Rowan, A.", "AU  - unknown"),
        base.replace("ER  -\n", ""),
    ] {
        input.push_str(&replacement);
    }
    input.push_str(&base.replace(
        "ER  -",
        "T1  - READING THE TIDES\nA1  - rowan, a.\nY1  - 2020/03/04/\nER  -",
    ));
    let queue = review_sources(&[("test", &input)]).unwrap();
    assert_eq!(queue.metadata_hints(), [MetadataHint::Inspect(vec![1, 10])]);
}
