# Record Deduplicator

[![CI](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)
[![Windows build checks](https://img.shields.io/badge/Windows-build%20checks-0078D4)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)
[![macOS build checks](https://img.shields.io/badge/macOS-build%20checks-555555)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)

An offline bibliographic record deduplication project for systematic reviews and
meta-analyses, built around a shared Rust engine. The goal is to identify duplicate
database records while preserving distinct reports, original metadata, and the
reasoning behind each decision.

**In development:** the CLI can review shared full DOIs and PMIDs directly from
a UTF-8 RIS export or a two-column TSV file. It produces a review report, not
a deduplicated export. Automated merging and the desktop application are not
available on this branch. There are no prebuilt release downloads.

## Features and design

The planned workflow combines:

- **Multiple input formats:** RIS, PubMed NBIB, BibTeX, EndNote XML, CSV/TSV, and
  Web of Science tagged records.
- **Deterministic and fuzzy matching:** normalized identifiers and bibliographic
  fields, multiple candidate-generation keys, and string-similarity comparisons.
- **Protection for distinct reports:** safeguards for protocols, corrections,
  conference abstracts, preprints, and other related publications.
- **Auditable decisions:** preserved source records, field-level provenance,
  reviewable matches, and reversible decisions.
- **Reproducible output:** versioned policies, retained-record exports, and counts
  that can be traced back to the original inputs.
- **Local processing:** one Rust engine shared by a CLI and a planned Tauri
  desktop interface, with no upload required for deduplication.

See the [engine contract](docs/10-engine-contract.md) for the technical rules.

## Build from source

### Prerequisites

- Git.
- A current stable [Rust toolchain](https://rustup.rs/) with Cargo. The workspace
  declares Rust 1.85 or newer.
- Native build tools: the Visual Studio C++ build tools on Windows, or Xcode
  Command Line Tools on macOS (`xcode-select --install`). See the
  [Rust installation guide](https://doc.rust-lang.org/book/ch01-01-installation.html)
  for platform setup.

Clone the repository and build the current CLI:

```sh
git clone https://github.com/SKang393/record-deduplicator.git
cd record-deduplicator
cargo build --release --locked -p dedup-cli
```

The executable is `target/release/dedup.exe` on Windows and
`target/release/dedup` on macOS. Python and Node.js are not required for this CLI
build.

## Usage

Use `--help` for syntax or `--version` for the build version.

### Review an RIS export

```sh
cargo run --release --locked -p dedup-cli -- review-ids --ris records.ris
```

This command reads records beginning with `TY  -` and ending with `ER  -`.
It uses the DOI field (`DO`) and DOI-resolver or PubMed article URLs in `UR`.
Numeric accession or local record IDs (`AN`, `ID`) are not inferred to be PMIDs.
A repeated equivalent identifier counts once. Unsupported or contradictory
values within a record withhold that identifier from matching. A usable ID of
the other kind can still lead to a review group, but never an automatic merge.

The report gives each record or nonblank stray fragment an occurrence number.
Each `source` line points to the original bytes: the start is inclusive and
the end is exclusive. It also reports record status, field count, and the number
of fields not used for identifier matching. Ordered and repeated fields,
indented continuations, unrecognized tags, and original line endings remain
in the source. No record is reserialized. Keep the original export; the report
is not a backup or persistent project bundle.

A missing end tag, unexpected untagged line, or fragment is reported and kept
out of matching. The retained count includes those occurrences. Files are read
without modification and rechecked against the captured bytes before review.
Use `--ris -` for standard input. Non-UTF-8 input or a limit violation (16 MiB,
100,000 occurrences, or 250,000 fields) produces no partial report.

### Review a two-column TSV projection

```sh
cargo run --release --locked -p dedup-cli -- review-ids records.tsv
```

Use a literal tab between the header fields `doi` and `pmid`, in that order.
Each following line represents one record. Leave an unknown field empty but
keep its tab. The first data line is row 1. For example, with three rows:

```text
doi	pmid
10.0000/example	11
https://doi.org/10.0000/EXAMPLE	22
10.0000/other	22
```

Rows 1 and 2 share a DOI but disagree on PMID. Rows 2 and 3 share a PMID but
disagree on DOI. The output lists **two separate review groups**, not a cluster
of three. All three rows are retained; none is suppressed.

- The command reads a file without changing it, or reads standard input when
  the filename is `-`. Output goes to standard output. The report references
  row numbers, not identifier values. Keep the source file to inspect the rows.
- DOI comparison ignores ASCII case and known resolver prefixes. Resolver paths
  are percent-decoded once. Literal suffixes, versions and punctuation stay
  significant. Only supported ASCII DOI forms and positive digit-only PMIDs
  without leading zeros are indexed. No registry lookup is performed.
- Empty, malformed or unsupported data rows are counted and diagnosed if they
  have no usable identifier or contain a formatting error. A valid identifier
  can still contribute evidence when the other field is unsupported. There is
  no title matching and no inference about records without a shared key.
- Each group indicates whether the other identifier agrees for every member,
  conflicts, or is incomplete. Groups may overlap. The summary counts each
  candidate record once. Even full agreement never automatically merges rows.
- The TSV is an unquoted two-column projection, not a general spreadsheet or
  bibliographic file importer. LF and CRLF and an optional UTF-8 BOM are accepted.
  A wrong header, non-UTF-8 input, more than 100,000 rows, or more than 16 MiB
  of input stops the run with an error and no report.

## Development

The repository separates the Rust engine (`crates/dedup-core`) from the command
line entry point (`crates/dedup-cli`). The desktop architecture is documented in
[`app/README.md`](app/README.md) and the
[architecture specification](docs/04-architecture-and-ui-spec.md).

Run the same source checks used in CI:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Documentation contributors can check local Markdown links with Python 3.10 or
newer:

```sh
python scripts/check_docs.py
```

## Validation

Human validation across selected research fields and records is planned.
Accuracy estimates will require independently reviewed reference labels and a
completed evaluation. Automated build checks establish software buildability,
not deduplication accuracy.

## Documentation

- [Documentation index](docs/00-index.md)
- [Methodology and literature](docs/01-methodology-and-literature.md)
- [Matching rules and pipeline](docs/02-deduplication-rules-and-pipeline.md)
- [Database schemas and formats](docs/03-database-schemas-and-formats.md)

## Contributing

[Open an issue](https://github.com/SKang393/record-deduplicator/issues) to discuss
a bug, format requirement, or proposed change. Include your operating system,
the command you ran, and the expected and observed behavior. Use synthetic or
redistributable examples rather than private or licensed database exports.

Keep changes focused, follow the [engine contract](docs/10-engine-contract.md),
and run the relevant checks above.

## License

Licensed under the [Apache License 2.0](LICENSE).
