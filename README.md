# Record Deduplicator

[![CI](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)
[![Windows build checks](https://img.shields.io/badge/Windows-build%20checks-0078D4)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)
[![macOS build checks](https://img.shields.io/badge/macOS-build%20checks-555555)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)

An offline bibliographic record deduplication project for systematic reviews and
meta-analyses, built around a shared Rust engine. The goal is to identify duplicate
database records while preserving distinct reports, original metadata, and the
reasoning behind each decision.

**In development:** the CLI can review shared full DOIs and PMIDs directly from
one or more UTF-8 RIS exports or a two-column TSV file. The review queue keeps
every occurrence and leaves the input files intact. Automated merging and the desktop application are not
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

Build the CLI from the review branch.

```sh
git clone --branch review/native-ris-audits https://github.com/SKang393/record-deduplicator.git
cd record-deduplicator
cargo build --release --locked -p dedup-cli
```

The executable is `target/release/dedup.exe` on Windows and
`target/release/dedup` on macOS. Python and Node.js are not required for this CLI
build.

## Usage

For a first run, follow the [two-export walkthrough](#verify-a-copy-found-through-metadata).
It creates its own inputs, adds the built executable to your Bash path, and
shows how to verify a copy, reopen pending decisions and reconcile the export.
Other examples assume `dedup` is on your path. Use `--help` for syntax or
`--version` for the build version.

### Review multiple RIS exports together

Assign a unique label to each export. This command compares two database
searches and saves the review queue to a new file.

```sh
dedup review-ids --ris-source catalog catalog.ris --ris-source index index.ris > review.tsv
```

Choose a new output filename separate from your inputs. Labels may contain 1
to 64 ASCII letters, digits, dots, hyphens or underscores and must start with
a letter or digit. They are case-sensitive. Reordering the arguments produces
the same report. Two separately labeled exports retain their own occurrences,
even when their filenames or contents are identical.

- Each `input` row records a label, byte length, occurrence count and SHA-256
  digest. Keep the files with the report. The digest lets you check that you
  are inspecting the same bytes later.
- A `candidate` row lists occurrences sharing a full DOI or PMID. For example,
  `catalog:3,index:3` points to the third occurrence in each file. Groups can
  overlap and can include several records from one file. The final column
  reports agreement, conflict or incomplete evidence for the other identifier.
- A `triage` row with signal `title_variant_locator` names two occurrences
  from different sources, at least one without a usable DOI or PMID. They share
  a year, RIS type, journal, volume and final page range or article number.
  Issue and work type must also match, including when absent. The titles
  agree across selected punctuation
  changes or after dropping one delimited subtitle of up to eight words.
  At least five title words must agree. The rule skips keys shared by more
  than four records, conflicting identifiers and differing first-author
  surnames when both names use comma form. Use `--inspect-pair` to see the
  evidence. The `inspect_only` row adds no editable decision and removes
  zero records. Either occurrence can have a sheet row through other evidence.
- Each `source` row gives the original half-open byte range and a disposition.
  `review_candidate` means inspect its candidate groups and any `issue` rows.
  `review_input` means inspect a damaged record or identifier issue.
  `retained` means the occurrence has no flagged issue or shared identifier.
  All three dispositions keep the record. Reports of the same study that
  share only a title, accession or study identifier stay separate.

The command accepts up to 128 named files. The combined limits are 16 MiB,
100,000 occurrences and 250,000 tagged fields. Input errors and exceeded limits
stop the run before it writes any report rows.

### Verify a copy found through metadata

This example creates a catalog article, an index copy missing its DOI, and
an index record with an extra subtitle at the same journal location. After
building the review branch above, start at the repository root and run the
blocks in the same Bash session (Git Bash on Windows).

```sh
export PATH="$PWD/target/release:$PATH"
cd "$(mktemp -d)"
cat > catalog.ris <<'RIS'
TY  - JOUR
TI  - Marsh transects after spring flooding
AU  - Reed, E.
PY  - 2021
JO  - Fable Ecology
VL  - 8
SP  - 21-26
DO  - 10.0000/marsh-a
ER  -
RIS
sed '/^DO  - /d' catalog.ris > index.ris
sed '/^DO  - /d; s/^TI  - .*/&: seasonal observations/' catalog.ris >> index.ris
review() { dedup review-ids "$@" --ris-source cat catalog.ris --ris-source idx index.ris; }
review --init-metadata-decisions choices.tsv
review --inspect-pair choices.tsv cat:1 idx:1
review --inspect-pair choices.tsv cat:1 idx:2
```

Find these two lines in the queue from `--init-metadata-decisions`.

```text
triage	title_author_year_type	cat:1,idx:1	inspect_metadata_only
triage	title_variant_locator	cat:1,idx:2	inspect_only
```

The first pair has two pending sheet rows. In the comparison, both records
give the same title and author in *Fable Ecology*, volume 8, pages 21–26.
For the second pair, read the extra subtitle under `[idx:2] outside-decision-sheet`.
Each heading gives the occurrence's sheet status and original byte range.
`cat:1` is editable because of its exact-title pair with `idx:1`.
Leave `idx:2` in the export. Record the keep target and rationale for the
verified index copy by editing the two existing rows as follows.

```sh
awk -F '\t' '
  $1 == "cat:1" {print "cat:1\tkeep"; next}
  $1 == "idx:1" {print "idx:1\tverified-duplicate-of=cat:1\tMatched Reed, Fable Ecology 8, pages 21-26."; next}
  {print}
' choices.tsv > edit.tmp && mv edit.tmp choices.tsv
review --resume-decisions choices.tsv
review --pending-decisions choices.tsv
review --export-reviewed choices.tsv reviewed
cat reviewed/mapping.tsv
cat reviewed/reconciliation.tsv
```

In the reopened queue, the `decision` row for `idx:1` now reads
`verified-duplicate-of=cat:1` and the `rationale` row records why. The
title-variant lead still appears in that queue even though the pending view
counts zero open sheet decisions. Reconciliation counts three occurrences,
one verified copy removed and two retained. In `reviewed/mapping.tsv`,
`cat:1` and `idx:1` point to `output=1` in `reviewed/reviewed.ris`, each
with its own byte span. The `not-candidate` mapping for `idx:2` means it
falls outside this sheet and keeps its own `output=2`.

To [open this export in Zotero](#import-the-reviewed-ris-into-zotero), get
the full filename before leaving the walkthrough shell. In Git Bash on
Windows, run `cygpath -aw reviewed/reviewed.ris`. On macOS or Linux, run
`printf '%s/reviewed/reviewed.ris\n' "$PWD"`.

To undo only the copy decision, restore its row to `pending` and drop the
rationale column. Export to a fresh directory and inspect the new mapping.

```sh
awk -F '\t' '$1 == "idx:1" {print "idx:1\tpending"; next} {print}' choices.tsv > edit.tmp && mv edit.tmp choices.tsv
review --export-reviewed choices.tsv restored
cat restored/mapping.tsv
cat restored/reconciliation.tsv
```

The restored total shows all three retained. The kept catalog entry maps to
`output=1`, the reopened index copy to `output=2`, and the subtitled record
to `output=3`.

Every editable row starts pending. Reopening or exporting checks the original
source fingerprints. A verified copy needs an explicit keep target and a
single-line rationale of up to 512 UTF-8 bytes. All records assigned to that
target must belong to one metadata hint of at most 16 records and agree on
title, authors, year, RIS type, journal, volume and final page range or
article number (`SP`/`EP`). Work type (`M3`) and issue (`IS`) must also
agree when supplied. Conflicting supported DOIs or PMIDs block the set.
Leave unresolved copies and distinct reports pending or mark them keep.
Existing identifier-only sheets still open with `--resume-decisions`.

### Return to pending decisions

Reopen your saved sheet with the same named exports. This command uses the
sheet saved by the undo example above.

```sh
dedup review-ids --pending-decisions choices.tsv --ris-source cat catalog.ris --ris-source idx index.ris
```

The command checks the saved fingerprints and writes a read-only TSV report
to standard output. In this example the summary counts one pending occurrence
and the group row identifies its keeper.

```text
pending	title_author_year_type	idx:1	inspect_metadata_only;keep=cat:1;assigned=
```

The third column lists pending occurrences in one direct evidence group.
`keep` lists its explicitly kept members and `assigned` lists members already
marked as copies. Groups flagged for conflicting alternate IDs or containing
a record with contradictory IDs appear first. Within each tier
the order is DOI, PMID, then opt-in metadata hints. Every group with a pending
member appears, even when it has zero or several keepers. Groups can overlap and
the summary counts each pending occurrence once.

Pass two keys from a group to `--inspect-pair` to read their tagged fields and
continuation lines in source order. The display checks the sheet's fingerprints
and escapes control characters for reading in a terminal. If the records give
insufficient evidence, leave the decision pending. Edit `choices.tsv` when you
reach a decision, then rerun `--pending-decisions`. Use `--resume-decisions`
for the full queue, including input issues.

### Reconcile a reviewed export before screening

Open `reconciliation.tsv` in the export directory. It has one row per named
source and one total row. Every count represents input occurrences. Each
removed copy counts against the source it came from, including when another
source supplied the kept record. Use `mapping.tsv` for individual decisions,
source byte spans and output ordinals.

- `input = removed_id + removed_metadata + retained`. The removal columns
  count explicit duplicate decisions applied before literature screening.
- `retained = keep + pending_id + pending_metadata + outside_sheet_flagged + outside_sheet_unflagged`.
  Start with a source's pending columns to see where decisions remain open.
  A pending row with an identifier candidate counts under `pending_id` even
  if it also has a metadata hint. `pending_metadata` counts metadata-only
  rows in the opt-in sheet.
- The outside-sheet columns count exported occurrences with no decision row
  in the selected sheet. For `outside_sheet_flagged`, inspect the queue's
  `review_input` records and their input issues.

The total `retained` count is the number of records in `reviewed.ris` going
forward to screening. Title/abstract exclusions, full-text exclusions and
study inclusion decisions belong to later screening stages.

### Import the reviewed RIS into Zotero

In Zotero, choose **File → Import… → A file** and select `reviewed.ris` from
the export directory. Import into a new collection so you can compare its item
count with `retained` in the total row of `reconciliation.tsv`. Pending records
count toward that total. Keep the original exports, decision sheet and
`mapping.tsv` together to trace each item back to its source occurrences.

The [synthetic Zotero handoff](testdata/fixtures/README.md#zotero-ris-handoff)
includes a ready-to-import file and the fields observed in Zotero Desktop.
Zotero's [file import guide](https://www.zotero.org/support/kb/importing_standardized_formats)
lists the supported import formats.

### Review a single RIS export

```sh
cargo run --release --locked -p dedup-cli -- review-ids --ris records.ris
```

This command reads records beginning with `TY  -` and ending with `ER  -`.
LF, CRLF, and bare CR line endings may occur within the same file.
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

To measure a 15,210-occurrence synthetic search, build the release CLI and
run `python scripts/check_large_review.py target/release/dedup` (use
`dedup.exe` on Windows). It checks the export against generated report
identities and prints candidate load, unresolved categories and CLI timings.

To measure title variants on 47 independently labeled synthetic occurrences,
run `python scripts/check_title_variants.py target/release/dedup`. Supply
`--before <original-cli>` to compare queues, sheet membership and export
bytes against the previous CLI. Add `--scale-targets` to count the 111
variant-title copies from the earlier scale audit separately.

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
