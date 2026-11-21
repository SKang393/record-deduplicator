//! Inspection-only bibliographic leads for occurrences without a usable identifier.
use std::collections::BTreeMap;

use super::{Field, Occurrence, ReviewQueue};
use crate::normalize::pagination::{Pagination, parse_pagination};

mod variants;
pub use variants::LocatorPairs;
mod citation;
pub use citation::CitationValue;

pub const MAX_HINT_MEMBERS: usize = 16;

/// Members and the overflow reference are one-based rows in the named review queue.
#[derive(Debug, PartialEq, Eq)]
pub enum MetadataHint {
    Inspect(Vec<usize>),
    Overflow { first: usize, count: usize },
}

#[derive(Default)]
struct Block {
    members: Vec<usize>,
    count: usize,
    includes_unidentified: bool,
}

// Repeated representations of a scalar (e.g. TI/T1 or PY/Y1) must agree.
fn agreed<T: PartialEq>(mut values: impl Iterator<Item = Option<T>>) -> Option<T> {
    let first = values.next()??;
    values
        .all(|value| value.as_ref() == Some(&first))
        .then_some(first)
}

fn text(field: &Field<'_>) -> Option<String> {
    if field.continued {
        return None;
    }
    let mut words = field.first_line_value.split_whitespace();
    let mut value = words.next()?.to_owned();
    for word in words {
        value.push(' ');
        value.push_str(word);
    }
    let value = value.to_lowercase();
    (value.chars().any(char::is_alphabetic)
        && !matches!(
            value.as_str(),
            "unknown" | "anonymous" | "n/a" | "not available"
        ))
    .then_some(value)
}

fn year(field: &Field<'_>) -> Option<u16> {
    if field.continued {
        return None;
    }
    let mut parts = field
        .first_line_value
        .trim()
        .trim_end_matches('/')
        .split('/');
    let year = parts.next()?;
    if year.len() != 4 || !year.bytes().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    for max in [12, 31] {
        if let Some(part) = parts.next() {
            if part.len() > 2
                || !part.bytes().all(|ch| ch.is_ascii_digit())
                || !(1..=max).contains(&part.parse::<u8>().ok()?)
            {
                return None;
            }
        }
    }
    let year = year.parse().ok()?;
    (parts.next().is_none() && year != 0).then_some(year)
}

fn title(rec: &Occurrence<'_>) -> Option<String> {
    agreed(
        rec.fields
            .iter()
            .filter(|f| matches!(f.tag, "TI" | "T1"))
            .map(text),
    )
}

fn publication_year(rec: &Occurrence<'_>) -> Option<u16> {
    agreed(
        rec.fields
            .iter()
            .filter(|f| matches!(f.tag, "PY" | "Y1"))
            .map(year),
    )
}

fn key(rec: &Occurrence<'_>) -> Option<(String, String, u16, String)> {
    if !rec.usable() {
        return None;
    }
    let title = title(rec)?;
    let author = agreed(
        ["AU", "A1"]
            .into_iter()
            .filter_map(|tag| rec.fields.iter().find(|f| f.tag == tag))
            .map(text),
    )?;
    let year = publication_year(rec)?;
    Some((title, author, year, text(rec.fields.first()?)?))
}

/// Additional evidence required for an explicitly verified metadata decision.
/// Equality is deliberately strict. A hint by itself supplies none of this evidence.
#[derive(Debug, PartialEq, Eq)]
pub struct PublicationDetails {
    authors: Vec<String>,
    locator: PublicationLocator,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PublicationLocator {
    work_type: Option<String>,
    journal: String,
    volume: String,
    issue: Option<String>,
    pages: Pagination,
}

impl Occurrence<'_> {
    fn authors(&self) -> Option<Vec<String>> {
        agreed(["AU", "A1"].into_iter().filter_map(|tag| {
            let mut fields = self.fields.iter().filter(|f| f.tag == tag).peekable();
            fields.peek()?;
            Some(fields.map(text).collect::<Option<Vec<_>>>())
        }))
    }

    pub fn publication_details(&self) -> Option<PublicationDetails> {
        Some(PublicationDetails {
            authors: self.authors()?,
            locator: self.publication_locator()?,
        })
    }

    // Distinguish an absent optional field from ambiguous or continued values.
    fn scalar(&self, tag: &str) -> Option<Option<&str>> {
        let mut fields = self.fields.iter().filter(|f| f.tag == tag).peekable();
        if fields.peek().is_none() {
            return Some(None);
        }
        agreed(fields.map(|f| {
            let value = f.first_line_value.trim();
            (!f.continued && !value.is_empty()).then_some(value)
        }))
        .map(Some)
    }

    /// Final page range or article number from this occurrence's original SP/EP fields.
    pub fn final_pagination(&self) -> Option<Pagination> {
        let pages = parse_pagination(self.scalar("SP")??, self.scalar("EP")?);
        matches!(
            &pages,
            Pagination::Pages { end: Some(_), .. } | Pagination::ArticleNumber(_)
        )
        .then_some(pages)
    }

    fn journal(&self) -> Option<String> {
        agreed(
            self.fields
                .iter()
                .filter(|f| matches!(f.tag, "JO" | "JF" | "T2"))
                .map(text),
        )
    }

    fn publication_locator(&self) -> Option<PublicationLocator> {
        self.locator_with_pagination(self.final_pagination()?)
    }

    fn locator_with_pagination(&self, pages: Pagination) -> Option<PublicationLocator> {
        let journal = self.journal()?;
        let volume = self.scalar("VL")??;
        if !volume.bytes().any(|b| b.is_ascii_digit()) {
            return None;
        }
        Some(PublicationLocator {
            work_type: self.scalar("M3")?.map(str::to_owned),
            journal,
            volume: volume.to_owned(),
            issue: self.scalar("IS")?.map(str::to_owned),
            pages,
        })
    }
}

impl ReviewQueue<'_> {
    /// Exact metadata blocks do not become ID groups or authorize decisions.
    /// Each occurrence contributes at most one membership. No pairs are expanded.
    pub fn metadata_hints(&self) -> Vec<MetadataHint> {
        if self.entries.iter().all(|entry| entry.has_usable_identifier) {
            return Vec::new();
        }
        let records = self
            .sources
            .iter()
            .flat_map(|named| &named.source.occurrences);
        let mut blocks = BTreeMap::new();
        for (index, (rec, entry)) in records.zip(&self.entries).enumerate() {
            let Some(key) = key(rec) else { continue };
            let block: &mut Block = blocks.entry(key).or_default();
            block.count += 1;
            block.includes_unidentified |= !entry.has_usable_identifier;
            if block.count <= MAX_HINT_MEMBERS {
                block.members.push(index + 1);
            }
        }
        let mut blocks: Vec<_> = blocks
            .into_values()
            .filter(|block| block.includes_unidentified && block.count > 1)
            .collect();
        blocks.sort_unstable_by_key(|block| block.members[0]);
        blocks
            .into_iter()
            .map(|block| {
                if block.count > MAX_HINT_MEMBERS {
                    MetadataHint::Overflow {
                        first: block.members[0],
                        count: block.count,
                    }
                } else {
                    MetadataHint::Inspect(block.members)
                }
            })
            .collect()
    }
}
