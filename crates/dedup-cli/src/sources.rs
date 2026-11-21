use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io::{self, BufRead, Write},
    path::Path,
};

use super::decisions::{Decision, ResumedSheet};
use super::start::Inspection;
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

pub(super) enum PendingMode<'a> {
    List,
    CreateAndReview(&'a mut dyn BufRead),
    Reopen(&'a mut dyn BufRead),
}

pub(super) enum Action<'a> {
    Review,
    Coverage(super::coverage::Plan<'a>),
    Init {
        path: &'a OsStr,
        metadata: bool,
        guided: bool,
    },
    Resume(&'a OsStr),
    Pending(&'a OsStr, PendingMode<'a>),
    EarlierChoices {
        current: &'a OsStr,
        earlier: &'a OsStr,
    },
    InspectPair(&'a OsStr, &'a OsStr, &'a OsStr),
    Export {
        sheet: &'a OsStr,
        directory: &'a Path,
        include_asreview: bool,
    },
}

pub(super) fn run(args: &[OsString]) -> Result<(), String> {
    let (action, args) = match args {
        [flag, rest @ ..] if flag == "--check-coverage" => {
            let (plan, sources) = super::coverage::Plan::parse(rest)?;
            if sources.is_empty() {
                return plan.check(&[], &[], &BTreeMap::new());
            }
            (Action::Coverage(plan), sources)
        }
        [flag, path, rest @ ..]
            if flag == "--init-decisions" || flag == "--init-metadata-decisions" =>
        {
            (
                Action::Init {
                    path,
                    metadata: flag == "--init-metadata-decisions",
                    guided: false,
                },
                rest,
            )
        }
        [flag, path, rest @ ..] if flag == "--resume-decisions" => (Action::Resume(path), rest),
        [flag, current, from, earlier, rest @ ..]
            if flag == "--pending-decisions" && from == "--earlier-sheet" =>
        {
            (Action::EarlierChoices { current, earlier }, rest)
        }
        [flag, path, rest @ ..] if flag == "--pending-decisions" => {
            (Action::Pending(path, PendingMode::List), rest)
        }
        [flag, path, left, right, rest @ ..] if flag == "--inspect-pair" => {
            (Action::InspectPair(path, left, right), rest)
        }
        [flag, path, dir, rest @ ..] if flag == "--export-reviewed" => {
            let (include_asreview, sources) = match rest {
                [option, sources @ ..] if option == "--asreview-ris" => (true, sources),
                sources => (false, sources),
            };
            (
                Action::Export {
                    sheet: path,
                    directory: dir.as_ref(),
                    include_asreview,
                },
                sources,
            )
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
    let guided_review = matches!(
        action,
        Action::Init { guided: true, .. }
            | Action::Pending(_, PendingMode::CreateAndReview(_) | PendingMode::Reopen(_))
    );
    let mut labels_by_fingerprint = BTreeMap::new();
    for named in &queue.sources {
        let records = &named.source.occurrences;
        if let Some((index, rec)) = records.iter().enumerate().find(|(_, rec)| !rec.usable()) {
            let location = format!(
                "Invalid RIS at {}:{} (bytes {}..{}, {}).",
                named.label,
                index + 1,
                rec.bytes.start,
                rec.bytes.end,
                super::occurrence_status(rec)
            );
            if let Some(byte) = rec.embedded_header_byte {
                return Err(format!(
                    "{location} A UTF-8 BOM precedes a TY header inside the file at byte {byte}. \
                     Check for an interrupted batch and supply each complete export as a separately named source."
                ));
            }
            return Err(format!(
                "{location} Expected complete TY/ER records. Inspect with `dedup review-ids --ris <file>`. \
                 For BibTeX, see the README section \"Bring a BibTeX export into a named RIS review\"."
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
        if matches!(action, Action::Coverage(_)) || (guided_review && !records.is_empty()) {
            labels_by_fingerprint
                .entry(hash.into())
                .or_insert_with(Vec::new)
                .push(named.label);
        }
    }
    let show_repeated_downloads = || {
        for labels in labels_by_fingerprint
            .values()
            .filter(|labels| labels.len() > 1)
        {
            eprintln!(
                "Source labels {} have the same whole-file SHA-256. Each label contributes its own occurrences. \
                 Check your search log. If a download was selected by mistake, start a new sheet with the intended sources.",
                labels.join(", ")
            );
        }
    };
    let mut guided_pending_rows = None;
    let sheet = match action {
        Action::Review => None,
        Action::Coverage(plan) => {
            return plan.check(&queue.sources, &manifest, &labels_by_fingerprint);
        }
        Action::Init {
            path,
            metadata,
            guided,
        } => {
            let count = super::decisions::create(path, &queue, &manifest, metadata)?;
            guided_pending_rows = guided.then_some(count);
            None
        }
        Action::Resume(path) => Some(super::decisions::resume(path, &queue, &manifest)?),
        Action::EarlierChoices { current, earlier } => {
            let sheet = super::decisions::resume(current, &queue, &manifest)
                .map_err(|e| format!("Current sheet: {e}"))?;
            let earlier_text =
                super::decisions::read_sheet(earlier).map_err(|e| format!("Earlier sheet: {e}"))?;
            let labels = super::decisions::saved_source_labels(&earlier_text);
            let (inputs, identities): (Vec<_>, Vec<_>) = queue
                .sources
                .iter()
                .zip(&manifest)
                .filter(|(source, _)| labels.contains(source.label))
                .map(|(source, identity)| {
                    ((source.label, source.source.original), identity.clone())
                })
                .unzip();
            if inputs.is_empty() {
                return Err("Cannot locate any earlier named inputs. Check the earlier sheet and select its original downloads along with the update.".into());
            }
            let earlier_queue = ris::review_sources(&inputs).map_err(queue_error)?;
            let earlier_sheet =
                super::decisions::parse_sheet(&earlier_text, &earlier_queue, &identities)
                    .map_err(|e| format!("Earlier sheet: {e}"))?;
            recheck_sources()?;
            let mut out = io::BufWriter::new(io::stdout().lock());
            return super::pending::earlier_choices(
                &queue,
                &sheet,
                &earlier_queue,
                &earlier_sheet,
                &mut out,
            )
            .and_then(|()| out.flush())
            .map_err(|_| "Cannot write earlier choices.".into());
        }
        Action::InspectPair(path, left, right) => {
            let sheet = super::decisions::resume(path, &queue, &manifest)?;
            return super::inspect::pair(&queue, &sheet.decisions, left, right)
                .map_err(|e| e.to_string());
        }
        Action::Export {
            sheet,
            directory,
            include_asreview,
        } => {
            let sheet = super::decisions::resume(sheet, &queue, &manifest)?;
            return super::reviewed::prepare(
                &queue,
                &manifest,
                &sheet.decisions,
                include_asreview,
            )?
            .publish(directory);
        }
        Action::Pending(path, mode) => {
            let guide = match mode {
                PendingMode::List => None,
                PendingMode::Reopen(input) => Some(input),
                PendingMode::CreateAndReview(input) => {
                    let count = super::decisions::create(path, &queue, &manifest, true)?;
                    eprintln!(
                        "Created {} with {count} pending rows. You can edit the sheet while this guide is open. Type :pending after saving to list the current groups.",
                        Path::new(path).display()
                    );
                    Some(input)
                }
            };
            let parse = |text: &str| super::decisions::parse_sheet(text, &queue, &manifest);
            let text = super::decisions::read_sheet(path)?;
            let mut sheet = parse(&text)?;
            let mut validated_text = Some(text);
            let guided = guide.is_some();
            let show_pending = |sheet: &ResumedSheet| -> Result<(), String> {
                let mut out = io::BufWriter::new(io::stdout().lock());
                let first_group = super::pending::render(&queue, &manifest, sheet, &mut out)
                    .and_then(|first| out.flush().map(|()| first))
                    .map_err(|_| "Cannot write report.")?;
                if let Some(row) = first_group.filter(|_| guided) {
                    eprintln!("First unresolved group from the pending list:\n  {row}");
                }
                Ok(())
            };
            show_pending(&sheet)?;
            if let Some(input) = guide {
                show_repeated_downloads();
                let read_current_sheet = || {
                    recheck_sources()?;
                    super::decisions::read_sheet(path)
                };
                let Some(directory) = super::start::export_destination(input, |request| {
                    let current = read_current_sheet()?;
                    if validated_text.as_deref() != Some(current.as_str()) {
                        sheet = match parse(&current) {
                            Ok(updated) => updated,
                            Err(error) => {
                                validated_text = None;
                                eprintln!(
                                    "{error}\nResolve the sheet error, then retry this command or press Enter to close."
                                );
                                return Ok(());
                            }
                        };
                        validated_text = Some(current);
                        eprintln!("Reloaded the saved decision sheet.");
                    }
                    match request {
                        Inspection::Pending => show_pending(&sheet),
                        Inspection::Find(query) => {
                            super::inspect::find(&queue, &sheet.decisions, query)
                                .map_err(|e| e.to_string())
                        }
                        Inspection::ListLeads => {
                            let pairs = queue.locator_pairs();
                            let count = pairs.title_variants.len() + pairs.complementary_ids.len();
                            eprintln!(
                                "Inspection-only pairs: {count}. Use :inspect with the two keys from a row. These pairs add no decision rows. Both records stay in the export unless a saved decision removes one. The guide keeps no history of viewed pairs."
                            );
                            let mut out = io::BufWriter::new(io::stdout().lock());
                            write_inspection_leads(&queue, pairs, &mut out)
                                .and_then(|()| out.flush())
                                .map_err(|_| "Cannot write inspection leads.".into())
                        }
                        Inspection::Pair(left, right) => match super::inspect::pair(
                            &queue,
                            &sheet.decisions,
                            left.as_ref(),
                            right.as_ref(),
                        ) {
                            Err(e) if e.kind() == io::ErrorKind::InvalidInput => {
                                eprintln!("{e}");
                                Ok(())
                            }
                            result => result.map_err(|e| e.to_string()),
                        },
                    }
                })?
                else {
                    return Ok(());
                };
                let mut decisions = parse(&read_current_sheet()?)?.decisions;
                if decisions != sheet.decisions {
                    eprintln!("Saved decisions changed. Previewing the current sheet.");
                }
                loop {
                    let export = super::reviewed::prepare(&queue, &manifest, &decisions, false)?;
                    if !super::start::confirm_export(
                        input,
                        path.as_ref(),
                        &directory,
                        &export,
                        &queue,
                        &decisions,
                    )? {
                        break;
                    }
                    let approved_sheet = read_current_sheet()?;
                    let current = parse(&approved_sheet)?.decisions;
                    if current != decisions {
                        eprintln!(
                            "Saved decisions changed after the preview. Review the updated preview and confirm again."
                        );
                        decisions = current;
                        continue;
                    }
                    let published = export.publish_if(&directory, || {
                        let latest = read_current_sheet()?;
                        if latest == approved_sheet {
                            return Ok(true);
                        }
                        decisions = parse(&latest)?.decisions;
                        Ok(false)
                    })?;
                    if !published {
                        eprintln!(
                            "The saved sheet changed while writing the temporary files. Check the saved choices and confirm again, even if the totals match."
                        );
                        continue;
                    }
                    eprintln!(
                        "Saved {} with mapping.tsv and reconciliation.tsv.",
                        directory.join("reviewed.ris").display()
                    );
                    break;
                }
            }
            return Ok(());
        }
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    render_queue(
        &queue,
        &manifest,
        sheet,
        guided_pending_rows.is_some(),
        &mut out,
    )
    .and_then(|()| out.flush())
    .map_err(|_| String::from("Cannot write report."))?;
    if let Some(count) = guided_pending_rows {
        show_repeated_downloads();
        eprintln!(
            "Pending rows in your new decision sheet: {count}. The queue's candidate_records and groups count only shared DOI or PMID matches. Metadata comparisons can add editable rows."
        );
    }
    Ok(())
}

fn write_inspection_leads(
    queue: &ReviewQueue<'_>,
    pairs: ris::LocatorPairs,
    out: &mut impl Write,
) -> io::Result<()> {
    for (signal, leads) in [
        ("title_variant_locator", pairs.title_variants),
        ("complementary_id_locator", pairs.complementary_ids),
    ] {
        for rows in leads {
            let [a, b] = rows.map(|row| &queue.entries[row - 1]);
            writeln!(
                out,
                "triage\t{signal}\t{}:{},{}:{}\tinspect_only",
                a.label, a.ordinal, b.label, b.ordinal
            )?;
        }
    }
    Ok(())
}

fn render_queue(
    queue: &ReviewQueue<'_>,
    manifest: &[String],
    sheet: Option<ResumedSheet>,
    guided: bool,
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
    if guided && !pairs.complementary_ids.is_empty() {
        eprintln!(
            "Look for complementary_id_locator to see optional comparisons ({} total). The citation details match, with a DOI in one source and a PMID in the other. Reopen the guide and type :leads to find these keys again. These leads add no removal decisions.",
            pairs.complementary_ids.len()
        );
    }
    write_inspection_leads(queue, pairs, out)?;
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
