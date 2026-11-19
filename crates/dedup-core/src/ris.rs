//! Read-only RIS source view. Spans are half-open byte offsets in the original
//! UTF-8 input, including its original line endings. No field is reserialized.

use crate::identifier_review::{
    InputError, Issue, MAX_INPUT_BYTES, MAX_RECORDS, Review, Row, doi_key, pmid_key, review_rows,
};
use std::ops::Range;

pub const MAX_FIELDS: usize = 250_000;

#[derive(Debug)]
pub struct Field<'a> {
    pub tag: &'a str,
    pub first_line_value: &'a str,
    pub bytes: Range<usize>,
    pub continued: bool,
}

#[derive(Debug)]
pub struct Occurrence<'a> {
    pub bytes: Range<usize>,
    pub fields: Vec<Field<'a>>,
    pub has_type: bool,
    pub has_end: bool,
    pub malformed_lines: usize,
}

impl Occurrence<'_> {
    pub fn usable(&self) -> bool {
        self.has_type && self.has_end && self.malformed_lines == 0
    }

    /// Non-identifier metadata remains in the source, even when not interpreted.
    pub fn unmapped_fields(&self) -> usize {
        self.fields
            .iter()
            .filter(|f| f.tag != "TY" && f.tag != "ER" && projected(f).is_none())
            .count()
    }
}

#[derive(Debug)]
pub struct Source<'a> {
    pub original: &'a str,
    pub occurrences: Vec<Occurrence<'a>>,
}

fn tag(line: &str) -> Option<(&str, &str)> {
    let b = line.as_bytes();
    if b.len() < 5
        || !b[0].is_ascii_uppercase()
        || !(b[1].is_ascii_uppercase() || b[1].is_ascii_digit())
        || &b[2..5] != b"  -"
    {
        return None;
    }
    let tail = &line[5..];
    if !tail.is_empty() && !tail.starts_with([' ', '\t']) {
        return None;
    }
    Some((&line[..2], tail.trim_start_matches([' ', '\t'])))
}

/// A nonblank fragment outside TY/ER gets an ordinal instead of disappearing.
/// A new TY ends an unterminated occurrence; its fields never leak into the next.
pub fn parse(input: &str) -> Result<Source<'_>, InputError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(InputError::ByteLimit);
    }
    let mut records = Vec::new();
    let mut current: Option<Occurrence<'_>> = None;
    let mut field_count = 0;
    let mut offset = 0;
    let mut can_continue = false;
    for raw in input.split_inclusive('\n') {
        let end = offset + raw.len();
        let body = raw.strip_suffix('\n').unwrap_or(raw);
        let body = body.strip_suffix('\r').unwrap_or(body);
        let body = if offset == 0 {
            body.strip_prefix('\u{feff}').unwrap_or(body)
        } else {
            body
        };
        let parsed = tag(body);
        let starts = matches!(parsed, Some(("TY", _)));
        if starts {
            records.extend(current.take());
        }
        if current.is_none() && (parsed.is_some() || !body.trim_matches([' ', '\t']).is_empty()) {
            if records.len() >= MAX_RECORDS {
                return Err(InputError::RecordLimit);
            }
            current = Some(Occurrence {
                bytes: offset..end,
                fields: Vec::new(),
                has_type: starts,
                has_end: false,
                malformed_lines: 0,
            });
            can_continue = false;
        }
        if let Some(rec) = current.as_mut() {
            rec.bytes.end = end;
            if body.bytes().any(|b| b.is_ascii_control() && b != b'\t') {
                rec.malformed_lines += 1;
                can_continue = false;
                offset = end;
                continue;
            }
            match parsed {
                Some((key, value)) => {
                    field_count += 1;
                    if field_count > MAX_FIELDS {
                        return Err(InputError::FieldLimit);
                    }
                    if (key == "TY" && value.trim().is_empty())
                        || (key == "ER" && !value.trim().is_empty())
                    {
                        rec.malformed_lines += 1;
                    }
                    rec.fields.push(Field {
                        tag: key,
                        first_line_value: value,
                        bytes: offset..end,
                        continued: false,
                    });
                    can_continue = true;
                    if key == "ER" {
                        rec.has_end = true;
                        records.push(current.take().expect("active occurrence"));
                        can_continue = false;
                    }
                }
                None if body.trim_matches([' ', '\t']).is_empty() => {
                    if let Some(last) = rec.fields.last_mut().filter(|_| can_continue) {
                        last.bytes.end = end;
                    }
                }
                None if can_continue
                    && body.starts_with([' ', '\t'])
                    && tag(body.trim_start()).is_none() =>
                {
                    let field = rec.fields.last_mut().expect("active field");
                    field.continued = true;
                    field.bytes.end = end;
                    if field.tag == "TY" {
                        rec.malformed_lines += 1;
                    }
                }
                None => {
                    rec.malformed_lines += 1;
                    can_continue = false;
                }
            }
        }
        offset = end;
    }
    if let Some(prev) = current {
        records.push(prev);
    }
    Ok(Source {
        original: input,
        occurrences: records,
    })
}

