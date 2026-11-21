//! Named RIS sources retain independent byte spans and one-based ordinals.

use super::{ParseCounts, Source, parse_source};
use crate::identifier_review::{InputError, Review, review_rows};

pub const MAX_SOURCES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueError {
    SourceCount,
    InvalidLabel,
    DuplicateLabel,
    Input(InputError),
}

/// A label identifies a source independently of its file path and argument position.
pub fn valid_source_label(label: &str) -> bool {
    let mut bytes = label.bytes();
    label.len() <= 64
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[derive(Debug)]
pub struct NamedSource<'a> {
    pub label: &'a str,
    pub source: Source<'a>,
}

/// Every disposition retains the original occurrence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    ReviewCandidate,
    ReviewInput,
    Retained,
}

#[derive(Debug)]
pub struct QueueEntry<'a> {
    pub label: &'a str,
    pub ordinal: usize,
    pub disposition: Disposition,
    pub(super) has_usable_identifier: bool,
}

#[derive(Debug)]
pub struct ReviewQueue<'a> {
    pub sources: Vec<NamedSource<'a>>,
    /// Entries follow source label, then local ordinal. Row n in the review
    /// refers to `entries[n - 1]`.
    pub entries: Vec<QueueEntry<'a>>,
    pub review: Review,
}

/// Parse each input independently under shared limits and group identifiers
/// across all occurrences. Equal bytes under distinct labels remain separate.
pub fn review_sources<'a>(inputs: &[(&'a str, &'a str)]) -> Result<ReviewQueue<'a>, QueueError> {
    if !(1..=MAX_SOURCES).contains(&inputs.len()) {
        return Err(QueueError::SourceCount);
    }
    if inputs.iter().any(|&(label, _)| !valid_source_label(label)) {
        return Err(QueueError::InvalidLabel);
    }
    let mut ordered = inputs.to_vec();
    ordered.sort_unstable_by_key(|&(label, _)| label);
    if ordered.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(QueueError::DuplicateLabel);
    }

    let mut counts = ParseCounts::default();
    let mut sources = Vec::with_capacity(ordered.len());
    let mut entries = Vec::new();
    let mut rows = Vec::new();
    let mut issues = Vec::new();
    for (label, input) in ordered {
        let source = parse_source(input, &mut counts).map_err(QueueError::Input)?;
        source.append_rows(&mut rows, &mut issues);
        for (index, occurrence) in source.occurrences.iter().enumerate() {
            let row = &rows[entries.len()];
            entries.push(QueueEntry {
                label,
                ordinal: index + 1,
                has_usable_identifier: row.doi.is_some() || row.pmid.is_some(),
                disposition: if occurrence.usable() {
                    Disposition::Retained
                } else {
                    Disposition::ReviewInput
                },
            });
        }
        sources.push(NamedSource { label, source });
    }
    let review = review_rows(&rows, issues);
    for &(row, _) in &review.issues {
        entries[row - 1].disposition = Disposition::ReviewInput;
    }
    for group in &review.groups {
        for &row in &group.rows {
            entries[row - 1].disposition = Disposition::ReviewCandidate;
        }
    }
    Ok(ReviewQueue {
        sources,
        entries,
        review,
    })
}
