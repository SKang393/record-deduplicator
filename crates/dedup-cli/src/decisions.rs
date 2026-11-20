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
    ris::{Disposition, MetadataHint, ReportIdentifiers, ReviewQueue},
};

const VERSION: &str = "dedup-ris-decisions\t1";
const METADATA_VERSION: &str = "dedup-ris-decisions\t2";
const END: &str = "end";

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Decision {
    NotCandidate,
    Pending,
    Keep,
    DuplicateOf(usize),
    VerifiedMetadata { target: usize, rationale: String },
}

impl Decision {
    pub(super) fn target(&self) -> Option<usize> {
        match self {
            Self::DuplicateOf(target) | Self::VerifiedMetadata { target, .. } => Some(*target),
            _ => None,
        }
    }

    pub(super) fn label(&self, queue: &ReviewQueue<'_>) -> Cow<'static, str> {
        match self {
            Self::NotCandidate => "not-candidate".into(),
            Self::Pending => "pending".into(),
            Self::Keep => "keep".into(),
            Self::DuplicateOf(index) => {
                let target = &queue.entries[*index];
                format!("duplicate-of={}:{}", target.label, target.ordinal).into()
            }
            Self::VerifiedMetadata { target, .. } => {
                let target = &queue.entries[*target];
                format!("verified-duplicate-of={}:{}", target.label, target.ordinal).into()
            }
        }
    }
}

fn metadata_groups(queue: &ReviewQueue<'_>, hints: &[MetadataHint]) -> Vec<Option<usize>> {
    let mut groups = vec![None; queue.entries.len()];
    for hint in hints {
        if let MetadataHint::Inspect(rows) = hint {
            for &row in rows {
                groups[row - 1] = Some(rows[0]);
            }
        }
    }
    groups
}

fn candidates<'a>(
    queue: &'a ReviewQueue<'_>,
    groups: &'a [Option<usize>],
) -> impl Iterator<Item = (usize, String)> + 'a {
    queue
        .entries
        .iter()
        .enumerate()
        .filter(|(i, entry)| {
            entry.disposition == Disposition::ReviewCandidate || groups[*i].is_some()
        })
        .map(|(index, entry)| (index, format!("{}:{}", entry.label, entry.ordinal)))
}

pub(super) fn create(
    path: &OsStr,
    queue: &ReviewQueue<'_>,
    manifest: &[String],
    metadata: bool,
) -> Result<(), String> {
    let version = if metadata { METADATA_VERSION } else { VERSION };
    let mut sheet = format!("{version}\n");
    for identity in manifest {
        writeln!(sheet, "{identity}").unwrap();
    }
    let hints = if metadata {
        queue.metadata_hints()
    } else {
        Vec::new()
    };
    for (_, key) in candidates(queue, &metadata_groups(queue, &hints)) {
        writeln!(sheet, "{key}\tpending").unwrap();
    }
    writeln!(sheet, "{END}").unwrap();
    let mut file =
        File::create_new(path).map_err(|e| format!("Cannot create a new decision sheet: {e}"))?;
    file.write_all(sheet.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("Cannot finish decision sheet: {e}"))
}

pub(super) struct ResumedSheet {
    pub decisions: Vec<Decision>,
    /// Only opt-in sheets can use metadata hint membership for decisions.
    pub metadata_hints: Option<Vec<MetadataHint>>,
}

pub(super) fn resume(
    path: &OsStr,
    queue: &ReviewQueue<'_>,
    manifest: &[String],
) -> Result<ResumedSheet, String> {
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
    // Calc writes short rows out to the width of the sheet with empty cells.
    let mut lines = text
        .strip_prefix('\u{feff}')
        .unwrap_or(&text)
        .lines()
        .map(|line| line.trim_end_matches('\t'));
    let metadata = match lines.next() {
        Some(VERSION) => false,
        Some(METADATA_VERSION) => true,
        _ => return Err("Unsupported decision sheet header or version.".into()),
    };
    for identity in manifest {
        if lines.next() != Some(identity.as_str()) {
            return Err(
                "Decision sheet inputs differ. Check all RIS labels and source bytes.".into(),
            );
        }
    }
    let metadata_hints = metadata.then(|| queue.metadata_hints());
    let hints = metadata_groups(queue, metadata_hints.as_deref().unwrap_or_default());
    let mut choices = BTreeMap::new();
    for (index, key) in candidates(queue, &hints) {
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
        let keep_target = |other: &str| match choices.get(other) {
            Some(&(target, "keep")) => Ok(target),
            _ => Err(format!(
                "The duplicate target for {key} must have a keep decision."
            )),
        };
        decisions[index] = match choice {
            "keep" => Decision::Keep,
            "pending" => Decision::Pending,
            _ if metadata && choice.starts_with("verified-duplicate-of=") => {
                let (other, rationale) = choice["verified-duplicate-of=".len()..]
                    .split_once('\t')
                    .filter(|(_, s)| !s.trim().is_empty() && s.len() <= 512
                        && !s.contains(|ch: char| ch.is_control() || matches!(ch, '\u{2028}' | '\u{2029}')))
                    .ok_or_else(|| format!("Expected a tab and a single-line rationale (1 to 512 UTF-8 bytes) for {key}."))?;
                let target = keep_target(other)?;
                Decision::VerifiedMetadata {
                    target,
                    rationale: rationale.to_owned(),
                }
            }
            _ => {
                let other = choice.strip_prefix("duplicate-of=").ok_or_else(|| format!(
                    "Unknown decision for {key}. Expected pending, keep or duplicate-of=<occurrence>."
                ))?;
                let target = keep_target(other)?;
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
    validate_metadata_sets(queue, &decisions, &hints)?;
    Ok(ResumedSheet {
        decisions,
        metadata_hints,
    })
}

/// Apply the same guards to every member, including copies assigned via an identifier.
fn validate_metadata_sets(
    queue: &ReviewQueue<'_>,
    decisions: &[Decision],
    hints: &[Option<usize>],
) -> Result<(), String> {
    let mut sets: BTreeMap<_, _> = decisions
        .iter()
        .filter_map(|decision| match decision {
            Decision::VerifiedMetadata { target, .. } => {
                Some((*target, (None, ReportIdentifiers::default())))
            }
            _ => None,
        })
        .collect();
    if sets.is_empty() {
        return Ok(());
    }
    let records = queue.sources.iter().flat_map(|s| &s.source.occurrences);
    for (i, (rec, decision)) in records.zip(decisions).enumerate() {
        let target = decision.target().unwrap_or(i);
        let Some((publication, ids)) = sets.get_mut(&target) else {
            continue;
        };
        let entry = &queue.entries[i];
        let key = format!("{}:{}", entry.label, entry.ordinal);
        if hints[i].is_none() || hints[i] != hints[target] {
            return Err(format!(
                "{key} and its keep target must belong to the same bounded metadata hint."
            ));
        }
        let details = rec.publication_details();
        if details.is_none() || (publication.is_some() && *publication != details) {
            return Err(format!(
                "Missing or differing publication details (authors, work type, journal, volume, issue or final pages/article number) for {key} and its keep target."
            ));
        }
        if publication.is_none() {
            *publication = details;
        }
        ids.observe(rec);
        if let Some(kind) = ids.conflict() {
            return Err(format!(
                "Conflicting {} in the verified metadata set at {key}.",
                match kind {
                    Identifier::Doi => "DOI",
                    Identifier::Pmid => "PMID",
                }
            ));
        }
    }
    Ok(())
}
