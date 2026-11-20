//! Export an already validated review without editing the sources or decision sheet.
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs::File,
    io::{self, Write},
    path::Path,
};

use dedup_core::{
    identifier_review::Identifier,
    ris::{ReportIdentifiers, ReviewQueue},
};
use sha2::{Digest, Sha256};

use super::decisions::Decision;

pub(super) fn export(
    directory: &Path,
    queue: &ReviewQueue<'_>,
    manifest: &[String],
    decisions: &[Decision],
) -> Result<(), String> {
    // A removed occurrence may name a keep target later in the queue.
    let mut retained = 0;
    let mut output_ordinals = Vec::with_capacity(decisions.len());
    let mut proposed_sets = BTreeMap::<usize, ReportIdentifiers>::new();
    for decision in decisions {
        if let Some(target) = decision.target() {
            proposed_sets.entry(target).or_default();
        } else {
            retained += 1;
        }
        output_ordinals.push(retained);
    }
    let version = if decisions
        .iter()
        .any(|d| matches!(d, Decision::VerifiedMetadata { .. }))
    {
        2
    } else {
        1
    };
    let mut mapping =
        format!("type\tsignal\trows\tdetail\nschema\texport\t{version}\treviewed-ris\n");
    for identity in manifest {
        writeln!(mapping, "{identity}").unwrap();
    }
    let total = queue.entries.len();
    writeln!(
        mapping,
        "summary\tall\t{total}\tretained={retained};suppressed={}",
        total - retained
    )
    .unwrap();

    let mut ris = String::new();
    let records = queue.sources.iter().flat_map(|named| {
        named
            .source
            .occurrences
            .iter()
            .map(|rec| (named.source.original, rec))
    });
    for (index, (original, rec)) in records.enumerate() {
        let entry = &queue.entries[index];
        if !rec.usable() {
            return Err(format!(
                "Cannot export damaged occurrence {}:{}. Inspect its source span in the review queue.",
                entry.label, entry.ordinal
            ));
        }
        let decision = &decisions[index];
        let representative = match decision.target() {
            Some(target) => target,
            None => {
                let raw = &original[rec.bytes.clone()];
                let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
                ris.push_str(&raw.replace("\r\n", "\n").replace('\r', "\n"));
                if !ris.ends_with('\n') {
                    ris.push('\n');
                }
                index
            }
        };
        // A pending or separately kept report may legitimately share this set's DOI.
        if let Some(ids) = proposed_sets.get_mut(&representative) {
            ids.observe(rec);
            if let Some(conflict) = ids.conflict() {
                let kind = match conflict {
                    Identifier::Doi => "DOI",
                    Identifier::Pmid => "PMID",
                };
                let target = &queue.entries[representative];
                return Err(format!(
                    "Conflicting {kind} values in the proposed duplicates for {}:{} at {}:{}. Review the decisions for this set.",
                    target.label, target.ordinal, entry.label, entry.ordinal
                ));
            }
        }
        writeln!(
            mapping,
            "map\t{}\t{}:{}\tbytes={}..{};output={}",
            decision.label(queue),
            entry.label,
            entry.ordinal,
            rec.bytes.start,
            rec.bytes.end,
            output_ordinals[representative]
        )
        .unwrap();
        if let Decision::VerifiedMetadata { rationale, .. } = decision {
            writeln!(
                mapping,
                "rationale\tmetadata\t{}:{}\t{rationale}",
                entry.label, entry.ordinal
            )
            .unwrap();
        }
    }
    writeln!(
        mapping,
        "output\tris\treviewed.ris\tbytes={};sha256={:x}",
        ris.len(),
        Sha256::digest(ris.as_bytes())
    )
    .unwrap();
    publish(directory, &ris, &mapping).map_err(|e| format!("Cannot create reviewed export: {e}"))
}

fn publish(directory: &Path, ris: &str, mapping: &str) -> io::Result<()> {
    let parent = directory
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let staging = tempfile::Builder::new()
        .prefix(".dedup-export-")
        .tempdir_in(parent)?;
    for (name, contents) in [("reviewed.ris", ris), ("mapping.tsv", mapping)] {
        let mut file = File::create_new(staging.path().join(name))?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }
    // Publish both files together. An existing or concurrently created destination wins.
    renamore::rename_exclusive(staging.path(), directory)?;
    let _ = staging.keep();
    Ok(())
}
