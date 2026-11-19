use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io::{self, Write},
};

use dedup_core::{
    identifier_review as ids,
    ris::{self, Disposition, QueueError, ReviewQueue},
};
use sha2::{Digest, Sha256};

const USAGE: &str = "Expected --ris-source <label> <file> for each source. See dedup --help.";

fn queue_error(error: QueueError) -> &'static str {
    match error {
        QueueError::SourceCount => "Supply between 1 and 128 RIS sources.",
        QueueError::InvalidLabel => {
            "Source labels must be 1 to 64 ASCII letters, digits, dots, underscores or hyphens, starting with a letter or digit."
        }
        QueueError::DuplicateLabel => "Each RIS source needs a unique label.",
        QueueError::Input(error) => super::input_error(error),
    }
}

fn named_paths(args: &[OsString]) -> Result<BTreeMap<&str, &OsStr>, &'static str> {
    let sources = args.chunks_exact(3);
    if !sources.remainder().is_empty() {
        return Err(USAGE);
    }
    if sources.len() == 0 || sources.len() > ris::MAX_SOURCES {
        return Err(queue_error(QueueError::SourceCount));
    }
    let mut paths = BTreeMap::new();
    for source in sources {
        if source[0] != "--ris-source" {
            return Err(USAGE);
        }
        let label = source[1]
            .to_str()
            .filter(|s| ris::valid_source_label(s))
            .ok_or(queue_error(QueueError::InvalidLabel))?;
        if source[2] == "-" {
            return Err("Named RIS sources require regular files.");
        }
        if paths.insert(label, source[2].as_os_str()).is_some() {
            return Err(queue_error(QueueError::DuplicateLabel));
        }
    }
    Ok(paths)
}

pub(super) fn run(args: &[OsString]) -> Result<(), String> {
    let paths = named_paths(args)?;
    let mut remaining = ids::MAX_INPUT_BYTES;
    let mut snapshots = Vec::with_capacity(paths.len());
    for (label, path) in paths {
        let text = super::read_input(path, true, remaining)
            .map_err(|error| format!("{label}: {error}"))?;
        remaining -= text.len();
        snapshots.push((label, path, text));
    }
    let inputs: Vec<_> = snapshots
        .iter()
        .map(|(label, _, text)| (*label, text.as_str()))
        .collect();
    let queue = ris::review_sources(&inputs).map_err(queue_error)?;
    // Validate every captured file before emitting any part of the review.
    for (label, path, text) in &snapshots {
        super::recheck(path, text.as_bytes()).map_err(|error| format!("{label}: {error}"))?;
    }
    let mut out = io::BufWriter::new(io::stdout().lock());
    render_queue(&queue, &mut out)
        .and_then(|()| out.flush())
        .map_err(|_| String::from("Cannot write report."))
}

fn render_queue(queue: &ReviewQueue<'_>, out: &mut impl Write) -> io::Result<()> {
    super::render(&queue.review, out, |global| {
        let entry = &queue.entries[global - 1];
        format!("{}:{}", entry.label, entry.ordinal)
    })?;
    let mut entries = queue.entries.iter();
    for named in &queue.sources {
        let original = named.source.original.as_bytes();
        let hash = Sha256::digest(original);
        writeln!(
            out,
            "input\tris\t{}\tbytes={};occurrences={};sha256={hash:x}",
            named.label,
            original.len(),
            named.source.occurrences.len()
        )?;
        for rec in &named.source.occurrences {
            let entry = entries.next().expect("one queue entry per occurrence");
            let disposition = match entry.disposition {
                Disposition::ReviewCandidate => "review_candidate",
                Disposition::ReviewInput => "review_input",
                Disposition::Retained => "retained",
            };
            super::render_occurrence(
                out,
                format_args!("{}:{}", entry.label, entry.ordinal),
                rec,
                Some(disposition),
            )?;
        }
    }
    Ok(())
}
