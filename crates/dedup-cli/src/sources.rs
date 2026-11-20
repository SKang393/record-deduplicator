use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io::{self, Write},
};

use super::decisions::Decision;
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

enum Action<'a> {
    Review,
    Init(&'a OsStr),
    Resume(&'a OsStr),
    Export(&'a OsStr, &'a OsStr),
}

pub(super) fn run(args: &[OsString]) -> Result<(), String> {
    let (action, args) = match args {
        [flag, path, rest @ ..] if flag == "--init-decisions" => (Action::Init(path), rest),
        [flag, path, rest @ ..] if flag == "--resume-decisions" => (Action::Resume(path), rest),
        [flag, path, dir, rest @ ..] if flag == "--export-reviewed" => {
            (Action::Export(path, dir), rest)
        }
        _ => (Action::Review, args),
    };
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
    let manifest: Vec<_> = queue
        .sources
        .iter()
        .map(|named| {
            let original = named.source.original.as_bytes();
            let hash = Sha256::digest(original);
            format!(
                "input\tris\t{}\tbytes={};occurrences={};sha256={hash:x}",
                named.label,
                original.len(),
                named.source.occurrences.len()
            )
        })
        .collect();
    let decisions = match action {
        Action::Review => None,
        Action::Init(path) => {
            super::decisions::create(path, &queue, &manifest)?;
            None
        }
        Action::Resume(path) => Some(super::decisions::resume(path, &queue, &manifest)?),
        Action::Export(path, dir) => {
            let decisions = super::decisions::resume(path, &queue, &manifest)?;
            return super::reviewed::export(dir.as_ref(), &queue, &manifest, &decisions);
        }
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    render_queue(&queue, &manifest, decisions.as_deref(), &mut out)
        .and_then(|()| out.flush())
        .map_err(|_| String::from("Cannot write report."))
}

fn render_queue(
    queue: &ReviewQueue<'_>,
    manifest: &[String],
    decisions: Option<&[Decision]>,
    out: &mut impl Write,
) -> io::Result<()> {
    let reference = |global: usize| {
        let entry = &queue.entries[global - 1];
        format!("{}:{}", entry.label, entry.ordinal)
    };
    super::render(&queue.review, out, reference)?;
    for hint in queue.metadata_hints() {
        match hint {
            ris::MetadataHint::Inspect(rows) => {
                let refs = rows
                    .into_iter()
                    .map(reference)
                    .collect::<Vec<_>>()
                    .join(",");
                writeln!(
                    out,
                    "triage\ttitle_author_year_type\t{refs}\tinspect_metadata_only"
                )?;
            }
            ris::MetadataHint::Overflow { first, count } => {
                writeln!(
                    out,
                    "triage_overflow\ttitle_author_year_type\t{}\tmembers={count};limit={}",
                    reference(first),
                    ris::MAX_HINT_MEMBERS
                )?;
            }
        }
    }
    let mut entries = queue.entries.iter();
    for (named, identity) in queue.sources.iter().zip(manifest) {
        writeln!(out, "{identity}")?;
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
    if let Some(decisions) = decisions {
        for (entry, &decision) in queue.entries.iter().zip(decisions) {
            if decision != Decision::NotCandidate {
                writeln!(
                    out,
                    "decision\thuman\t{}:{}\t{}",
                    entry.label,
                    entry.ordinal,
                    decision.label(queue)
                )?;
            }
        }
    }
    Ok(())
}
