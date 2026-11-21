//! Read the published mapping shared by trace and reviewed RIS verification.
use std::{
    io::{self, Read, Write},
    path::Path,
};

use dedup_core::{identifier_review::MAX_RECORDS, ris};
use sha2::{Digest, Sha256};

pub(super) const INVALID: &str = "Invalid reviewed export mapping.";
pub(super) const RATIONALE_SCHEMA: &str = "schema\texport\t2\treviewed-ris";

// Each exported occurrence fits within 1 KiB including a possible rationale row.
const MAX_MAPPING_BYTES: usize = (MAX_RECORDS + ris::MAX_SOURCES + 4) * 1024;

pub(super) fn read(mapping: &Path) -> Result<String, String> {
    let mut text = String::new();
    super::open_ris(mapping.as_os_str())
        .map_err(|_| "Cannot open mapping.tsv as a regular file.")?
        .take(MAX_MAPPING_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|_| "Cannot read UTF-8 mapping.tsv.")?;
    let mut header = text.lines();
    if text.len() > MAX_MAPPING_BYTES
        || header.next() != Some("type\tsignal\trows\tdetail")
        || !matches!(
            header.next(),
            Some("schema\texport\t1\treviewed-ris" | RATIONALE_SCHEMA)
        )
    {
        return Err(INVALID.into());
    }
    Ok(text)
}

fn output_fingerprint(mapping: &str) -> Result<(u64, &str), &'static str> {
    let mut outputs = mapping
        .lines()
        .filter(|line| line.split('\t').next() == Some("output"));
    let row = outputs
        .next()
        .ok_or("The mapping has no reviewed RIS output fingerprint.")?;
    if outputs.next().is_some() {
        return Err(
            "The mapping has multiple output rows. Supply the original mapping from the export.",
        );
    }
    let invalid = "Expected a reviewed.ris byte length and SHA-256 in the mapping output row.";
    let (bytes, digest) = row
        .strip_prefix("output\tris\treviewed.ris\tbytes=")
        .and_then(|detail| detail.split_once(";sha256="))
        .ok_or(invalid)?;
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid);
    }
    Ok((bytes.parse().map_err(|_| invalid)?, digest))
}

pub(super) fn verify_output(
    mapping: &Path,
    reviewed_ris: &Path,
    reconciliation: Option<&Path>,
) -> Result<(), String> {
    let text = read(mapping)?;
    let (expected_len, expected_hash) = output_fingerprint(&text)?;
    let file = super::open_ris(reviewed_ris.as_os_str())
        .map_err(|_| "Cannot open the supplied reviewed RIS as a regular file.")?;
    let mut hasher = Sha256::new();
    let length = io::copy(&mut file.take(expected_len.saturating_add(1)), &mut hasher)
        .map_err(|_| "Cannot read the supplied reviewed RIS.")?;
    if length != expected_len {
        return Err("Reviewed RIS byte length differs from the mapping output row.".into());
    }
    let digest = format!("{:x}", hasher.finalize());
    if !digest.eq_ignore_ascii_case(expected_hash) {
        return Err("Reviewed RIS SHA-256 differs from the mapping output row.".into());
    }
    let (extra_columns, extra_values) = match reconciliation {
        Some(path) => {
            let hash = super::reconciliation::verify(&text, path)?;
            (
                "\treconciliation_path\treconciliation_sha256\tcount_check",
                format!("\t{path:?}\t{hash}\tmapped_disposition_counts_match"),
            )
        }
        None => ("", String::new()),
    };
    writeln!(
        io::stdout().lock(),
        "result\tmapping_path\tmapping_sha256\treviewed_ris_path\treviewed_ris_bytes\treviewed_ris_sha256{extra_columns}\nmatched_output_fingerprint\t{mapping:?}\t{:x}\t{reviewed_ris:?}\t{length}\t{digest}{extra_values}",
        Sha256::digest(text.as_bytes())
    ).map_err(|_| "Cannot write verification result.".into())
}
