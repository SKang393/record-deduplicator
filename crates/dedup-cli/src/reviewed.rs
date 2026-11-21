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
    ris::{Disposition, Occurrence, ReportIdentifiers, ReviewQueue},
};
use sha2::{Digest, Sha256};

use super::decisions::Decision;

pub(super) struct PreparedExport {
    ris: String,
    asreview: Option<String>,
    mapping: String,
    reconciliation: String,
    pub retained: usize,
    pub removed: usize,
    pub pending: usize,
}

fn append_record(output: &mut String, raw: &str) {
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    output.push_str(&raw.replace("\r\n", "\n").replace('\r', "\n"));
    if !output.ends_with('\n') {
        output.push('\n');
    }
}

// ASReview selects title and abstract columns across the dataset. Use one
// spelling per field in its companion RIS while preserving each record's values.
fn asreview_record(raw: &str, rec: &Occurrence<'_>) -> Result<String, &'static str> {
    let mut result = String::with_capacity(raw.len());
    let mut copied = 0;
    let mut title_seen = false;
    let mut abstract_seen = false;
    for field in &rec.fields {
        let tag = match field.tag {
            "TI" | "T1" if title_seen => return Err("title (TI/T1)"),
            "AB" | "N2" if abstract_seen => return Err("abstract (AB/N2)"),
            "TI" | "T1" => {
                title_seen = true;
                "TI"
            }
            "AB" | "N2" => {
                abstract_seen = true;
                "AB"
            }
            other => other,
        };
        let mut start = field.bytes.start - rec.bytes.start;
        if start == 0 && raw.starts_with('\u{feff}') {
            start += '\u{feff}'.len_utf8();
        }
        let after_dash = start + 5; // parsed two-byte tag and "  -"
        let separator = raw.as_bytes().get(after_dash);
        if field.tag == tag && (tag == "ER" || separator == Some(&b' ')) {
            continue;
        }
        result.push_str(&raw[copied..start]);
        result.push_str(tag);
        result.push_str("  - ");
        copied = after_dash + usize::from(matches!(separator, Some(b' ' | b'\t')));
    }
    result.push_str(&raw[copied..]);
    Ok(result)
}

pub(super) fn prepare(
    queue: &ReviewQueue<'_>,
    manifest: &[String],
    decisions: &[Decision],
    include_asreview: bool,
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
    let schema = if decisions.iter().any(|d| d.mapping_rationale().is_some()) {
        super::mapping::RATIONALE_SCHEMA
    } else {
        "schema\texport\t1\treviewed-ris"
    };
    let mut mapping = format!("type\tsignal\trows\tdetail\n{schema}\n");
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
    let mut asreview = include_asreview.then(String::new);
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
                append_record(&mut ris, raw);
                if let Some(copy) = &mut asreview {
                    let record = asreview_record(raw, rec).map_err(|field| format!(
                        "Cannot prepare asreview.ris. Retained occurrence {}:{} has multiple {field} fields. Use the ordinary reviewed export to preserve all values.",
                        entry.label, entry.ordinal
                    ))?;
                    append_record(copy, &record);
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
        if let Some((kind, rationale)) = decision.mapping_rationale() {
            writeln!(
                mapping,
                "rationale\t{kind}\t{}:{}\t{rationale}",
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
        asreview,
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
            Decision::Keep(_) => &mut self.keep,
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
    let mut table = format!("{}\n", super::reconciliation::HEADER);
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
        self.write_files(staging.path())?;
        // Publish all files together. An existing or concurrently created destination wins.
        renamore::rename_exclusive(staging.path(), directory)?;
        let _ = staging.keep();
        Ok(())
    }

    fn write_files(self, staging: &Path) -> io::Result<()> {
        let mut files = Vec::new();
        if let Some(copy) = self.asreview {
            let receipt = format!(
                "{:x} *reviewed.ris\n{:x} *asreview.ris\n",
                Sha256::digest(self.ris.as_bytes()),
                Sha256::digest(copy.as_bytes())
            );
            files.extend([("asreview.ris", copy), ("asreview.sha256", receipt)]);
        }
        files.extend([
            ("reviewed.ris", self.ris),
            ("reconciliation.tsv", self.reconciliation),
            // An interrupted staging directory should not contain a verifiable RIS/mapping
            // pair while the reconciliation is still missing or incomplete.
            ("mapping.tsv", self.mapping),
        ]);
        for (name, contents) in files {
            let mut file = File::create_new(staging.join(name))?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn failed_reconciliation_write_leaves_no_mapping_in_staging() {
        let staging = tempfile::tempdir().unwrap();
        // Prevent exclusive creation of the counts file on any supported filesystem.
        let counts_path = staging.path().join("reconciliation.tsv");
        fs::write(&counts_path, b"occupied").unwrap();
        let export = PreparedExport {
            ris: "TY  - JOUR\nER  -\n".into(),
            asreview: None,
            mapping: String::new(),
            reconciliation: String::new(),
            retained: 1,
            removed: 0,
            pending: 0,
        };

        let error = export.write_files(staging.path()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            fs::read_to_string(staging.path().join("reviewed.ris")).unwrap(),
            "TY  - JOUR\nER  -\n"
        );
        assert_eq!(fs::read(counts_path).unwrap(), b"occupied");
        assert!(!staging.path().join("mapping.tsv").exists());
    }
}
