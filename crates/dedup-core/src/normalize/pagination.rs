//! Pagination comparison without treating document lengths as page numbers.

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Pagination {
    Missing,
    Provisional,
    PageCount(u32),
    ArticleNumber(String),
    Pages {
        start: String,
        end: Option<String>,
        supplement: bool,
    },
    Unrecognized(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaginationComparison {
    Agreement,
    Conflict,
    Unknown,
}

/// Online-first and other provisional labels do not establish final pages.
pub fn is_provisional_metadata(value: &str) -> bool {
    let lower = value.to_lowercase();
    [
        "online first",
        "online ahead of print",
        "ahead of print",
        "advance online",
        "in press",
        "early access",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn page_count(value: &str) -> Option<u32> {
    let compact: String = value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    let compact = compact.trim_end_matches('.');
    let digits = compact
        .strip_suffix("pp")
        .or_else(|| compact.strip_suffix('p'))?;
    if digits.is_empty() || !digits.bytes().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let count = digits.parse::<u32>().ok()?;
    (count > 0).then_some(count)
}

fn article_number(value: &str) -> Option<String> {
    let lower = value.trim().to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("article number ")
        .or_else(|| lower.strip_prefix("article no. "))
        .or_else(|| lower.strip_prefix("art. no. "))
    {
        let rest = rest.trim();
        return (!rest.is_empty()
            && rest
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '.'))
        .then(|| rest.to_owned());
    }
    let digits = lower.strip_prefix('e')?;
    (digits.len() >= 5 && digits.bytes().all(|ch| ch.is_ascii_digit())).then_some(lower)
}

fn valid_page(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|ch| ch.is_ascii_alphanumeric())
        && value.chars().any(|ch| ch.is_ascii_digit())
}

fn expand_elided_end(start: &str, end: &str) -> Option<String> {
    if !start.bytes().all(|ch| ch.is_ascii_digit()) || !end.bytes().all(|ch| ch.is_ascii_digit()) {
        return Some(end.to_ascii_lowercase());
    }
    if end.len() >= start.len() {
        return Some(end.to_owned());
    }
    let start_num = start.parse::<u64>().ok()?;
    let end_num = end.parse::<u64>().ok()?;
    let power = u32::try_from(end.len()).ok()?;
    let base = 10_u64.checked_pow(power)?;
    let mut expanded = (start_num / base).checked_mul(base)?.checked_add(end_num)?;
    if expanded < start_num {
        expanded = expanded.checked_add(base)?;
    }
    Some(expanded.to_string())
}

/// Interpret a start/page field and an optional separate end field. The result
/// is comparison metadata; callers retain the original fields for export.
pub fn parse_pagination(start: &str, end: Option<&str>) -> Pagination {
    let raw_start = start.trim();
    let raw_end = end.map(str::trim).filter(|value| !value.is_empty());
    if raw_start.is_empty() {
        return Pagination::Missing;
    }
    if is_provisional_metadata(raw_start) || raw_end.is_some_and(is_provisional_metadata) {
        return Pagination::Provisional;
    }
    if raw_end.is_none() {
        if let Some(count) = page_count(raw_start) {
            return Pagination::PageCount(count);
        }
        if let Some(article) = article_number(raw_start) {
            return Pagination::ArticleNumber(article);
        }
    }
    let normalized = raw_start.replace(['–', '—', '−'], "-");
    let (first, last) = if let Some(end) = raw_end {
        (normalized.trim(), Some(end))
    } else if let Some((first, last)) = normalized.split_once('-') {
        (first.trim(), Some(last.trim()))
    } else {
        (normalized.trim(), None)
    };
    if !valid_page(first) || last.is_some_and(|value| !valid_page(value)) {
        return Pagination::Unrecognized(raw_start.to_owned());
    }
    let first = first.to_ascii_lowercase();
    let last = match last {
        Some(last) => match expand_elided_end(&first, last) {
            Some(last) => Some(last),
            None => return Pagination::Unrecognized(raw_start.to_owned()),
        },
        None => None,
    };
    if let (Ok(start), Some(Ok(end))) =
        (first.parse::<u64>(), last.as_deref().map(str::parse::<u64>))
    {
        if end < start {
            return Pagination::Unrecognized(raw_start.to_owned());
        }
    }
    let supplement = first.starts_with('s')
        || first
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, 'i' | 'v' | 'x'));
    Pagination::Pages {
        start: first,
        end: last,
        supplement,
    }
}

/// Missing or provisional pagination is unknown, not agreement or contradiction.
/// Document page counts are not used as evidence of report identity.
pub fn compare_pagination(left: &Pagination, right: &Pagination) -> PaginationComparison {
    match (left, right) {
        (
            Pagination::Pages {
                start: a,
                end: a_end,
                supplement: a_supplement,
            },
            Pagination::Pages {
                start: b,
                end: b_end,
                supplement: b_supplement,
            },
        ) => {
            if a != b || a_supplement != b_supplement {
                return PaginationComparison::Conflict;
            }
            match (a_end, b_end) {
                (Some(a), Some(b)) if a != b => PaginationComparison::Conflict,
                (Some(_), None) | (None, Some(_)) => PaginationComparison::Unknown,
                _ => PaginationComparison::Agreement,
            }
        }
        (Pagination::ArticleNumber(a), Pagination::ArticleNumber(b)) => {
            if a == b {
                PaginationComparison::Agreement
            } else {
                PaginationComparison::Conflict
            }
        }
        _ => PaginationComparison::Unknown,
    }
}
