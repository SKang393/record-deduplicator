//! Resolve an occurrence in a completed export, independently of the editable sheet.
use std::{
    ffi::{OsStr, OsString},
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use dedup_core::{identifier_review::MAX_INPUT_BYTES, ris};
use sha2::{Digest, Sha256};

use super::{decisions::valid_rationale, mapping::INVALID};

fn optional_row<'a>(
    mapping: &'a str,
    kind: &str,
    key: &str,
) -> Result<Option<(&'a str, &'a str)>, String> {
    let mut matches = mapping.lines().filter_map(|line| {
        let mut fields = line.splitn(4, '\t');
        if fields.next()? != kind {
            return None;
        }
        let signal = fields.next()?;
        if fields.next()? != key {
            return None;
        }
        Some(fields.next().map(|detail| (signal, detail)).ok_or(INVALID))
    });
    let found = matches.next().transpose()?;
    if matches.next().is_some() {
        return Err(format!("Multiple {kind} rows for {key} in mapping.tsv."));
    }
    Ok(found)
}

fn row<'a>(mapping: &'a str, kind: &str, key: &str) -> Result<(&'a str, &'a str), String> {
    optional_row(mapping, kind, key)?
        .ok_or_else(|| format!("No {kind} row for {key} in mapping.tsv."))
}

fn is_retained(decision: &str) -> bool {
    matches!(decision, "keep" | "pending" | "not-candidate")
}

// The exporter writes the retained occurrences in map-row order.
fn retained_key_at(mapping: &str, output: usize) -> Option<&str> {
    mapping
        .lines()
        .filter_map(|line| {
            let (decision, row) = line.strip_prefix("map\t")?.split_once('\t')?;
            let (key, _) = row.split_once('\t')?;
            is_retained(decision).then_some(key)
        })
        .nth(output - 1)
}

struct Location<'a> {
    key: &'a str,
    ordinal: usize,
    decision: &'a str,
    output: usize,
    span: &'a str,
    identity: &'a str,
    file_size: usize,
    digest: &'a str,
    files: Vec<PathBuf>,
}

impl<'a> Location<'a> {
    fn lookup(mapping: &'a str, key: &'a str) -> Result<Self, String> {
        let syntax = "Expected an occurrence key from mapping.tsv, for example catalog:2.";
        let (label, number) = key.split_once(':').ok_or(syntax)?;
        let ordinal = number.parse::<usize>().map_err(|_| syntax)?;
        if !ris::valid_source_label(label) || ordinal == 0 || number != ordinal.to_string() {
            return Err(syntax.into());
        }
        let (decision, detail) = row(mapping, "map", key)?;
        let (span, output) = detail.split_once(";output=").ok_or(INVALID)?;
        let output = output.parse().ok().filter(|&n| n > 0).ok_or(INVALID)?;
        if is_retained(decision) && retained_key_at(mapping, output) != Some(key) {
            return Err(format!(
                "Output {output} for {key} disagrees with the retained order in mapping.tsv. Use the original mapping from this export."
            ));
        }
        let (format, identity) = row(mapping, "input", label)?;
        let file_size = identity
            .strip_prefix("bytes=")
            .and_then(|s| s.split_once(';'))
            .and_then(|(n, _)| n.parse().ok())
            .filter(|&n| n <= MAX_INPUT_BYTES)
            .ok_or(INVALID)?;
        let (_, digest) = identity.rsplit_once(";sha256=").ok_or(INVALID)?;
        if format != "ris" || digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(INVALID.into());
        }
        Ok(Self {
            key,
            ordinal,
            decision,
            output,
            span,
            identity,
            file_size,
            digest,
            files: Vec::new(),
        })
    }

    fn verify(&self, source: &ris::Source<'_>) -> Result<(), String> {
        let rec = source
            .occurrences
            .get(self.ordinal - 1)
            .filter(|r| r.usable())
            .ok_or(INVALID)?;
        let identity = format!(
            "bytes={};occurrences={};sha256={}",
            source.original.len(),
            source.occurrences.len(),
            self.digest
        );
        if identity != self.identity
            || format!("bytes={}..{}", rec.bytes.start, rec.bytes.end) != self.span
        {
            return Err(format!(
                "Original occurrence {} disagrees with mapping.tsv.",
                self.key
            ));
        }
        Ok(())
    }
}

