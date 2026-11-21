//! Prompt for the arguments to an ordinary, local named-RIS review.
use super::{
    decisions::Decision,
    pending::Groups,
    reviewed::PreparedExport,
    sources::{self, Action, PendingMode},
};
use dedup_core::ris;
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, ErrorKind, Write},
    path::{Path, PathBuf},
};

fn ask(input: &mut dyn BufRead, prompt: &str) -> Result<Option<String>, String> {
    eprint!("{prompt} ");
    io::stderr()
        .flush()
        .map_err(|_| "Cannot display the question.")?;
    let mut line = String::new();
    let count = input
        .read_line(&mut line)
        .map_err(|_| "Cannot read an answer.")?;
    let answer = line.trim_end_matches(['\r', '\n']);
    if count == 0 {
        Ok(None)
    } else if answer == ":cancel" {
        eprintln!("Review setup cancelled.");
        Ok(None)
    } else {
        Ok(Some(answer.to_owned()))
    }
}

fn ask_path(input: &mut dyn BufRead, prompt: &str) -> Result<Option<String>, String> {
    ask_path_or_action(input, prompt, |_| Ok(false))
}

fn ask_path_or_action(
    input: &mut dyn BufRead,
    prompt: &str,
    mut action: impl FnMut(&str) -> Result<bool, String>,
) -> Result<Option<String>, String> {
    while let Some(answer) = ask(input, prompt)? {
        // An existing literal name wins, including names containing quotes or backslashes.
        match fs::symlink_metadata(&answer) {
            Ok(_) => return Ok(Some(answer)),
            Err(e) if e.kind() == ErrorKind::PermissionDenied => return Ok(Some(answer)),
            Err(_) => {}
        }
        if action(&answer)? {
            continue;
        }
        let quoted = answer.strip_prefix('"').and_then(|s| s.strip_suffix('"'));
        let single_quoted = answer
            .strip_prefix('\'')
            .and_then(|s| s.strip_suffix('\''))
            .filter(|s| Path::new(s).is_absolute());
        if let Some(path) = quoted.or(single_quoted) {
            return Ok(Some(path.to_owned()));
        }
        if cfg!(unix)
            && answer.starts_with('/')
            && answer.contains("\\ ")
            && answer.split('\\').skip(1).all(|s| s.starts_with(' '))
        {
            let path = answer.replace("\\ ", " ");
            if fs::symlink_metadata(&path).is_ok() {
                return Ok(Some(path));
            }
            // A new name is unambiguous when only the unescaped parent exists.
            if Path::new(&answer)
                .parent()
                .is_some_and(|p| fs::metadata(p).is_err_and(|e| e.kind() == ErrorKind::NotFound))
                && Path::new(&path).parent().is_some_and(Path::is_dir)
            {
                return Ok(Some(path));
            }
            eprintln!(
                "Paste the full path with spaces as written. To keep a literal backslash in a new name, enclose the full path in double quotes."
            );
            continue;
        }
        return Ok(Some(answer));
    }
    Ok(None)
}

fn ask_source_path(input: &mut dyn BufRead) -> Result<Option<String>, String> {
    while let Some(path) = ask_path(input, "Paste or type the path to this RIS export:")? {
        if Path::new(&path).is_file() {
            return Ok(Some(path));
        }
        eprintln!(
            "Cannot locate a regular file there. Paste its full path from Explorer or Finder."
        );
    }
    Ok(None)
}

