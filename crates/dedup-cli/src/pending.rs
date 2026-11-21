//! Read-only projection of the direct evidence groups in a reopened sheet.
use std::io::{self, Write};

use dedup_core::{
    identifier_review::{Issue, OtherEvidence},
    ris::{MetadataHint, ReviewQueue},
};

use crate::decisions::{Decision, ResumedSheet};

pub(super) fn render(
    queue: &ReviewQueue<'_>,
    manifest: &[String],
    sheet: &ResumedSheet,
    out: &mut impl Write,
) -> io::Result<Option<String>> {
    let mut conflict_in_record = vec![false; queue.entries.len()];
    for &(row, issue) in &queue.review.issues {
        if matches!(issue, Issue::ConflictingDoi | Issue::ConflictingPmid) {
            conflict_in_record[row - 1] = true;
        }
    }
    // Each membership appears once per direct group, even with many keepers.
    let mut conflicts = Vec::new();
    let mut remaining = Vec::new();
    let mut add = |members: &[usize], signal: &str, evidence: &str, conflicting_group: bool| {
        if !members
            .iter()
            .any(|&row| sheet.decisions[row - 1] == Decision::Pending)
        {
            return;
        }
        let mut pending = Vec::new();
        let mut keep = Vec::new();
        let mut assigned = Vec::new();
        let mut conflicting_members = Vec::new();
        for &row in members {
            let entry = &queue.entries[row - 1];
            let key = format!("{}:{}", entry.label, entry.ordinal);
            if conflict_in_record[row - 1] {
                conflicting_members.push(key.clone());
            }
            match sheet.decisions[row - 1] {
                Decision::Pending => pending.push(key),
                Decision::Keep(_) => keep.push(key),
                Decision::DuplicateOf(_) | Decision::VerifiedMetadata { .. } => assigned.push(key),
                Decision::NotCandidate => {}
            }
        }
        let mut line = format!(
            "pending\t{signal}\t{}\t{evidence};keep={};assigned={}",
            pending.join(","),
            keep.join(","),
            assigned.join(",")
        );
        if !conflicting_members.is_empty() {
            line.push_str(";conflicting_ids_in=");
            line.push_str(&conflicting_members.join(","));
        }
        if conflicting_group || !conflicting_members.is_empty() {
            conflicts.push(line);
        } else {
            remaining.push(line);
        }
    };
    for group in &queue.review.groups {
        let (signal, evidence) = crate::group_labels(group);
        add(
            &group.rows,
            signal,
            evidence,
            group.other_evidence == OtherEvidence::Conflict,
        );
    }
    if let Some(hints) = &sheet.metadata_hints {
        for hint in hints {
            if let MetadataHint::Inspect(rows) = hint {
                add(
                    rows,
                    "title_author_year_type",
                    "inspect_metadata_only",
                    false,
                );
            }
        }
    }

    let count = sheet
        .decisions
        .iter()
        .filter(|d| **d == Decision::Pending)
        .count();
    writeln!(out, "type\tsignal\trows\tdetail")?;
    writeln!(
        out,
        "summary\tpending\t{count}\tgroups={};flagged_groups={}",
        conflicts.len() + remaining.len(),
        conflicts.len()
    )?;
    for identity in manifest {
        writeln!(out, "{identity}")?;
    }
    for line in conflicts.iter().chain(&remaining) {
        writeln!(out, "{line}")?;
    }
    // A lone group is already next to the prompt. Otherwise return the same
    // first row we just printed, including its flags and all member keys.
    Ok(if conflicts.len() + remaining.len() > 1 {
        conflicts.into_iter().chain(remaining).next()
    } else {
        None
    })
}
