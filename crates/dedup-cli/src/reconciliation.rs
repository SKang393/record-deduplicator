//! Compare the counts represented in an export mapping with the supplied report.
use std::{collections::BTreeMap, io::Read, path::Path};

use dedup_core::{identifier_review::MAX_RECORDS, ris};
use sha2::{Digest, Sha256};

use super::mapping::INVALID;

pub(super) const HEADER: &str = "scope\tsource\tinput\tremoved_id\tremoved_metadata\tretained\tkeep\tpending_id\tpending_metadata\toutside_sheet_flagged\toutside_sheet_unflagged";
const BAD_REPORT: &str = "Invalid source or total row in reconciliation.tsv.";
type Columns = [usize; 9];

fn number(value: &str) -> Option<usize> {
    value.parse().ok().filter(|&n| n <= MAX_RECORDS)
}

fn occurrence(key: &str) -> Option<(&str, usize)> {
    let (label, ordinal) = key.split_once(':')?;
    if !ris::valid_source_label(label) || !ordinal.starts_with(|c: char| matches!(c, '1'..='9')) {
        return None;
    }
    Some((label, number(ordinal)?))
}

fn columns(row: &str) -> Option<Columns> {
    let mut fields = row.split('\t');
    let mut values = [0; 9];
    for value in &mut values {
        *value = number(fields.next()?)?;
    }
    fields.next().is_none().then_some(values)
}

fn check(mapping: &str, report: &str) -> Result<(), String> {
    let mut lines = report.lines();
    if lines.next() != Some(HEADER) {
        return Err("Unexpected reconciliation.tsv header.".into());
    }
    let totals = lines
        .next_back()
        .and_then(|l| l.strip_prefix("total\t\t"))
        .and_then(columns)
        .ok_or(BAD_REPORT)?;
    let mut sources = BTreeMap::new();
    let mut sums = [0; 9];
    for line in lines {
        let (label, fields) = line
            .strip_prefix("source\t")
            .and_then(|l| l.split_once('\t'))
            .ok_or(BAD_REPORT)?;
        let counts = columns(fields).ok_or(BAD_REPORT)?;
        if !ris::valid_source_label(label)
            || sources.len() >= ris::MAX_SOURCES
            || sources.insert(label, counts).is_some()
        {
            return Err(BAD_REPORT.into());
        }
        for (sum, value) in sums.iter_mut().zip(counts) {
            *sum += value;
        }
    }
    if sources.is_empty() || sums != totals {
        return Err(
            "The reconciliation total differs from its source rows or source rows are missing."
                .into(),
        );
    }

    // Exported map rows follow the input manifest, in original occurrence order.
    let mut records = mapping.lines().filter_map(|l| l.strip_prefix("map\t"));
    for manifest in mapping.lines().filter_map(|l| l.strip_prefix("input\t")) {
        let (label, identity) = manifest
            .strip_prefix("ris\t")
            .and_then(|s| s.split_once('\t'))
            .ok_or(INVALID)?;
        let declared = identity
            .split(';')
            .nth(1)
            .and_then(|s| s.strip_prefix("occurrences="))
            .and_then(number)
            .ok_or(INVALID)?;
        let counts = sources
            .remove(label)
            .ok_or("Source labels differ between reconciliation.tsv and mapping.tsv.")?;
        check_source(label, declared, counts, &mut records)?;
    }
    if !sources.is_empty() || records.next().is_some() {
        return Err("Source coverage differs between reconciliation.tsv and mapping.tsv.".into());
    }
    let [input, id, metadata, retained, ..] = totals;
    let expected = format!(
        "all\t{input}\tretained={retained};suppressed={}",
        id + metadata
    );
    let mut summaries = mapping.lines().filter_map(|l| l.strip_prefix("summary\t"));
    if summaries.next() != Some(expected.as_str()) || summaries.next().is_some() {
        return Err("The reconciliation total differs from the mapping summary.".into());
    }
    Ok(())
}

fn check_source<'a>(
    label: &str,
    declared: usize,
    counts: Columns,
    records: &mut impl Iterator<Item = &'a str>,
) -> Result<(), String> {
    let (mut id, mut metadata, mut keep, mut pending, mut outside) = (0, 0, 0, 0, 0);
    for ordinal in 1..=declared {
        let mut fields = records.next().ok_or(INVALID)?.splitn(3, '\t');
        let action = fields.next().ok_or(INVALID)?;
        if fields.next().and_then(occurrence) != Some((label, ordinal)) || fields.next().is_none() {
            return Err(INVALID.into());
        }
        let removal = action
            .split_once('=')
            .and_then(|(kind, key)| occurrence(key).map(|_| kind));
        match (action, removal) {
            ("keep", _) => keep += 1,
            ("pending", _) => pending += 1,
            ("not-candidate", _) => outside += 1,
            (_, Some("duplicate-of")) => id += 1,
            (_, Some("verified-duplicate-of")) => metadata += 1,
            _ => return Err(INVALID.into()),
        }
    }
    let [
        input,
        removed_id,
        removed_metadata,
        retained,
        kept,
        pending_id,
        pending_metadata,
        flagged,
        unflagged,
    ] = counts;
    for (name, reported, mapped) in [
        ("input", input, declared),
        ("removed_id", removed_id, id),
        ("removed_metadata", removed_metadata, metadata),
        ("retained", retained, keep + pending + outside),
        ("keep", kept, keep),
        ("combined pending", pending_id + pending_metadata, pending),
        ("combined outside-sheet", flagged + unflagged, outside),
    ] {
        if reported != mapped {
            return Err(format!(
                "Source {label} has {reported} {name} in reconciliation.tsv and {mapped} in mapping.tsv."
            ));
        }
    }
    Ok(())
}

pub(super) fn verify(mapping: &str, path: &Path) -> Result<String, String> {
    // One short numeric row per named input, plus the header and total.
    const MAX_BYTES: usize = (ris::MAX_SOURCES + 2) * 256;
    let mut report = String::new();
    super::open_ris(path.as_os_str())
        .map_err(|_| "Cannot open the supplied reconciliation.tsv as a regular file.")?
        .take(MAX_BYTES as u64 + 1)
        .read_to_string(&mut report)
        .map_err(|_| "Cannot read UTF-8 reconciliation.tsv.")?;
    if report.len() > MAX_BYTES {
        return Err("Reconciliation report exceeds the supported size.".into());
    }
    check(mapping, &report)?;
    Ok(format!("{:x}", Sha256::digest(report.as_bytes())))
}
