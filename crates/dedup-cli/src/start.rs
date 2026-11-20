//! Prompt for the arguments to an ordinary, local named-RIS review.
use dedup_core::ris;
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, ErrorKind, Write},
    path::Path,
};

fn ask(input: &mut impl BufRead, prompt: &str) -> Result<Option<String>, String> {
    eprint!("{prompt} ");
    io::stderr()
        .flush()
        .map_err(|_| "Cannot display the question.")?;
    let mut line = String::new();
    let count = input
        .read_line(&mut line)
        .map_err(|_| "Cannot read an answer.")?;
    let answer = line.trim_end_matches(['\r', '\n']);
    if count == 0 || answer == ":cancel" {
        eprintln!("Review setup cancelled.");
        Ok(None)
    } else {
        Ok(Some(answer.to_owned()))
    }
}

fn path_text(answer: &str) -> &str {
    answer
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(answer)
}

pub(super) fn run() -> Result<(), String> {
    let mut input = io::stdin().lock();
    eprintln!("Start a local RIS review. Type :cancel at any prompt to stop.");
    let Some(answer) = ask(
        &mut input,
        "New or saved decision sheet path (e.g. choices.tsv):",
    )?
    else {
        return Ok(());
    };
    let sheet = path_text(&answer);
    if sheet.is_empty() {
        return Err("Enter a filename for the decision sheet.".into());
    }
    let existing = match fs::metadata(sheet) {
        Ok(m) if m.is_file() => true,
        Ok(_) => return Err("Choose a file for the decision sheet.".into()),
        Err(e) if e.kind() == ErrorKind::NotFound => false,
        Err(e) => return Err(format!("Cannot inspect the decision sheet: {e}")),
    };
    let (verb, flag) = if existing {
        eprintln!("Use the same source labels and original RIS exports as the saved sheet.");
        ("Reopen", "--pending-decisions")
    } else {
        eprintln!(
            "The new sheet will include candidates for human verification by identifier or metadata."
        );
        ("Create", "--init-metadata-decisions")
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
            let Some(answer) = ask(&mut input, "Paste or type the path to this RIS export:")?
            else {
                return Ok(());
            };
            let path = path_text(&answer);
            if Path::new(path).is_file() {
                break path.to_owned();
            }
            eprintln!("Cannot locate a regular file there. Try its full path.");
        };
        sources.push((label, path));
    }

    eprintln!("{verb} sheet {sheet} with these exports:");
    let mut args = vec![OsString::from(flag), OsString::from(sheet)];
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

    super::sources::run(&args)?;
    eprintln!(
        "Open {sheet} in a plain-text editor to review decisions. Each label:number refers to a numbered record in that export. Leave uncertain decisions pending."
    );
    Ok(())
}
