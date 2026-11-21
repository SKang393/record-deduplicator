//! Prompt for the arguments to an ordinary, local named-RIS review.
use super::{
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
    while let Some(answer) = ask(input, prompt)? {
        // An existing literal name wins, including names containing quotes or backslashes.
        match fs::symlink_metadata(&answer) {
            Ok(_) => return Ok(Some(answer)),
            Err(e) if e.kind() == ErrorKind::PermissionDenied => return Ok(Some(answer)),
            Err(_) => {}
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

pub(super) fn export_destination(input: &mut dyn BufRead) -> Result<Option<PathBuf>, String> {
    let Some(path) = ask_path(
        input,
        "To preview an export, enter a new folder path. Press Enter to close this session:",
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
) -> Result<bool, String> {
    eprintln!("Decision sheet: {}", sheet.display());
    eprintln!("New export folder: {}", directory.display());
    eprintln!("Explicitly marked duplicates to remove: {}", export.removed);
    eprintln!(
        "Records retained: {} (including {} pending).",
        export.retained, export.pending
    );
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
