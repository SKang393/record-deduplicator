//! Direct inspection leads only. These pairs never define sheet membership.
use super::{Occurrence, ReviewQueue, publication_year, text, title};
use crate::ris::ReportIdentifiers;
use std::collections::BTreeMap;

const MIN_WORDS: usize = 5;
const MAX_BLOCK: usize = 4;

fn words(title: &str) -> impl Iterator<Item = &str> {
    title
        .split(|c: char| {
            c.is_whitespace() || matches!(c, '.' | ',' | ':' | ';' | '-' | '–' | '—' | '(' | ')')
        })
        .filter(|word| !word.is_empty())
}

fn title_key(title: &str) -> Option<String> {
    let mut key = String::new();
    let mut count = 0;
    for word in words(title) {
        if count > 0 {
            key.push(' ');
        }
        key.push_str(word);
        count += 1;
    }
    (count >= MIN_WORDS).then_some(key)
}

fn without_subtitle(title: &str) -> Option<String> {
    // Strip one explicit trailing segment. Comparing only complete titles to
    // stems keeps "…: protocol" and "…: results" from matching each other.
    let (stem, suffix) = [": ", " — ", " – ", " - ", " ("]
        .into_iter()
        .filter(|&separator| separator != " (" || title.ends_with(')'))
        .filter_map(|separator| title.rsplit_once(separator))
        .max_by_key(|(stem, _)| stem.len())?;
    if (1..=8).contains(&words(suffix).take(9).count()) {
        title_key(stem)
    } else {
        None
    }
}

fn first_family(record: &Occurrence<'_>) -> Option<String> {
    let field = record
        .fields
        .iter()
        .find(|f| matches!(f.tag, "AU" | "A1"))?;
    let author = text(field)?;
    let family = author.split_once(',')?.0.trim();
    (!family.is_empty()).then(|| family.to_owned())
}

struct Member<'a> {
    row: usize,
    record: &'a Occurrence<'a>,
    full_title: bool,
}

impl ReviewQueue<'_> {
    /// Index a complete title and at most one explicit subtitle omission.
    /// A fifth member withholds the entire key. Never compare two omissions.
    pub fn variant_title_pairs(&self) -> Vec<[usize; 2]> {
        if self.entries.iter().all(|e| e.has_usable_identifier) {
            return Vec::new();
        }
        let mut locations = BTreeMap::new();
        let records = self.sources.iter().flat_map(|s| &s.source.occurrences);
        for (index, record) in records.enumerate() {
            if !record.usable() {
                continue;
            }
            let (Some(locator), Some(title), Some(year), Some(kind)) = (
                record.publication_locator(),
                title(record),
                publication_year(record),
                record.fields.first().and_then(text),
            ) else {
                continue;
            };
            let titles: &mut BTreeMap<String, Vec<Member>> =
                locations.entry((locator, year, kind)).or_default();
            for (form, full_title) in [(title_key(&title), true), (without_subtitle(&title), false)]
            {
                let Some(form) = form else { continue };
                let block = titles.entry(form).or_default();
                if block.len() <= MAX_BLOCK {
                    block.push(Member {
                        row: index + 1,
                        record,
                        full_title,
                    });
                }
            }
        }

        let mut pairs = Vec::new();
        for block in locations
            .into_values()
            .flat_map(BTreeMap::into_values)
            .filter(|b| (2..=MAX_BLOCK).contains(&b.len()))
        {
            for (i, a) in block.iter().enumerate() {
                for b in &block[i + 1..] {
                    let left = &self.entries[a.row - 1];
                    let right = &self.entries[b.row - 1];
                    if (!a.full_title && !b.full_title)
                        || left.label == right.label
                        || (left.has_usable_identifier && right.has_usable_identifier)
                        || (a.full_title && b.full_title && title(a.record) == title(b.record))
                    {
                        continue;
                    }
                    if matches!((first_family(a.record), first_family(b.record)), (Some(l), Some(r)) if l != r)
                    {
                        continue;
                    }
                    let mut ids = ReportIdentifiers::default();
                    ids.observe(a.record);
                    ids.observe(b.record);
                    if ids.conflict().is_none() {
                        pairs.push([a.row, b.row]);
                    }
                }
            }
        }
        pairs.sort_unstable();
        pairs
    }
}
