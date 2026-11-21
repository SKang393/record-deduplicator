//! Direct inspection leads only. These pairs never define sheet membership.
use super::{
    Occurrence, Pagination, PublicationLocator, ReviewQueue, publication_year, text, title,
};
use crate::ris::{Disposition, ReportIdentifiers};
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

impl Occurrence<'_> {
    // This exporter-specific locator is used only for inspection across field types.
    fn scopus_article_locator(&self) -> Option<PublicationLocator> {
        if self.fields.iter().any(|f| matches!(f.tag, "SP" | "EP"))
            || !self.scalar("DB")??.eq_ignore_ascii_case("Scopus")
        {
            return None;
        }
        let article = self.scalar("C7")??.to_ascii_lowercase();
        let digits = article.strip_prefix('e').unwrap_or(&article);
        if digits.len() < 5 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        self.locator_with_pagination(Pagination::ArticleNumber(article))
    }
}

#[derive(Default)]
struct TitleBlock<'a> {
    standard: Vec<Member<'a>>,
    scopus_articles: Vec<Member<'a>>,
}

/// Direct one-based queue rows for two distinct kinds of inspection evidence.
#[derive(Default)]
pub struct LocatorPairs {
    pub title_variants: Vec<[usize; 2]>,
    pub complementary_ids: Vec<[usize; 2]>,
}

impl ReviewQueue<'_> {
    pub fn variant_title_pairs(&self) -> Vec<[usize; 2]> {
        if self.entries.iter().all(|entry| entry.has_usable_identifier) {
            return Vec::new();
        }
        self.locator_pairs().title_variants
    }

    fn complementary_id_pair(&self, a: &Member<'_>, b: &Member<'_>) -> bool {
        let left = &self.entries[a.row - 1];
        let right = &self.entries[b.row - 1];
        if !a.full_title
            || !b.full_title
            || left.label == right.label
            || left.disposition != Disposition::Retained
            || right.disposition != Disposition::Retained
            || title(a.record) != title(b.record)
        {
            return false;
        }
        let Some(authors) = a.record.authors() else {
            return false;
        };
        if b.record.authors().as_ref() != Some(&authors) {
            return false;
        }
        // Retained entries have neither an identifier issue nor a shared-ID group.
        // Thus two supported namespaces without disagreement must be split across the rows.
        let mut ids = ReportIdentifiers::default();
        ids.observe(a.record);
        ids.observe(b.record);
        ids.conflict().is_none() && ids.doi.key.is_some() && ids.pmid.key.is_some()
    }

    /// Index a complete title and at most one explicit subtitle omission.
    /// A fifth paginated member withholds that key. C7 leads require one
    /// member of each kind. Never compare two omissions.
    pub fn locator_pairs(&self) -> LocatorPairs {
        let mut pairs = LocatorPairs::default();
        if self.sources.len() < 2 {
            return pairs;
        }
        let mut locations = BTreeMap::new();
        let records = self.sources.iter().flat_map(|s| &s.source.occurrences);
        for (index, record) in records.enumerate() {
            if !record.usable() {
                continue;
            }
            let (Some(title), Some(year), Some(kind)) = (
                title(record),
                publication_year(record),
                record.fields.first().and_then(text),
            ) else {
                continue;
            };
            let (locator, from_c7) = if let Some(locator) = record.publication_locator() {
                (locator, false)
            } else if let Some(locator) = record.scopus_article_locator() {
                (locator, true)
            } else {
                continue;
            };
            let titles: &mut BTreeMap<String, TitleBlock> =
                locations.entry((locator, year, kind)).or_default();
            for (form, full_title) in [(title_key(&title), true), (without_subtitle(&title), false)]
            {
                let Some(form) = form else { continue };
                if from_c7 && !full_title {
                    continue;
                }
                let block = titles.entry(form).or_default();
                // Keep separate caps so a C7 row cannot crowd out an ordinary lead.
                let (members, limit) = if from_c7 {
                    (&mut block.scopus_articles, 1)
                } else {
                    (&mut block.standard, MAX_BLOCK)
                };
                if members.len() <= limit {
                    members.push(Member {
                        row: index + 1,
                        record,
                        full_title,
                    });
                }
            }
        }

        for block in locations.into_values().flat_map(BTreeMap::into_values) {
            if let ([a], [b]) = (block.scopus_articles.as_slice(), block.standard.as_slice()) {
                if !b.record.fields.iter().any(|f| f.tag == "C7")
                    && self.complementary_id_pair(a, b)
                {
                    pairs
                        .complementary_ids
                        .push([a.row.min(b.row), a.row.max(b.row)]);
                }
            }
            let block = block.standard;
            if !(2..=MAX_BLOCK).contains(&block.len()) {
                continue;
            }
            if let [a, b] = block.as_slice() {
                if self.complementary_id_pair(a, b) {
                    pairs.complementary_ids.push([a.row, b.row]);
                }
            }
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
                        pairs.title_variants.push([a.row, b.row]);
                    }
                }
            }
        }
        pairs.title_variants.sort_unstable();
        pairs.complementary_ids.sort_unstable();
        pairs
    }
}
