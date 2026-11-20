mod decisions;
mod inspect;
mod pending;
mod reviewed;
mod sources;

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
        "record-deduplicator {}\n\nUsage: dedup --help | --version\n       dedup review-ids <input.tsv | ->\n       dedup review-ids --ris <input.ris | ->\n       dedup review-ids [--init-decisions <new.tsv> | --init-metadata-decisions <new.tsv> | --resume-decisions <saved.tsv> | --pending-decisions <saved.tsv> | --inspect-pair <saved.tsv> <label:n> <label:n> | --export-reviewed <saved.tsv> <new-dir>] --ris-source <label> <file> [--ris-source <label> <file> ...]\n\nGroup rows by a shared full DOI or PMID for review. Named RIS queues also show metadata inspection hints for records without usable identifiers.\nInput: UTF-8, header doi<TAB>pmid, one occurrence per data line. With --ris, review TY/ER records and report original byte spans.\nQueue output: TSV on standard output; occurrence numbers start at 1. Use --pending-decisions to view unresolved groups with their direct kept members. Use --inspect-pair for a plain-text view of two RIS records and their current decisions.\nFor metadata verification, create a sheet with --init-metadata-decisions. Mark the target keep and enter verified-duplicate-of=<label:row><TAB><rationale> on the copy after comparing the source reports.\nReviewed export: apply the verified sheet decisions and write reviewed.ris, mapping.tsv and per-source reconciliation.tsv in a new directory.",
        dedup_core::VERSION
    );
}

fn group_labels(group: &ids::Group) -> (&'static str, &'static str) {
    let kind = match group.shared {
        Identifier::Doi => "doi",
        Identifier::Pmid => "pmid",
    };
    let detail = match group.other_evidence {
        OtherEvidence::Conflict => "other_id_conflict",
        OtherEvidence::Agreement => "other_id_agreement",
        OtherEvidence::Incomplete => "other_id_incomplete",
    };
    (kind, detail)
}

fn render<T: std::fmt::Display>(
    report: &ids::Review,
    out: &mut impl Write,
    row_ref: impl Fn(usize) -> T,
) -> io::Result<()> {
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
        let (kind, detail) = group_labels(group);
        write!(out, "candidate\t{kind}\t")?;
        for (index, row) in group.rows.iter().enumerate() {
            if index > 0 {
                write!(out, ",")?;
            }
            write!(out, "{}", row_ref(*row))?;
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
        writeln!(out, "issue\tinput\t{}\t{label}", row_ref(ordinal))?;
    }
    Ok(())
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

fn open_ris(path: &OsStr) -> Result<File, &'static str> {
    if !std::fs::metadata(path)
        .map_err(|_| "Cannot inspect RIS input.")?
        .is_file()
    {
        return Err("RIS input must be a regular file.");
    }
    let file = File::open(path).map_err(|_| "Cannot open RIS input.")?;
    if !file
        .metadata()
        .map_err(|_| "Cannot inspect RIS input.")?
        .is_file()
    {
        return Err("RIS input must be a regular file.");
    }
    Ok(file)
}

fn read_input(
    path: &OsStr,
    native_ris: bool,
    remaining_bytes: usize,
) -> Result<String, &'static str> {
    let reader: Box<dyn Read> = if path == "-" {
        Box::new(io::stdin().lock())
    } else if native_ris {
        Box::new(open_ris(path)?)
    } else {
        Box::new(File::open(path).map_err(|_| "Cannot open input file.")?)
    };
    let mut bytes = Vec::new();
    reader
        .take(remaining_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read input.")?;
    if bytes.len() > remaining_bytes {
        return Err(input_error(InputError::ByteLimit));
    }
    String::from_utf8(bytes).map_err(|_| "Input is not UTF-8; no report produced.")
}

fn recheck(path: &OsStr, original: &[u8]) -> Result<(), &'static str> {
    let file = open_ris(path)?;
    if !same_bytes(file, original).map_err(|_| "Cannot recheck RIS source; no report produced.")? {
        return Err("RIS source changed while being read; no report produced.");
    }
    Ok(())
}

fn input_error(error: InputError) -> &'static str {
    match error {
        InputError::ByteLimit => "Input exceeds the 16 MiB limit; no report produced.",
        InputError::RecordLimit => "Input exceeds 100000 occurrences; no report produced.",
        InputError::Header => "Expected the UTF-8 header doi<TAB>pmid; no report produced.",
        InputError::FieldLimit => "Input exceeds 250000 RIS fields; no report produced.",
    }
}

fn render_occurrence(
    out: &mut impl Write,
    row: impl std::fmt::Display,
    rec: &ris::Occurrence<'_>,
    disposition: Option<&str>,
) -> io::Result<()> {
    let status = if !rec.has_type {
        "fragment"
    } else if !rec.has_end {
        "partial"
    } else if rec.malformed_lines > 0 {
        "malformed"
    } else {
        "complete"
    };
    write!(
        out,
        "source\tris\t{row}\tbytes={}..{};status={status};closed={};malformed_lines={};fields={};unmapped_fields={}",
        rec.bytes.start,
        rec.bytes.end,
        rec.has_end,
        rec.malformed_lines,
        rec.fields.len(),
        rec.unmapped_fields()
    )?;
    if let Some(label) = disposition {
        write!(out, ";disposition={label}")?;
    }
    writeln!(out)
}

fn run(path: &OsStr, native_ris: bool) -> Result<(), &'static str> {
    let input = read_input(path, native_ris, ids::MAX_INPUT_BYTES)?;
    let mut out = io::BufWriter::new(io::stdout().lock());
    let write_error = |_: io::Error| "Cannot write report.";
    if native_ris {
        let source = ris::parse(&input).map_err(input_error)?;
        if path != "-" {
            recheck(path, input.as_bytes())?;
        }
        render(&source.review(), &mut out, |row| row).map_err(write_error)?;
        for (index, rec) in source.occurrences.iter().enumerate() {
            render_occurrence(&mut out, index + 1, rec, None).map_err(write_error)?;
        }
    } else {
        let report = ids::review_tsv(&input).map_err(input_error)?;
        render(&report, &mut out, |row| row).map_err(write_error)?;
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
    let result = if args.len() >= 2
        && args[0] == "review-ids"
        && matches!(
            args[1].to_str(),
            Some(
                "--ris-source"
                    | "--init-decisions"
                    | "--init-metadata-decisions"
                    | "--resume-decisions"
                    | "--pending-decisions"
                    | "--inspect-pair"
                    | "--export-reviewed"
            )
        ) {
        sources::run(&args[1..])
    } else if native_ris || (args.len() == 2 && args[0] == "review-ids") {
        run(&args[if native_ris { 2 } else { 1 }], native_ris).map_err(String::from)
    } else {
        Err("Unsupported arguments. See dedup --help.".into())
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
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
