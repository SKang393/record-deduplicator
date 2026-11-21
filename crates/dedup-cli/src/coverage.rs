use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io::{self, Write},
};

use dedup_core::ris::{NamedSource, valid_source_label};

const USAGE: &str = "Use --check-coverage <snapshot> with --expected-total <count> or --batch-range <label> <first-last>, then --ris-source <label> <file> for each available batch.";

pub(super) struct Plan<'a> {
    snapshot: &'a str,
    expected_total: Option<u64>,
    ranges: BTreeMap<&'a str, (u64, u64)>,
}

fn label(arg: &OsStr) -> Result<&str, &'static str> {
    arg.to_str()
        .filter(|s| valid_source_label(s))
        .ok_or("Use RIS source label syntax for snapshot and batch names.")
}

fn number(s: &str) -> Result<u64, &'static str> {
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(USAGE);
    }
    s.parse().map_err(|_| USAGE)
}

impl<'a> Plan<'a> {
    pub(super) fn parse(args: &'a [OsString]) -> Result<(Self, &'a [OsString]), &'static str> {
        let (snapshot, mut args) = args.split_first().ok_or(USAGE)?;
        let mut plan = Self {
            snapshot: label(snapshot)?,
            expected_total: None,
            ranges: BTreeMap::new(),
        };
        while let Some(flag) = args.first() {
            match args {
                [_, n, rest @ ..] if flag == "--expected-total" => {
                    if plan.expected_total.is_some() {
                        return Err("Supply one expected total for this snapshot.");
                    }
                    plan.expected_total = Some(number(n.to_str().ok_or(USAGE)?)?);
                    args = rest;
                }
                [_, name, span, rest @ ..] if flag == "--batch-range" => {
                    let (first, last) =
                        span.to_str().and_then(|s| s.split_once('-')).ok_or(USAGE)?;
                    let (first, last) = (number(first)?, number(last)?);
                    if first == 0 || last < first {
                        return Err(
                            "Batch ranges are one-based and inclusive. The last position must be at least the first.",
                        );
                    }
                    if plan.ranges.insert(label(name)?, (first, last)).is_some() {
                        return Err("Each batch needs its own source label.");
                    }
                    args = rest;
                }
                _ if flag == "--ris-source" => break,
                _ => return Err(USAGE),
            }
        }
        if plan.expected_total.is_none() && plan.ranges.is_empty() {
            return Err(USAGE);
        }
        Ok((plan, args))
    }

    pub(super) fn check(
        &self,
        sources: &[NamedSource<'_>],
        manifest: &[String],
    ) -> Result<(), String> {
        let mut counts: BTreeMap<_, _> = sources
            .iter()
            .map(|s| (s.label, s.source.occurrences.len() as u64))
            .collect();
        let parsed: u64 = counts.values().sum();
        let expected = self
            .expected_total
            .map_or_else(|| "unspecified".into(), |n| n.to_string());
        let mut report = vec![format!(
            "snapshot\t{}\tparsed={parsed}\texpected_total={expected}",
            self.snapshot
        )];
        report.extend_from_slice(manifest);
        let mut issues = Vec::new();
        if self.expected_total.is_some_and(|n| n != parsed) {
            issues.push(format!(
                "total_count_mismatch\texpected={expected}\tparsed={parsed}"
            ));
        }

        let mut batches: Vec<_> = self
            .ranges
            .iter()
            .map(|(&name, &(first, last))| (first, last, name))
            .collect();
        batches.sort_unstable();
        // With a known total the expected window starts at position 1. Without
        // it only the declared window, including gaps between ranges, is known.
        let mut through = self.expected_total.map(|_| 0);
        for (first, last, name) in batches {
            report.push(format!("batch_range\t{name}\t{first}-{last}"));
            let width = last - first + 1;
            match counts.remove(name) {
                None => issues.push(format!("missing_batch\t{name}\tpositions={first}-{last}")),
                Some(n) if n != width => issues.push(format!(
                    "batch_count_mismatch\t{name}\texpected={width}\tparsed={n}"
                )),
                _ => {}
            }
            if let Some(end) = through {
                let gap_end = (first - 1).min(self.expected_total.unwrap_or(u64::MAX));
                if end < gap_end {
                    issues.push(format!("range_gap\tpositions={}-{gap_end}", end + 1));
                } else if first <= end {
                    issues.push(format!(
                        "range_overlap\t{name}\tpositions={first}-{}",
                        last.min(end)
                    ));
                }
            }
            through = Some(last.max(through.unwrap_or(0)));
            if self.expected_total.is_some_and(|n| last > n) {
                issues.push(format!("range_exceeds_total\t{name}\tlast={last}"));
            }
        }
        if !self.ranges.is_empty() {
            if let (Some(end), Some(total)) = (through, self.expected_total) {
                if end < total {
                    issues.push(format!("range_gap\tpositions={}-{total}", end + 1));
                }
            }
            for name in counts.keys() {
                issues.push(format!("unassigned_source\t{name}"));
            }
        }
        let matched = issues.is_empty();
        report.extend(issues.into_iter().map(|issue| format!("issue\t{issue}")));
        let status = if matched {
            "matched_expectations"
        } else {
            "discrepancies"
        };
        report.push(format!("result\t{}\t{status}", self.snapshot));
        writeln!(io::stdout().lock(), "{}", report.join("\n"))
            .map_err(|_| "Cannot write coverage report.")?;
        if matched {
            Ok(())
        } else {
            Err("Source coverage discrepancies. Check the snapshot's batch log.".into())
        }
    }
}
