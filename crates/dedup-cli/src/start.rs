//! Prompt for the arguments to an ordinary, local named-RIS review.
use super::{
    decisions::Decision,
    reviewed::PreparedExport,
    sources::{self, Action},
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
        let path = loop {
            let Some(path) = ask_path(&mut input, "Paste or type the path to this RIS export:")?
            else {
                return Ok(());
            };
            if Path::new(&path).is_file() {
                break path;
            }
            eprintln!(
                "Cannot locate a regular file there. Paste its full path from Explorer or Finder."
            );
        };
        sources.push((label, path));
    }

    eprintln!("{verb} sheet {sheet} with these exports:");
    let mut args = Vec::<OsString>::new();
    for (label, path) in sources {
        eprintln!("  {label} = {path}");
        args.extend(["--ris-source".into(), label.into(), path.into()]);
    }
    let Some(answer) = ask(
        &mut input,
        "Type yes to continue, or press Enter to cancel:",
    )?
    else {
        return Ok(());
    };
    if !answer.eq_ignore_ascii_case("yes") {
        eprintln!("Review setup cancelled.");
        return Ok(());
    }

    if existing {
        return sources::run_action(Action::Pending(sheet.as_ref(), Some(&mut input)), &args);
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

pub(super) fn export_destination(
    input: &mut dyn BufRead,
    mut inspect: impl FnMut(&str, &str) -> Result<(), String>,
) -> Result<Option<PathBuf>, String> {
    let Some(path) = ask_path_or_action(
        input,
        "To compare two keys from a pending row, type :inspect KEY,KEY. You can compare two pending records. To preview an export, enter a new folder path. Press Enter to close this session:",
        |answer| {
            let (command, keys) = answer
                .split_once(char::is_whitespace)
                .unwrap_or((answer, ""));
            if command != ":inspect" {
                return Ok(false);
            }
            match keys.split_once(',').map(|(a, b)| (a.trim(), b.trim())) {
                Some((left, right))
                    if !left.is_empty() && !right.is_empty() && !right.contains(',') =>
                {
                    inspect(left, right)?;
                }
                _ => eprintln!("Enter exactly two keys, for example :inspect catalog:1,index:2."),
            }
            Ok(true)
        },
    )?
    else {
        return Ok(None);
    };
    Ok((!path.is_empty()).then(|| PathBuf::from(path)))
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
        "Records retained: {} (including {} pending).",
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
    show_title_advisories(queue, decisions);
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

fn show_title_advisories(queue: &ris::ReviewQueue<'_>, decisions: &[Decision]) {
    if !decisions
        .iter()
        .any(|d| matches!(d, Decision::DuplicateOf(_)))
    {
        return;
    }
    // Rows are in queue order. Read each title at most once even for a shared keeper.
    let endings: Vec<_> = queue
        .sources
        .iter()
        .flat_map(|s| &s.source.occurrences)
        .zip(decisions)
        .map(|(record, decision)| match decision {
            Decision::Keep(_) | Decision::DuplicateOf(_) => record.protocol_or_results_wording(),
            _ => None,
        })
        .collect();
    let mut shown = false;
    for (index, decision) in decisions.iter().enumerate() {
        // A validated identifier decision already requires direct DOI or PMID group membership.
        let Decision::DuplicateOf(target) = decision else {
            continue;
        };
        let (Some((removed_stem, removed_word)), Some((kept_stem, kept_word))) =
            (&endings[index], &endings[*target])
        else {
            continue;
        };
        if removed_stem == kept_stem && removed_word != kept_word {
            let removed = &queue.entries[index];
            let kept = &queue.entries[*target];
            eprintln!(
                "Title wording differs for this shared-identifier removal. {}:{} (remove) ends in '{removed_word}' and {}:{} (keep) ends in '{kept_word}'.",
                removed.label, removed.ordinal, kept.label, kept.ordinal
            );
            shown = true;
        }
    }
    if shown {
        eprintln!(
            "These words may describe separate reports or be literal labels in the same report. If unsure, cancel and reopen the sheet to compare the keys with :inspect."
        );
    }
}
