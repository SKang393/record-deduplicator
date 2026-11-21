//! Look up occurrences and display selected originals in a reopened review.
use std::{
    borrow::Cow,
    ffi::OsStr,
    io::{self, Write},
};

use dedup_core::{
    identifier_review::Identifier,
    ris::{CitationValue, Field, Occurrence, ReportIdentifiers, ReviewQueue},
};

use super::decisions::Decision;

fn sheet_status(decision: &Decision, queue: &ReviewQueue<'_>) -> Cow<'static, str> {
    match decision {
        Decision::NotCandidate => "outside-decision-sheet".into(),
        decision => decision.label(queue),
    }
}

fn single_line(text: &str) -> String {
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    line
}

// Compare the whole alternate spelling only when a single comma gives the split.
fn given_first(name: &str) -> Option<String> {
    let (family, given) = name.split_once(',')?;
    let (family, given) = (family.trim(), given.trim());
    if family.is_empty() || given.is_empty() || given.contains(',') {
        return None;
    }
    Some(format!("{given} {family}"))
}

pub(super) fn find(queue: &ReviewQueue<'_>, decisions: &[Decision], query: &str) -> io::Result<()> {
    let query = single_line(query).to_lowercase();
    let (prefix, rest) = query.split_once(' ').unwrap_or((&query, ""));
    let by_venue = prefix == "--venue";
    let (needle, required_title) = if by_venue || prefix == "--author" {
        let Some((fragment, title)) = rest
            .split_once('|')
            .map(|(field, title)| (field.trim(), title.trim()))
            .filter(|(field, title)| !field.is_empty() && !title.is_empty())
        else {
            let field = if by_venue { "VENUE" } else { "NAME" };
            eprintln!("Enter :find {prefix} {field} | TITLE with text on both sides of the bar.");
            return Ok(());
        };
        (fragment, Some(title))
    } else {
        (query.as_str(), None)
    };
    let field_tags: &[&str] = if by_venue {
        eprintln!(
            "Venue lookup uses JO, JF or T2. Use :find TITLE to include occurrences with missing or differently written venues."
        );
        &["JO", "JF", "T2"]
    } else {
        &["AU", "A1"]
    };
    let reversed_query = given_first(needle);
    let query_name = reversed_query.as_deref().unwrap_or(needle);
    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut count = 0;
    let mut decisions = decisions.iter();
    for named in &queue.sources {
        let field_text = |field: &Field<'_>| {
            // The parsed span includes continuation lines and starts with the tagged header.
            let tagged = &named.source.original[field.bytes.clone()];
            single_line(tagged.split_once('-').expect("parsed field header").1)
        };
        for (index, (record, decision)) in named
            .source
            .occurrences
            .iter()
            .zip(&mut decisions)
            .enumerate()
        {
            let titles = || {
                record
                    .fields
                    .iter()
                    .filter(|f| matches!(f.tag, "TI" | "T1"))
            };
            let title_hit = titles().map(field_text).find(|value| {
                value
                    .to_lowercase()
                    .contains(required_title.unwrap_or(needle))
            });
            if required_title.is_some() && title_hit.is_none() {
                continue;
            }
            let field_hit = record
                .fields
                .iter()
                .filter(|f| field_tags.contains(&f.tag))
                .find_map(|f| {
                    let original = field_text(f);
                    let value = original.to_lowercase();
                    (value.contains(needle)
                        || (!by_venue
                            && given_first(&value).as_deref().unwrap_or(&value) == query_name))
                        .then_some((f.tag, original))
                });
            if field_hit.is_none() && (required_title.is_some() || title_hit.is_none()) {
                continue;
            }
            let mut title = title_hit
                .or_else(|| titles().next().map(field_text))
                .unwrap_or_else(|| "(title absent)".into());
            if let Some((end, _)) = title.char_indices().nth(120) {
                title.truncate(end);
                title.push('…');
            }
            write!(
                out,
                "{}:{}\t{}\t{title:?}",
                named.label,
                index + 1,
                sheet_status(decision, queue)
            )?;
            if let Some((tag, original)) = field_hit {
                write!(out, "\t{tag}={original:?}")?;
            }
            writeln!(out)?;
            count += 1;
        }
    }
    if count == 0 {
        writeln!(
            out,
            "No occurrences matched. Try a shorter title or author fragment."
        )?;
    } else if count == 1 {
        writeln!(
            out,
            "Found 1. If you expected another occurrence, try :find WORD with one distinctive word from the title, without --author or --venue. Only existing sheet rows are editable."
        )?;
    } else {
        writeln!(
            out,
            "Found {count}. Use :inspect KEY,KEY to compare two originals. Only existing sheet rows are editable."
        )?;
    }
    out.flush()
}

fn display_value(value: &CitationValue) -> String {
    let values = match value {
        CitationValue::Absent => return "absent".into(),
        CitationValue::Unresolved => return "see original".into(),
        CitationValue::Conflicting => return "conflicting values".into(),
        CitationValue::Known(values) | CitationValue::AsSupplied(values) => values.as_slice(),
        CitationValue::IdentifierWithUnsupportedField(key) => std::slice::from_ref(key),
    };
    let mut text = values
        .iter()
        .map(|v| format!("{v:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    if let Some((end, _)) = text.char_indices().nth(72) {
        text.truncate(end);
        text.push('…');
    }
    if matches!(value, CitationValue::IdentifierWithUnsupportedField(_)) {
        text.push_str(" (additional unsupported field)");
    }
    text
}

// Use the full field span so a difference on a continuation line stays visible.
fn abstract_text<'a>(record: &Occurrence<'_>, raw: &'a str) -> Result<&'a str, &'static str> {
    let mut fields = record
        .fields
        .iter()
        .filter(|f| matches!(f.tag, "AB" | "N2"));
    let field = fields.next().ok_or("absent")?;
    if fields.next().is_some() {
        return Err("multiple fields (see original)");
    }
    let relative = field.bytes.start - record.bytes.start..field.bytes.end - record.bytes.start;
    let (_, value) = raw[relative].split_once('-').expect("parsed field header");
    let value = value.trim();
    if value.is_empty() {
        return Err("empty");
    }
    Ok(value)
}

