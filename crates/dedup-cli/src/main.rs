use dedup_core::identifier_review::{self as ids, Identifier, InputError, Issue, OtherEvidence};
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Read, Write},
    process::ExitCode,
};

use dedup_core::ris;

fn help() {
    println!(
        "record-deduplicator {}\n\nUsage: dedup --help | --version\n       dedup review-ids <input.tsv | ->\n       dedup review-ids --ris <input.ris | ->\n\nGroup rows by a shared full DOI or PMID for review. Keeps all records.\nInput: UTF-8, header doi<TAB>pmid, one occurrence per data line. With --ris, review TY/ER records and report original byte spans.\nOutput: TSV on standard output; occurrence numbers start at 1.\nNo fuzzy matching, merging, or record export.",
        dedup_core::VERSION
    );
}

fn render(report: &ids::Review, out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "type\tsignal\trows\tdetail")?;
    writeln!(
        out,
        "summary\tall\t{}\tretained={};suppressed=0;candidate_records={};groups={}",
        report.retained_records,
        report.retained_records,
        report.candidate_records,
        report.groups.len()
    )?;
    for group in &report.groups {
        let kind = match group.shared {
            Identifier::Doi => "doi",
            Identifier::Pmid => "pmid",
        };
        let detail = match group.other_evidence {
            OtherEvidence::Conflict => "other_id_conflict",
            OtherEvidence::Agreement => "other_id_agreement",
            OtherEvidence::Incomplete => "other_id_incomplete",
        };
        write!(out, "candidate\t{kind}\t")?;
        for (index, row) in group.rows.iter().enumerate() {
            if index > 0 {
                write!(out, ",")?;
            }
            write!(out, "{row}")?;
        }
        writeln!(out, "\t{detail}")?;
    }
    for &(ordinal, issue) in &report.issues {
        let label = match issue {
            Issue::ColumnCount => "wrong_column_count",
            Issue::UnsupportedDoi => "unsupported_doi",
            Issue::UnsupportedPmid => "unsupported_pmid",
            Issue::NoIdentifiers => "missing_identifiers",
            Issue::ConflictingDoi => "conflicting_dois_in_record",
            Issue::ConflictingPmid => "conflicting_pmids_in_record",
        };
        writeln!(out, "issue\tinput\t{ordinal}\t{label}")?;
    }
    out.flush()
}

fn same_bytes(mut reader: impl Read, original: &[u8]) -> io::Result<bool> {
    let mut buf = [0u8; 8192];
    let mut pos = 0;
    loop {
        let count = reader.read(&mut buf)?;
        if count == 0 {
            return Ok(pos == original.len());
        }
        if original.get(pos..pos + count) != Some(&buf[..count]) {
            return Ok(false);
        }
        pos += count;
    }
}

fn run(path: &OsStr, native_ris: bool) -> Result<(), &'static str> {
    let reader: Box<dyn Read> = if path == "-" {
        Box::new(io::stdin().lock())
    } else {
        let file = File::open(path).map_err(|_| "Cannot open input file.")?;
        if native_ris
            && !file
                .metadata()
                .map_err(|_| "Cannot inspect input.")?
                .is_file()
        {
            return Err("RIS input must be a regular file, or use standard input.");
        }
        Box::new(file)
    };
    let mut bytes = Vec::new();
    reader
        .take(ids::MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read input.")?;
    if bytes.len() > ids::MAX_INPUT_BYTES {
        return Err("Input exceeds the 16 MiB limit; no report produced.");
    }
    if native_ris && path != "-" {
        let file =
            File::open(path).map_err(|_| "Cannot recheck RIS source; no report produced.")?;
        if !same_bytes(file, &bytes)
            .map_err(|_| "Cannot recheck RIS source; no report produced.")?
        {
            return Err("RIS source changed while being read; no report produced.");
        }
    }
    let input =
        std::str::from_utf8(&bytes).map_err(|_| "Input is not UTF-8; no report produced.")?;
    let explain = |error| match error {
        InputError::ByteLimit => "Input exceeds the 16 MiB limit; no report produced.",
        InputError::RecordLimit => "Input exceeds 100000 occurrences; no report produced.",
        InputError::Header => "Expected the UTF-8 header doi<TAB>pmid; no report produced.",
        InputError::FieldLimit => "Input exceeds 250000 RIS fields; no report produced.",
    };
    let mut out = io::BufWriter::new(io::stdout().lock());
    let write_error = |_: io::Error| "Cannot write report.";
    if native_ris {
        let source = ris::parse(input).map_err(explain)?;
        let report = source.review();
        render(&report, &mut out).map_err(write_error)?;
        for (index, rec) in source.occurrences.iter().enumerate() {
            let status = if !rec.has_type {
                "fragment"
            } else if !rec.has_end {
                "partial"
            } else if rec.malformed_lines > 0 {
                "malformed"
            } else {
                "complete"
            };
            writeln!(out,"source\tris\t{}\tbytes={}..{};status={};closed={};malformed_lines={};fields={};unmapped_fields={}",index+1,
                rec.bytes.start,rec.bytes.end,status,rec.has_end,rec.malformed_lines,rec.fields.len(),rec.unmapped_fields()).map_err(write_error)?;
        }
    } else {
        let report = ids::review_tsv(input).map_err(explain)?;
        render(&report, &mut out).map_err(write_error)?;
    }
    out.flush().map_err(write_error)
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h")) {
        help();
        return ExitCode::SUCCESS;
    }
    if args.len() == 1 && (args[0] == "--version" || args[0] == "-V") {
        println!("dedup {}", dedup_core::VERSION);
        return ExitCode::SUCCESS;
    }
    let native_ris = args.len() == 3 && args[0] == "review-ids" && args[1] == "--ris";
    if native_ris || (args.len() == 2 && args[0] == "review-ids") {
        return match run(&args[if native_ris { 2 } else { 1 }], native_ris) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(2)
            }
        };
    }
    eprintln!("Unsupported arguments. See dedup --help.");
    ExitCode::from(2)
}

#[cfg(test)]
mod source_checks {
    use super::same_bytes;
    #[test]
    fn recheck_detects_replaced_shorter_and_longer_content() {
        assert!(same_bytes(&b"abc"[..], b"abc").unwrap());
        for changed in [b"ab".as_slice(), b"abcd", b"axc"] {
            assert!(!same_bytes(changed, b"abc").unwrap());
        }
    }
}
