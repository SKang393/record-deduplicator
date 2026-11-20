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
    ris::{Disposition, ReportIdentifiers, ReviewQueue},
};
use sha2::{Digest, Sha256};

use super::decisions::Decision;

pub(super) struct PreparedExport {
    ris: String,
    mapping: String,
    reconciliation: String,
    pub retained: usize,
    pub removed: usize,
    pub pending: usize,
}

pub(super) fn prepare(
    queue: &ReviewQueue<'_>,
    manifest: &[String],
    decisions: &[Decision],
) -> Result<PreparedExport, String> {
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
    let (reconciliation, counts) = reconciliation(queue, decisions);
    Ok(PreparedExport {
        ris,
        mapping,
        reconciliation,
        retained,
        removed: total - retained,
        pending: counts.pending_id + counts.pending_metadata,
    })
}

#[derive(Default)]
struct Counts {
    removed_id: usize,
    removed_metadata: usize,
    keep: usize,
    pending_id: usize,
    pending_metadata: usize,
    outside_flagged: usize,
    outside_unflagged: usize,
}

impl Counts {
    fn record(&mut self, decision: &Decision, disposition: Disposition) {
        let count = match decision {
            Decision::DuplicateOf(_) => &mut self.removed_id,
            Decision::VerifiedMetadata { .. } => &mut self.removed_metadata,
            Decision::Keep => &mut self.keep,
            Decision::Pending if disposition == Disposition::ReviewCandidate => {
                &mut self.pending_id
            }
            Decision::Pending => &mut self.pending_metadata,
            Decision::NotCandidate if disposition == Disposition::ReviewInput => {
                &mut self.outside_flagged
            }
            Decision::NotCandidate => &mut self.outside_unflagged,
        };
        *count += 1;
    }

    fn write_row(&self, table: &mut String, scope: &str, source: &str, input: usize) {
        let retained = self.keep
            + self.pending_id
            + self.pending_metadata
            + self.outside_flagged
            + self.outside_unflagged;
        writeln!(
            table,
            "{scope}\t{source}\t{input}\t{}\t{}\t{retained}\t{}\t{}\t{}\t{}\t{}",
            self.removed_id,
            self.removed_metadata,
            self.keep,
            self.pending_id,
            self.pending_metadata,
            self.outside_flagged,
            self.outside_unflagged
        )
        .unwrap();
    }
}

fn reconciliation(queue: &ReviewQueue<'_>, decisions: &[Decision]) -> (String, Counts) {
    let mut table = String::from(
        "scope\tsource\tinput\tremoved_id\tremoved_metadata\tretained\tkeep\tpending_id\tpending_metadata\toutside_sheet_flagged\toutside_sheet_unflagged\n",
    );
    let mut total = Counts::default();
    let mut entries = queue.entries.iter().zip(decisions);
    for named in &queue.sources {
        let mut source = Counts::default();
        for (entry, decision) in entries.by_ref().take(named.source.occurrences.len()) {
            source.record(decision, entry.disposition);
            total.record(decision, entry.disposition);
        }
        source.write_row(
            &mut table,
            "source",
            named.label,
            named.source.occurrences.len(),
        );
    }
    total.write_row(&mut table, "total", "", queue.entries.len());
    (table, total)
}

pub(super) fn require_new_destination(directory: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(directory) {
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Destination already exists. Choose a new folder name.",
        )),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

impl PreparedExport {
    pub(super) fn publish(self, directory: &Path) -> Result<(), String> {
        self.write_new_directory(directory)
            .map_err(|e| format!("Cannot create reviewed export: {e}"))
    }

    fn write_new_directory(self, directory: &Path) -> io::Result<()> {
        require_new_destination(directory)?;
        let parent = directory
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let staging = tempfile::Builder::new()
            .prefix(".dedup-export-")
            .tempdir_in(parent)?;
        for (name, contents) in [
            ("reviewed.ris", self.ris),
            ("mapping.tsv", self.mapping),
            ("reconciliation.tsv", self.reconciliation),
        ] {
            let mut file = File::create_new(staging.path().join(name))?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
        }
        // Publish all files together. An existing or concurrently created destination wins.
        renamore::rename_exclusive(staging.path(), directory)?;
        let _ = staging.keep();
        Ok(())
    }
}