fn display_word(word: Option<&str>) -> String {
    match word {
        None => "(end of abstract)".into(),
        Some(word) => match word.char_indices().nth(40) {
            Some((end, _)) => format!("{:?}…", &word[..end]),
            None => format!("{word:?}"),
        },
    }
}

fn compare_abstracts(
    out: &mut impl Write,
    left: Result<&str, &str>,
    right: Result<&str, &str>,
) -> io::Result<()> {
    if left == Err("absent") && right == Err("absent") {
        return Ok(());
    }
    let label = "  Abstract text (AB/N2)";
    let (Ok(left_text), Ok(right_text)) = (left, right) else {
        return writeln!(
            out,
            "{label}  {} | {}",
            left.err().unwrap_or("present"),
            right.err().unwrap_or("present")
        );
    };
    let mut left_words = left_text.split_whitespace();
    let mut right_words = right_text.split_whitespace();
    let mut position = 1;
    loop {
        match (left_words.next(), right_words.next()) {
            (None, None) => return writeln!(out, "{label}  same wording (spacing ignored)"),
            (left_word, right_word) if left_word != right_word => {
                return writeln!(
                    out,
                    "{label}  first difference at word {position}  {} | {}",
                    display_word(left_word),
                    display_word(right_word)
                );
            }
            _ => position += 1,
        }
    }
}

pub(super) fn pair(
    queue: &ReviewQueue<'_>,
    decisions: &[Decision],
    left: &OsStr,
    right: &OsStr,
) -> io::Result<()> {
    let locate = |key: &OsStr| {
        let syntax = "Expected an occurrence key from the review, for example catalog:2.";
        let (label, number) = key.to_str().and_then(|s| s.split_once(':')).ok_or(syntax)?;
        let ordinal = number.parse::<usize>().map_err(|_| syntax)?;
        if ordinal == 0 || number != ordinal.to_string() {
            return Err(syntax.to_owned());
        }
        let mut offset = 0;
        for named in &queue.sources {
            if named.label == label {
                if let Some(rec) = named.source.occurrences.get(ordinal - 1) {
                    let raw = &named.source.original[rec.bytes.clone()];
                    return Ok((offset + ordinal - 1, rec, raw));
                }
                break;
            }
            offset += named.source.occurrences.len();
        }
        Err(format!(
            "Unknown occurrence {key:?}. Use a label and ordinal from the review."
        ))
    };
    let invalid_key = |message| io::Error::new(io::ErrorKind::InvalidInput, message);
    let first = locate(left).map_err(invalid_key)?;
    let second = locate(right).map_err(invalid_key)?;
    if first.0 == second.0 {
        return Err(invalid_key(
            "Select two different occurrences to inspect.".into(),
        ));
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    let write_error = |_: io::Error| io::Error::other("Cannot write comparison.");
    writeln!(
        out,
        "Citation fields (normalized, {} | {})",
        left.to_string_lossy(),
        right.to_string_lossy()
    )
    .map_err(write_error)?;
    for ((name, a), (_, b)) in first
        .1
        .citation_fields()
        .into_iter()
        .zip(second.1.citation_fields())
    {
        let line = match (&a, &b) {
            (CitationValue::Absent, CitationValue::Absent) => continue,
            (CitationValue::Known(_), CitationValue::Known(_)) if a == b => {
                format!("{name} (same)  {}", display_value(&a))
            }
            (CitationValue::Known(_), CitationValue::Known(_)) => format!(
                "{name} (different)  {} | {}",
                display_value(&a),
                display_value(&b)
            ),
            _ => format!("{name}  {} | {}", display_value(&a), display_value(&b)),
        };
        writeln!(out, "  {line}").map_err(write_error)?;
    }
    compare_abstracts(
        &mut out,
        abstract_text(first.1, first.2),
        abstract_text(second.1, second.2),
    )
    .map_err(write_error)?;
    let mut ids = ReportIdentifiers::default();
    ids.observe(first.1);
    ids.observe(second.1);
    if let Some(conflict) = ids.conflict() {
        let kind = match conflict {
            Identifier::Doi => "DOI",
            Identifier::Pmid => "PMID",
        };
        writeln!(out, "Conflicting supported {kind} values prevent combining these two occurrences in a reviewed export.").map_err(write_error)?;
    }
    for (index, rec, raw) in [first, second] {
        let entry = &queue.entries[index];
        let status = sheet_status(&decisions[index], queue);
        writeln!(
            out,
            "[{}:{}] {} (bytes {}..{})",
            entry.label, entry.ordinal, status, rec.bytes.start, rec.bytes.end
        )
        .map_err(write_error)?;
        if rec.final_pagination().is_none() {
            let detail = if rec.fields.iter().any(|f| matches!(f.tag, "SP" | "EP")) {
                "supplies pagination fields without a usable final page range or article number"
            } else {
                "has no SP or EP field for a final page range or article number"
            };
            writeln!(out, "{}:{} {detail}.", entry.label, entry.ordinal).map_err(write_error)?;
        }
        // Keep source lines separate from headings, including malformed lines.
        let lines = raw
            .strip_prefix('\u{feff}')
            .unwrap_or(raw)
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        for line in lines.split_terminator('\n') {
            writeln!(out, "  {}", line.escape_debug()).map_err(write_error)?;
        }
    }
    out.flush().map_err(write_error)
}
