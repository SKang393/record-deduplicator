//! Editable review snapshots for named RIS inputs.
use std::{
    borrow::Cow,
    collections::BTreeMap,
    ffi::OsStr,
    fmt::Write as _,
    fs::{self, File},
    io::{Read, Write},
};

use dedup_core::{
    identifier_review::{Identifier, MAX_INPUT_BYTES},
    ris::{Disposition, ReviewQueue},
};

const VERSION: &str = "dedup-ris-decisions\t1";
const END: &str = "end";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Decision {
    NotCandidate,
    Pending,
    Keep,
    DuplicateOf(usize),
}

impl Decision {
    pub(super) fn label(self, queue: &ReviewQueue<'_>) -> Cow<'static, str> {
        match self {
            Self::NotCandidate => "not-candidate".into(),
            Self::Pending => "pending".into(),
            Self::Keep => "keep".into(),
            Self::DuplicateOf(index) => {
                let target = &queue.entries[index];
                format!("duplicate-of={}:{}", target.label, target.ordinal).into()
            }
        }
    }
}

fn candidates<'a>(queue: &'a ReviewQueue<'_>) -> impl Iterator<Item = (usize, String)> + 'a {
    queue
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.disposition == Disposition::ReviewCandidate)
        .map(|(index, entry)| (index, format!("{}:{}", entry.label, entry.ordinal)))
}

pub(super) fn create(
    path: &OsStr,
    queue: &ReviewQueue<'_>,
    manifest: &[String],
) -> Result<(), String> {
    let mut sheet = format!("{VERSION}\n");
    for identity in manifest {
        writeln!(sheet, "{identity}").unwrap();
    }
    for (_, key) in candidates(queue) {
        writeln!(sheet, "{key}\tpending").unwrap();
    }
    writeln!(sheet, "{END}").unwrap();
    let mut file =
        File::create_new(path).map_err(|e| format!("Cannot create a new decision sheet: {e}"))?;
    file.write_all(sheet.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("Cannot finish decision sheet: {e}"))
}

pub(super) fn resume(
    path: &OsStr,
    queue: &ReviewQueue<'_>,
    manifest: &[String],
) -> Result<Vec<Decision>, String> {
    if !fs::metadata(path).is_ok_and(|m| m.is_file()) {
        return Err("Expected a regular decision sheet file.".into());
    }
    let mut text = String::new();
    File::open(path)
        .map_err(|e| format!("Cannot open decision sheet: {e}"))?
        .take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("Cannot read UTF-8 decision sheet: {e}"))?;
    if text.len() > MAX_INPUT_BYTES {
        return Err("Decision sheet exceeds 16 MiB.".into());
    }
    let mut lines = text.strip_prefix('\u{feff}').unwrap_or(&text).lines();
    if lines.next() != Some(VERSION) {
        return Err("Unsupported decision sheet header or version.".into());
    }
    for identity in manifest {
        if lines.next() != Some(identity.as_str()) {
            return Err(
                "Decision sheet inputs differ. Check all RIS labels and source bytes.".into(),
            );
        }
    }
    let mut choices = BTreeMap::new();
    for (index, key) in candidates(queue) {
        let (saved_key, choice) = lines
            .next()
            .and_then(|line| line.split_once('\t'))
            .filter(|(saved_key, _)| *saved_key == key)
            .ok_or_else(|| format!("Decision sheet is missing the expected row for {key}."))?;
        choices.insert(saved_key, (index, choice));
    }
    if lines.next() != Some(END) || lines.next().is_some() {
        return Err("Decision sheet has unexpected rows or a missing end marker.".into());
    }

    // A record can belong to one DOI group and one PMID group. Check direct
    // membership without expanding large groups into all possible pairs.
    let mut groups = vec![[None; 2]; queue.entries.len()];
    for (id, group) in queue.review.groups.iter().enumerate() {
        let slot = match group.shared {
            Identifier::Doi => 0,
            Identifier::Pmid => 1,
        };
        for row in &group.rows {
            groups[row - 1][slot] = Some(id);
        }
    }
    let mut decisions = vec![Decision::NotCandidate; queue.entries.len()];
    for (&key, &(index, choice)) in &choices {
        decisions[index] = match choice {
            "keep" => Decision::Keep,
            "pending" => Decision::Pending,
            _ => {
                let other = choice.strip_prefix("duplicate-of=").ok_or_else(|| format!(
                    "Unknown decision for {key}. Expected pending, keep or duplicate-of=<occurrence>."
                ))?;
                let Some(&(target, "keep")) = choices.get(other) else {
                    return Err(format!(
                        "The duplicate target for {key} must have a keep decision."
                    ));
                };
                if !groups[index]
                    .iter()
                    .flatten()
                    .any(|id| groups[target].contains(&Some(*id)))
                {
                    return Err(format!(
                        "{key} and its duplicate target must share a direct candidate group."
                    ));
                }
                Decision::DuplicateOf(target)
            }
        };
    }
    Ok(decisions)
}
