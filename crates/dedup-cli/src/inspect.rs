//! Display two selected source records after reopening their decision sheet.
use std::{
    ffi::OsStr,
    io::{self, Write},
};

use dedup_core::ris::ReviewQueue;

use super::decisions::Decision;

pub(super) fn pair(
    queue: &ReviewQueue<'_>,
    decisions: &[Decision],
    left: &OsStr,
    right: &OsStr,
) -> Result<(), String> {
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
    let first = locate(left)?;
    let second = locate(right)?;
    if first.0 == second.0 {
        return Err("Select two different occurrences to inspect.".into());
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    let write_error = |_: io::Error| "Cannot write comparison.";
    for (index, rec, raw) in [first, second] {
        let entry = &queue.entries[index];
        let status = match &decisions[index] {
            Decision::NotCandidate => "outside-decision-sheet".into(),
            decision => decision.label(queue),
        };
        writeln!(
            out,
            "[{}:{}] {} (bytes {}..{})",
            entry.label, entry.ordinal, status, rec.bytes.start, rec.bytes.end
        )
        .map_err(write_error)?;
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
    out.flush().map_err(|_| "Cannot write comparison.".into())
}