pub(super) fn run() -> Result<(), String> {
    let mut input = io::stdin().lock();
    eprintln!("Start a local RIS review. Type :cancel at any prompt to stop.");
    let Some(sheet) = ask_path(
        &mut input,
        "New or saved decision sheet path (e.g. choices.tsv):",
    )?
    else {
        return Ok(());
    };
    if sheet.is_empty() {
        return Err("Enter a filename for the decision sheet.".into());
    }
    let existing = match fs::metadata(&sheet) {
        Ok(m) if m.is_file() => true,
        Ok(_) => return Err("Choose a file for the decision sheet.".into()),
        Err(e) if e.kind() == ErrorKind::NotFound => false,
        Err(e) => return Err(format!("Cannot inspect the decision sheet: {e}")),
    };
    let verb = if existing {
        eprintln!("Use the same source labels and original RIS exports as the saved sheet.");
        "Reopen"
    } else {
        eprintln!(
            "The new sheet will include candidates for human verification by identifier or metadata."
        );
        "Create"
    };

    let mut sources = Vec::new();
    while sources.len() < ris::MAX_SOURCES {
        let prompt = if sources.is_empty() {
            "First source label (e.g. catalog):"
        } else {
            "Next source label (press Enter when done):"
        };
        let Some(label) = ask(&mut input, prompt)? else {
            return Ok(());
        };
        if label.is_empty() && !sources.is_empty() {
            break;
        }
        if !ris::valid_source_label(&label) {
            eprintln!(
                "Use 1 to 64 letters (A-Z, a-z), digits, dots, underscores or hyphens. Start with a letter or digit."
            );
            continue;
        }
        if sources.iter().any(|(name, _)| name == &label) {
            eprintln!("That label is already in use. Choose another label.");
            continue;
        }
        let Some(path) = ask_source_path(&mut input)? else {
            return Ok(());
        };
        sources.push((label, path));
    }

    // Only suggest labels here. Reopening still verifies the entire sheet and each original.
    let saved_labels: Vec<_> = if existing {
        let text = super::decisions::read_sheet(sheet.as_ref())?;
        super::decisions::saved_source_labels(&text)
            .into_iter()
            .filter(|label| ris::valid_source_label(label))
            .map(str::to_owned)
            .collect()
    } else {
        Vec::new()
    };
    let confirmation = if existing {
        "Type yes to continue, or press Enter to cancel:"
    } else {
        "Type review to create the sheet and inspect pending groups now, or yes to create it and print the full queue. Press Enter to cancel:"
    };
    let answer = loop {
        eprintln!("{verb} sheet {sheet} with these exports:");
        for (label, path) in &sources {
            eprintln!("  {label} = {path}");
        }
        for label in &saved_labels {
            if !sources.iter().any(|(name, _)| name == label) {
                eprintln!("Missing saved source '{label}'. Type add {label} to supply its path.");
            }
        }
        eprintln!(
            "Type edit LABEL to change a listed path, or add LABEL to include another export."
        );
        let Some(answer) = ask(&mut input, confirmation)? else {
            return Ok(());
        };
        let mut words = answer.split_whitespace();
        let Some(command @ ("edit" | "add")) = words.next() else {
            break answer;
        };
        let label = words.next().unwrap_or("");
        if !ris::valid_source_label(label) || words.next().is_some() {
            eprintln!(
                "Enter {command} followed by one source label, for example {command} catalog."
            );
            continue;
        }
        let position = sources.iter().position(|(name, _)| name == label);
        if command == "edit" && position.is_none() {
            eprintln!("The list has no '{label}' source. Check the label you want to edit.");
            continue;
        }
        if command == "add" && position.is_some() {
            eprintln!("'{label}' is already listed. To change its path, type edit {label}.");
            continue;
        }
        if command == "add" && sources.len() == ris::MAX_SOURCES {
            eprintln!("The source list is full. To replace a path, use its existing label.");
            continue;
        }
        let Some(path) = ask_source_path(&mut input)? else {
            return Ok(());
        };
        if let Some(i) = position {
            sources[i].1 = path;
        } else {
            sources.push((label.to_owned(), path));
        }
    };
    let mut args = Vec::<OsString>::new();
    for (label, path) in sources {
        args.extend(["--ris-source".into(), label.into(), path.into()]);
    }
    if !existing && answer.eq_ignore_ascii_case("review") {
        return sources::run_action(
            Action::Pending(sheet.as_ref(), PendingMode::CreateAndReview(&mut input)),
            &args,
        );
    }
    if !answer.eq_ignore_ascii_case("yes") {
        eprintln!("Review setup cancelled.");
        return Ok(());
    }

    if existing {
        return sources::run_action(
            Action::Pending(sheet.as_ref(), PendingMode::Reopen(&mut input)),
            &args,
        );
    }
    sources::run_action(
        Action::Init {
            path: sheet.as_ref(),
            metadata: true,
            guided: true,
        },
        &args,
    )?;
    eprintln!(
        "Open {sheet} in a plain-text editor or follow the LibreOffice Calc steps in the README. Each label:number refers to a numbered record in that export. Leave uncertain decisions pending."
    );
    Ok(())
}

