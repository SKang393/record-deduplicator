use dedup_core::normalize::pagination::{
    Pagination, PaginationComparison, compare_pagination, parse_pagination,
};

fn compare(a: (&str, Option<&str>), b: (&str, Option<&str>)) -> PaginationComparison {
    compare_pagination(&parse_pagination(a.0, a.1), &parse_pagination(b.0, b.1))
}

#[test]
fn combined_and_separate_ranges_agree_after_expansion() {
    for range in ["478-82", "478\u{2013}82", "478\u{2014}82", "478\u{2212}82"] {
        assert_eq!(
            compare((range, None), ("478", Some("482"))),
            PaginationComparison::Agreement
        );
    }
    assert_eq!(
        compare(("198-03", None), ("198", Some("203"))),
        PaginationComparison::Agreement
    );
}

#[test]
fn final_pages_distinguish_conflicts_from_missing_evidence() {
    assert_eq!(
        compare(("81", Some("89")), ("81", Some("90"))),
        PaginationComparison::Conflict
    );
    assert_eq!(
        compare(("81", None), ("81", Some("90"))),
        PaginationComparison::Unknown
    );
    assert_eq!(
        compare(("S4-S8", None), ("4-8", None)),
        PaginationComparison::Conflict
    );
    assert!(matches!(
        parse_pagination("91-80", None),
        Pagination::Unrecognized(_)
    ));
    assert_eq!(
        compare(("91-80", None), ("91-80", None)),
        PaginationComparison::Unknown
    );
}

#[test]
fn document_lengths_and_provisional_values_are_not_matching_evidence() {
    assert_eq!(parse_pagination("12 pp.", None), Pagination::PageCount(12));
    for value in ["12 pp.", "online ahead of print", "", "?"] {
        assert_eq!(
            compare((value, None), (value, None)),
            PaginationComparison::Unknown
        );
    }
    assert_eq!(
        compare(("12 p", None), ("12", None)),
        PaginationComparison::Unknown
    );
}

#[test]
fn article_numbers_compare_only_with_other_article_numbers() {
    assert_eq!(
        compare(("Article number E12345", None), ("e12345", None)),
        PaginationComparison::Agreement
    );
    assert_eq!(
        compare(("e12345", None), ("e12346", None)),
        PaginationComparison::Conflict
    );
    assert_eq!(
        compare(("e12345", None), ("12345", None)),
        PaginationComparison::Unknown
    );
}
