//! Look up occurrences and display selected originals in a reopened review.
use std::{
    borrow::Cow,
    ffi::OsStr,
    io::{self, Write},
};

use dedup_core::{
    identifier_review::Identifier,
    ris::{CitationValue, Field, ReportIdentifiers, ReviewQueue},
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

pub(super) fn find(queue: &ReviewQueue<'_>, decisions: &[Decision], query: &str) -> io::Result<()> {
    let needle = single_line(query).to_lowercase();
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
            let hit = record
                .fields
                .iter()
                .filter(|f| matches!(f.tag, "TI" | "T1" | "AU" | "A1"))
                .find(|f| field_text(f).to_lowercase().contains(&needle));
            let Some(hit) = hit else { continue };
            let title = if matches!(hit.tag, "TI" | "T1") {
                Some(hit)
            } else {
                record.fields.iter().find(|f| matches!(f.tag, "TI" | "T1"))
            };
            let mut title = title
                .map(field_text)
                .unwrap_or_else(|| "(title absent)".into());
            if let Some((end, _)) = title.char_indices().nth(120) {
                title.truncate(end);
                title.push('…');
            }
            writeln!(
                out,
                "{}:{}\t{}\t{title:?}",
                named.label,
                index + 1,
                sheet_status(decision, queue)
            )?;
            count += 1;
        }
    }
    if count == 0 {
        writeln!(
            out,
            "No occurrences matched. Try a shorter title or author fragment."
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
