use dedup_core::identifier_review::{self as ids, Identifier, InputError, Issue, OtherEvidence};
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Read, Write},
    process::ExitCode,
};

fn help() {
    println!(
        "record-deduplicator {}\n\nUsage: dedup --help | --version\n       dedup review-ids <input.tsv | ->\n\nGroup rows by a shared full DOI or PMID for review. Keeps all records.\nInput: UTF-8, header doi<TAB>pmid, one occurrence per data line.\nOutput: TSV on standard output; row numbers start at 1 after the header.\nNo fuzzy matching, merging, or record export.",
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
        };
        writeln!(out, "issue\tinput\t{ordinal}\t{label}")?;
    }
    out.flush()
}

fn run(path: &OsStr) -> Result<(), &'static str> {
    let reader: Box<dyn Read> = if path == "-" {
        Box::new(io::stdin().lock())
    } else {
        Box::new(File::open(path).map_err(|_| "Cannot open input file.")?)
    };
    let mut bytes = Vec::new();
    reader
        .take(ids::MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read input.")?;
    if bytes.len() > ids::MAX_INPUT_BYTES {
        return Err("Input exceeds the 16 MiB limit; no report produced.");
    }
    let input =
        std::str::from_utf8(&bytes).map_err(|_| "Input is not UTF-8; no report produced.")?;
    let report = ids::review_tsv(input).map_err(|error| match error {
        InputError::ByteLimit => "Input exceeds the 16 MiB limit; no report produced.",
        InputError::RecordLimit => "Input exceeds 100000 records; no report produced.",
        InputError::Header => "Expected the UTF-8 header doi<TAB>pmid; no report produced.",
    })?;
    render(&report, &mut io::BufWriter::new(io::stdout().lock()))
        .map_err(|_| "Cannot write report.")
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
    if args.len() == 2 && args[0] == "review-ids" {
        return match run(&args[1]) {
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