fn resolve(folder: &Path, locations: &mut [Location<'_>]) -> Result<(), String> {
    let io_error = |_: io::Error| "Cannot inspect files in the originals directory.";
    for item in fs::read_dir(folder).map_err(io_error)? {
        let item = item.map_err(io_error)?;
        if !item.file_type().map_err(io_error)?.is_file() {
            continue;
        }
        let size = item.metadata().map_err(io_error)?.len();
        if !locations.iter().any(|loc| loc.file_size as u64 == size) {
            continue;
        }
        // Hash before decoding: unrelated files of the same length need not be RIS or UTF-8.
        let path = item.path();
        let mut bytes = Vec::new();
        super::open_ris(path.as_os_str())?
            .take(size + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if !locations.iter().any(|loc| loc.digest == digest) {
            continue;
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| INVALID)?;
        let source = ris::parse(text).map_err(super::input_error)?;
        for loc in locations.iter_mut().filter(|loc| loc.digest == digest) {
            loc.verify(&source)?;
            loc.files.push(path.clone());
        }
    }
    Ok(())
}

pub(super) fn run(
    mapping: &Path,
    key: &OsStr,
    originals: &[OsString],
    show_keep_reason: bool,
) -> Result<(), String> {
    let text = super::mapping::read(mapping)?;
    let selected = Location::lookup(&text, key.to_str().ok_or("Expected a label and ordinal.")?)?;
    let keep_reason = if show_keep_reason {
        if selected.decision != "keep" {
            return Err("Select an explicitly kept occurrence to show its keep reason.".into());
        }
        match optional_row(&text, "rationale", selected.key)? {
            Some(("keep", reason))
                if valid_rationale(reason)
                    && text.lines().nth(1) == Some(super::mapping::RATIONALE_SCHEMA) =>
            {
                Some(reason)
            }
            Some(_) => return Err(INVALID.into()),
            None => Some(""),
        }
    } else {
        None
    };
    let mut locations = vec![selected];
    let decision = locations[0].decision;
    if let Some(key) = decision
        .strip_prefix("duplicate-of=")
        .or_else(|| decision.strip_prefix("verified-duplicate-of="))
    {
        let keeper = Location::lookup(&text, key)?;
        if keeper.decision != "keep" || keeper.output != locations[0].output {
            return Err("Removed occurrence and keeper disagree in mapping.tsv.".into());
        }
        locations.push(keeper);
    } else if !is_retained(decision) {
        return Err(INVALID.into());
    }

    for folder in originals {
        resolve(folder.as_ref(), &mut locations)?;
    }
    for loc in &mut locations {
        if loc.files.is_empty() {
            return Err(format!(
                "No original file matches {} (SHA-256 {}).",
                loc.key, loc.digest
            ));
        }
        loc.files.sort();
        loc.files.dedup();
    }
    let mut report = io::BufWriter::new(io::stdout().lock());
    let write_error = |_: io::Error| "Cannot write trace.";
    writeln!(
        report,
        "occurrence\tdecision\toutput\toriginal_bytes\tsource_sha256\tmatching_path{}",
        if show_keep_reason {
            "\tkeep_reason"
        } else {
            ""
        }
    )
    .map_err(write_error)?;
    for location in locations {
        for path in location.files {
            write!(
                report,
                "{}\t{}\t{}\t{}\t{}\t{:?}",
                location.key,
                location.decision,
                location.output,
                location.span,
                location.digest,
                path
            )
            .map_err(write_error)?;
            if let Some(reason) = keep_reason {
                write!(report, "\t{reason}").map_err(write_error)?;
            }
            writeln!(report).map_err(write_error)?;
        }
    }
    report.flush().map_err(|_| "Cannot write trace.".into())
}