pub(super) enum Inspection<'a> {
    Pending(Groups),
    ListLeads,
    Find(&'a str),
    Pair(&'a str, &'a str),
}

pub(super) fn export_destination(
    input: &mut dyn BufRead,
    mut inspect: impl FnMut(Inspection<'_>) -> Result<(), String>,
) -> Result<Option<PathBuf>, String> {
    while let Some(path) = ask_path_or_action(
        input,
        "To compare two keys, type :inspect KEY,KEY. Look up records with :find TEXT, :find --author NAME | TITLE or :find --venue VENUE | TITLE. Type :pending for all pending groups, :pending metadata for title/author/year/type groups, or :leads for inspection-only pairs. To preview an export, enter a new folder path. Press Enter to close this session:",
        |answer| {
            if answer == ":leads" {
                inspect(Inspection::ListLeads)?;
                return Ok(true);
            }
            let (command, argument) = answer
                .split_once(char::is_whitespace)
                .unwrap_or((answer, ""));
            if command == ":pending" {
                match argument.trim() {
                    "" => inspect(Inspection::Pending(Groups::All))?,
                    "metadata" => inspect(Inspection::Pending(Groups::Metadata))?,
                    _ => eprintln!("Use :pending or :pending metadata."),
                }
                return Ok(true);
            }
            if command == ":find" {
                let query = argument.trim();
                if query.is_empty() {
                    eprintln!("Enter part of a title or an author after :find.");
                } else {
                    inspect(Inspection::Find(query))?;
                }
                return Ok(true);
            }
            if command != ":inspect" {
                return Ok(false);
            }
            match argument.split_once(',').map(|(a, b)| (a.trim(), b.trim())) {
                Some((left, right))
                    if !left.is_empty() && !right.is_empty() && !right.contains(',') =>
                {
                    inspect(Inspection::Pair(left, right))?;
                }
                _ => eprintln!("Enter exactly two keys, for example :inspect catalog:1,index:2."),
            }
            Ok(true)
        },
    )? {
        if path.is_empty() {
            break;
        }
        let directory = PathBuf::from(path);
        match super::reviewed::require_new_destination(&directory) {
            Ok(()) => return Ok(Some(directory)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => eprintln!("{e}"),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(None)
}

pub(super) fn confirm_export(
    input: &mut dyn BufRead,
    sheet: &Path,
    directory: &Path,
    export: &PreparedExport,
    queue: &ris::ReviewQueue<'_>,
    decisions: &[Decision],
) -> Result<bool, String> {
    eprintln!("Decision sheet: {}", sheet.display());
    eprintln!("New export folder: {}", directory.display());
    eprintln!("Explicitly marked duplicates to remove: {}", export.removed);
    eprintln!(
        "Records to export for screening: {} (including {} pending duplicate review). A report may appear more than once.",
        export.retained, export.pending
    );
    if decisions
        .iter()
        .any(|d| matches!(d, Decision::Keep(Some(_))))
    {
        eprintln!(
            "Keep reasons are optional. The decision sheet and exported mapping.tsv contain them in plain text. \
             The reviewed RIS and ordinary trace omit them."
        );
    }
    show_removal_advisories(queue, decisions);
    show_edition_advisories(queue, decisions);
    let Some(answer) = ask(
        input,
        "Type write to create this export, or press Enter to cancel:",
    )?
    else {
        return Ok(false);
    };
    if answer.eq_ignore_ascii_case("write") {
        return Ok(true);
    }
    eprintln!("Export cancelled.");
    Ok(false)
}

fn show_edition_advisories(queue: &ris::ReviewQueue<'_>, decisions: &[Decision]) {
    if !decisions
        .iter()
        .any(|d| matches!(d, Decision::VerifiedMetadata { .. }))
    {
        return;
    }
    let editions: Vec<_> = queue
        .sources
        .iter()
        .flat_map(|s| &s.source.occurrences)
        .zip(decisions)
        .map(|(record, decision)| match decision {
            Decision::Keep(_) | Decision::VerifiedMetadata { .. } => {
                record.first_or_second_edition()
            }
            _ => None,
        })
        .collect();
    let mut shown = false;
    for (index, decision) in decisions.iter().enumerate() {
        let Decision::VerifiedMetadata { target, .. } = decision else {
            continue;
        };
        let (Some((from, qualifier)), Some((to, other))) = (editions[index], editions[*target])
        else {
            continue;
        };
        if from != to && qualifier.eq_ignore_ascii_case(other) {
            let removed = &queue.entries[index];
            let kept = &queue.entries[*target];
            eprintln!(
                "Different ET edition numbers in this proposed metadata removal. {}:{} (remove) says edition {from} and {}:{} (keep) says edition {to}.",
                removed.label, removed.ordinal, kept.label, kept.ordinal
            );
            shown = true;
        }
    }
    if shown {
        eprintln!(
            "The edition labels may describe separate reports. If unsure, cancel and compare the keys with :inspect before deciding."
        );
    }
}

fn show_removal_advisories(queue: &ris::ReviewQueue<'_>, decisions: &[Decision]) {
    if !decisions
        .iter()
        .any(|d| matches!(d, Decision::DuplicateOf(_)))
    {
        return;
    }
    // Rows are in queue order. Read each record at most once even for a shared keeper.
    let wording: Vec<_> = queue
        .sources
        .iter()
        .flat_map(|s| &s.source.occurrences)
        .zip(decisions)
        .map(|(record, decision)| match decision {
            Decision::Keep(_) | Decision::DuplicateOf(_) => (
                record.protocol_or_results_wording(),
                record.protocol_or_results_work_type(),
                record.notice_title(),
            ),
            _ => (None, None, None),
        })
        .collect();
    let mut shown = false;
    for (index, decision) in decisions.iter().enumerate() {
        // A validated identifier decision already requires direct DOI or PMID group membership.
        let Decision::DuplicateOf(target) = decision else {
            continue;
        };
        let (removed_title, removed_type, removed_notice) = &wording[index];
        let (kept_title, kept_type, kept_notice) = &wording[*target];
        let removed = &queue.entries[index];
        let kept = &queue.entries[*target];
        if let (Some(left), Some(right)) = (removed_notice, kept_notice) {
            let notice = left
                .notice_prefix_for(right)
                .map(|p| (p, removed))
                .or_else(|| right.notice_prefix_for(left).map(|p| (p, kept)));
            if let Some((prefix, entry)) = notice {
                eprintln!(
                    "Check {}:{} (remove) and {}:{} (keep). The notice prefix '{prefix}' on {}:{} introduces the other record's title. A separately issued notice can repeat the article's identifier.",
                    removed.label,
                    removed.ordinal,
                    kept.label,
                    kept.ordinal,
                    entry.label,
                    entry.ordinal
                );
                shown = true;
            }
        }
        match (removed_title, kept_title, removed_type, kept_type) {
            (Some((stem, word)), Some((other_stem, other_word)), _, _)
                if stem == other_stem && word != other_word =>
            {
                eprintln!(
                    "Title wording differs for this shared-identifier removal. {}:{} (remove) ends in '{word}' and {}:{} (keep) ends in '{other_word}'.",
                    removed.label, removed.ordinal, kept.label, kept.ordinal
                );
            }
            (_, _, Some(word), Some(other_word)) if word != other_word => eprintln!(
                "Check the M3 work types for this shared-identifier removal. {}:{} (remove) has '{word}' wording and {}:{} (keep) has '{other_word}' wording.",
                removed.label, removed.ordinal, kept.label, kept.ordinal
            ),
            _ => continue,
        }
        shown = true;
    }
    if shown {
        eprintln!(
            "These words may describe separate reports or be literal labels in the same report. If unsure, cancel and reopen the sheet to compare the keys with :inspect. Use keep for both distinct reports, or leave an uncertain removal pending."
        );
    }
}
