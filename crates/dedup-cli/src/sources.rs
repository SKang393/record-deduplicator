use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io::{self, BufRead, Write},
    path::Path,
};

use super::decisions::{Decision, ResumedSheet};
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

pub(super) enum Action<'a> {
    Review,
    Init { path: &'a OsStr, metadata: bool },
    Resume(&'a OsStr),
    Pending(&'a OsStr, Option<&'a mut dyn BufRead>),
    InspectPair(&'a OsStr, &'a OsStr, &'a OsStr),
    Export(&'a OsStr, &'a Path),
}

pub(super) fn run(args: &[OsString]) -> Result<(), String> {
    let (action, args) = match args {
        [flag, path, rest @ ..]
            if flag == "--init-decisions" || flag == "--init-metadata-decisions" =>
        {
            (
                Action::Init {
                    path,
                    metadata: flag == "--init-metadata-decisions",
                },
                rest,
            )
        }
        [flag, path, rest @ ..] if flag == "--resume-decisions" => (Action::Resume(path), rest),
        [flag, path, rest @ ..] if flag == "--pending-decisions" => {
            (Action::Pending(path, None), rest)
        }
        [flag, path, left, right, rest @ ..] if flag == "--inspect-pair" => {
            (Action::InspectPair(path, left, right), rest)
        }
        [flag, path, dir, rest @ ..] if flag == "--export-reviewed" => {
            (Action::Export(path, dir.as_ref()), rest)
        }
        _ => (Action::Review, args),
    };
    run_action(action, args)
}

pub(super) fn run_action(action: Action<'_>, args: &[OsString]) -> Result<(), String> {
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
    let recheck_sources = || {
        snapshots.iter().try_for_each(|(label, path, text)| {
            super::recheck(path, text.as_bytes()).map_err(|error| format!("{label}: {error}"))
        })
    };
    recheck_sources()?;
    let mut manifest = Vec::with_capacity(queue.sources.len());
    for named in &queue.sources {
        let records = &named.source.occurrences;
        if let Some((index, rec)) = records.iter().enumerate().find(|(_, rec)| !rec.usable()) {
            return Err(format!(
                "Invalid RIS at {}:{} (bytes {}..{}, {}). Expected complete TY/ER records. \
                 Inspect with `dedup review-ids --ris <file>`. For BibTeX, see the README section \
                 \"Bring a BibTeX export into a named RIS review\".",
                named.label,
                index + 1,
                rec.bytes.start,
                rec.bytes.end,
                super::occurrence_status(rec)
            ));
        }
        let original = named.source.original.as_bytes();
        let hash = Sha256::digest(original);
        manifest.push(format!(
            "input\tris\t{}\tbytes={};occurrences={};sha256={hash:x}",
            named.label,
            original.len(),
            records.len()
        ));
    }
    let sheet = match action {
        Action::Review => None,
        Action::Init { path, metadata } => {
            super::decisions::create(path, &queue, &manifest, metadata)?;
            None
        }
        Action::Resume(path) => Some(super::decisions::resume(path, &queue, &manifest)?),
        Action::InspectPair(path, left, right) => {
            let sheet = super::decisions::resume(path, &queue, &manifest)?;
            return super::inspect::pair(&queue, &sheet.decisions, left, right);
        }
        Action::Export(path, directory) => {
            let sheet = super::decisions::resume(path, &queue, &manifest)?;
            return super::reviewed::prepare(&queue, &manifest, &sheet.decisions)?
                .publish(directory);
        }
        Action::Pending(path, guide) => {
            let sheet = super::decisions::resume(path, &queue, &manifest)?;
            {
                let mut out = io::BufWriter::new(io::stdout().lock());
                super::pending::render(&queue, &manifest, &sheet, &mut out)
                    .and_then(|()| out.flush())
                    .map_err(|_| "Cannot write report.")?;
            }
            if let Some(input) = guide {
                let Some(directory) = super::start::export_destination(input)? else {
                    return Ok(());
                };
                super::reviewed::require_new_destination(&directory).map_err(|e| e.to_string())?;
                let export = super::reviewed::prepare(&queue, &manifest, &sheet.decisions)?;
                if super::start::confirm_export(input, path.as_ref(), &directory, &export)? {
                    recheck_sources()?;
                    if super::decisions::resume(path, &queue, &manifest)?.decisions
                        != sheet.decisions
                    {
                        return Err("Decisions changed during confirmation. Reopen the sheet to preview again.".into());
                    }
                    export.publish(&directory)?;
                    eprintln!(
                        "Saved {} with mapping.tsv and reconciliation.tsv.",
                        directory.join("reviewed.ris").display()
                    );
                }
            }
            return Ok(());
        }
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    render_queue(&queue, &manifest, sheet, &mut out)
        .and_then(|()| out.flush())
        .map_err(|_| String::from("Cannot write report."))
}

fn render_queue(
    queue: &ReviewQueue<'_>,
    manifest: &[String],
    sheet: Option<ResumedSheet>,
    out: &mut impl Write,
) -> io::Result<()> {
    let reference = |global: usize| {
        let entry = &queue.entries[global - 1];
        format!("{}:{}", entry.label, entry.ordinal)
    };
    super::render(&queue.review, out, reference)?;
    let (decisions, hints) = match sheet {
        Some(sheet) => (Some(sheet.decisions), sheet.metadata_hints),
        None => (None, None),
    };
    for hint in hints.unwrap_or_else(|| queue.metadata_hints()) {
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
    let pairs = queue.locator_pairs();
    for (signal, leads) in [
        ("title_variant_locator", pairs.title_variants),
        ("complementary_id_locator", pairs.complementary_ids),
    ] {
        for [left, right] in leads {
            writeln!(
                out,
                "triage\t{signal}\t{},{}\tinspect_only",
                reference(left),
                reference(right)
            )?;
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
    if let Some(ref decisions) = decisions {
        for (entry, decision) in queue.entries.iter().zip(decisions) {
            if *decision != Decision::NotCandidate {
                writeln!(
                    out,
                    "decision\thuman\t{}:{}\t{}",
                    entry.label,
                    entry.ordinal,
                    decision.label(queue)
                )?;
                if let Decision::VerifiedMetadata { rationale, .. } = decision {
                    writeln!(
                        out,
                        "rationale\tmetadata\t{}:{}\t{rationale}",
                        entry.label, entry.ordinal
                    )?;
                }
            }
        }
    }
    Ok(())
}
