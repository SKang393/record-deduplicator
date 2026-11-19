//! Group input occurrences by a shared full identifier without merging records.
//! Ordinals are one-based data row numbers (the TSV header is not a record).

use std::collections::BTreeMap;

pub const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RECORDS: usize = 100_000;
const MAX_ID_BYTES: usize = 2048;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Identifier {
    Doi,
    Pmid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OtherEvidence {
    /// At least two different supported identifiers in the other column.
    Conflict,
    /// Every row has the same supported value in the other column.
    Agreement,
    /// Missing or unsupported values prevent complete agreement.
    Incomplete,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Group {
    pub shared: Identifier,
    pub rows: Vec<usize>,
    pub other_evidence: OtherEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Issue {
    ColumnCount,
    UnsupportedDoi,
    UnsupportedPmid,
    NoIdentifiers,
    ConflictingDoi,
    ConflictingPmid,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Review {
    pub retained_records: usize,
    pub candidate_records: usize,
    pub groups: Vec<Group>,
    pub issues: Vec<(usize, Issue)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputError {
    ByteLimit,
    RecordLimit,
    Header,
    FieldLimit,
}

#[derive(Default)]
pub(crate) struct Row {
    pub(crate) doi: Option<String>,
    pub(crate) pmid: Option<String>,
    pub(crate) doi_conflict: bool,
    pub(crate) pmid_conflict: bool,
}

fn decode_path(raw: &str) -> Option<String> {
    let mut result = Vec::with_capacity(raw.len());
    let mut bytes = raw.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let high = char::from(bytes.next()?).to_digit(16)?;
            let low = char::from(bytes.next()?).to_digit(16)?;
            result.push((high * 16 + low) as u8);
        } else {
            result.push(byte);
        }
    }
    String::from_utf8(result).ok()
}

pub(crate) fn doi_key(raw: &str) -> Option<String> {
    if raw.len() > MAX_ID_BYTES {
        return None;
    }
    let text = raw.trim().to_ascii_lowercase();
    let prefixes = [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
    ];
    let doi = if let Some(path) = prefixes.iter().find_map(|prefix| text.strip_prefix(prefix)) {
        // Query strings and fragments are not part of a resolver path.
        if path.contains(['?', '#']) {
            return None;
        }
        decode_path(path)?.to_ascii_lowercase()
    } else {
        text.strip_prefix("doi:")
            .unwrap_or(&text)
            .trim_start()
            .to_owned()
    };
    let (prefix, suffix) = doi.strip_prefix("10.")?.split_once('/')?;
    if prefix.len() < 4
        || !prefix.bytes().all(|ch| ch.is_ascii_digit())
        || suffix.is_empty()
        || !suffix.bytes().all(|ch| ch.is_ascii_graphic())
    {
        return None;
    }
    Some(doi)
}

pub(crate) fn pmid_key(raw: &str) -> Option<String> {
    let digits = raw.trim();
    (!digits.is_empty()
        && digits.len() <= MAX_ID_BYTES
        && !digits.starts_with('0')
        && digits.bytes().all(|b| b.is_ascii_digit()))
    .then(|| digits.to_owned())
}

fn value(row: &Row, id: Identifier) -> Option<&str> {
    match id {
        Identifier::Doi => row.doi.as_deref(),
        Identifier::Pmid => row.pmid.as_deref(),
    }
}

fn groups_for(
    rows: &[Row],
    shared: Identifier,
    other: Identifier,
    seen: &mut [bool],
) -> Vec<Group> {
    let mut blocks: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        if let Some(key) = value(row, shared) {
            blocks.entry(key).or_default().push(index + 1);
        }
    }
    let mut groups = Vec::new();
    for ordinals in blocks.into_values().filter(|block| block.len() > 1) {
        let mut first = None;
        let mut missing = false;
        let mut conflict = false;
        for &ordinal in &ordinals {
            seen[ordinal - 1] = true;
            let row = &rows[ordinal - 1];
            conflict |= match other {
                Identifier::Doi => row.doi_conflict,
                Identifier::Pmid => row.pmid_conflict,
            };
            match value(row, other) {
                None => missing = true,
                Some(key) => match first {
                    None => first = Some(key),
                    Some(prev) if prev != key => conflict = true,
                    _ => {}
                },
            }
        }
        let other_evidence = if conflict {
            OtherEvidence::Conflict
        } else if missing {
            OtherEvidence::Incomplete
        } else {
            OtherEvidence::Agreement
        };
        groups.push(Group {
            shared,
            rows: ordinals,
            other_evidence,
        });
    }
    groups.sort_unstable_by_key(|g| g.rows[0]);
    groups
}

/// Accept a UTF-8 TSV projection with exactly the header `doi<TAB>pmid`.
/// A malformed data row is retained and diagnosed. There is no transitive
/// closure or pair expansion: each occurrence belongs to at most two groups.
pub fn review_tsv(input: &str) -> Result<Review, InputError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(InputError::ByteLimit);
    }
    let mut lines = input.strip_prefix('\u{feff}').unwrap_or(input).lines();
    if lines.next() != Some("doi\tpmid") {
        return Err(InputError::Header);
    }
    let mut rows = Vec::new();
    let mut issues = Vec::new();
    for line in lines {
        if rows.len() >= MAX_RECORDS {
            return Err(InputError::RecordLimit);
        }
        let ordinal = rows.len() + 1;
        let mut row = Row::default();
        match line
            .split_once('\t')
            .filter(|(_, right)| !right.contains('\t'))
        {
            None => issues.push((ordinal, Issue::ColumnCount)),
            Some((doi, pmid)) => {
                row.doi = doi_key(doi);
                row.pmid = pmid_key(pmid);
                if !doi.trim().is_empty() && row.doi.is_none() {
                    issues.push((ordinal, Issue::UnsupportedDoi));
                }
                if !pmid.trim().is_empty() && row.pmid.is_none() {
                    issues.push((ordinal, Issue::UnsupportedPmid));
                }
                if doi.trim().is_empty() && pmid.trim().is_empty() {
                    issues.push((ordinal, Issue::NoIdentifiers));
                }
            }
        }
        rows.push(row);
    }
    Ok(review_rows(&rows, issues))
}

pub(crate) fn review_rows(rows: &[Row], issues: Vec<(usize, Issue)>) -> Review {
    let mut seen = vec![false; rows.len()];
    let mut groups = groups_for(rows, Identifier::Doi, Identifier::Pmid, &mut seen);
    groups.extend(groups_for(
        rows,
        Identifier::Pmid,
        Identifier::Doi,
        &mut seen,
    ));
    Review {
        retained_records: rows.len(),
        candidate_records: seen.into_iter().filter(|&b| b).count(),
        groups,
        issues,
    }
}