enum Projected<'a> {
    Doi(&'a str),
    Pmid(&'a str),
}
fn after_ascii_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|part| part.eq_ignore_ascii_case(prefix))?;
    Some(&value[prefix.len()..])
}
fn projected<'a>(field: &Field<'a>) -> Option<Projected<'a>> {
    let v = field.first_line_value.trim();
    match field.tag {
        "DO" => Some(Projected::Doi(v)),
        "UR" => {
            for prefix in [
                "https://pubmed.ncbi.nlm.nih.gov/",
                "http://pubmed.ncbi.nlm.nih.gov/",
            ] {
                if let Some(path) = after_ascii_prefix(v, prefix) {
                    return Some(Projected::Pmid(path.strip_suffix('/').unwrap_or(path)));
                }
            }
            for prefix in [
                "https://doi.org/",
                "http://doi.org/",
                "https://dx.doi.org/",
                "http://dx.doi.org/",
            ] {
                if after_ascii_prefix(v, prefix).is_some() {
                    return Some(Projected::Doi(v));
                }
            }
            None
        }
        _ => None,
    }
}

#[derive(Default)]
struct Values {
    key: Option<String>,
    had_field: bool,
    unsupported: bool,
    conflict: bool,
}
impl Values {
    fn add(&mut self, key: Option<String>) {
        self.had_field = true;
        match key {
            None => self.unsupported = true,
            Some(value) => match self.key.as_ref() {
                None => self.key = Some(value),
                Some(prev) if prev != &value => self.conflict = true,
                _ => {}
            },
        }
    }
    fn take_key(&mut self) -> Option<String> {
        if self.unsupported || self.conflict {
            None
        } else {
            self.key.take()
        }
    }
}

impl Source<'_> {
    /// Skip ambiguous namespaces and withhold all evidence from structurally
    /// damaged occurrences. Every occurrence still enters reconciliation.
    pub fn review(&self) -> Review {
        let mut rows = Vec::with_capacity(self.occurrences.len());
        let mut issues = Vec::new();
        for (index, rec) in self.occurrences.iter().enumerate() {
            let mut row = Row::default();
            if rec.usable() {
                let mut d = Values::default();
                let mut p = Values::default();
                for field in &rec.fields {
                    match projected(field) {
                        Some(Projected::Doi(raw)) => {
                            d.add(if field.continued { None } else { doi_key(raw) })
                        }
                        Some(Projected::Pmid(raw)) => {
                            p.add(if field.continued { None } else { pmid_key(raw) })
                        }
                        None => {}
                    }
                }
                let ordinal = index + 1;
                if d.unsupported {
                    issues.push((ordinal, Issue::UnsupportedDoi));
                }
                if p.unsupported {
                    issues.push((ordinal, Issue::UnsupportedPmid));
                }
                if d.conflict {
                    issues.push((ordinal, Issue::ConflictingDoi));
                }
                if p.conflict {
                    issues.push((ordinal, Issue::ConflictingPmid));
                }
                if !d.had_field && !p.had_field {
                    issues.push((ordinal, Issue::NoIdentifiers));
                }
                row.doi_conflict = d.conflict;
                row.pmid_conflict = p.conflict;
                row.doi = d.take_key();
                row.pmid = p.take_key();
            }
            rows.push(row);
        }
        review_rows(&rows, issues)
    }
}
