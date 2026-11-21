# Record Deduplicator

[![CI](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)
[![Windows build checks](https://img.shields.io/badge/Windows-build%20checks-0078D4)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)
[![macOS build checks](https://img.shields.io/badge/macOS-build%20checks-555555)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)

An offline bibliographic record deduplication project for systematic reviews and
meta-analyses, built around a shared Rust engine. The goal is to identify duplicate
database records while preserving distinct reports, original metadata, and the
reasoning behind each decision.

**In development.** The CLI accepts named UTF-8 RIS files for a saved
decision sheet and a reviewed RIS export. A standalone two-column DOI/PMID
TSV produces an identifier report. Both workflows leave the inputs intact.
Removing a copy from the export requires a human decision. Use a review
build or compile the CLI from source. The desktop application is planned.

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

## Try the CLI without Rust

For a review-branch run that includes the CLI packaging step, sign in to
GitHub, open the successful run for your chosen commit, and select its
`recorddedu-…tar.gz` under **Artifacts**. Choose `x86_64-pc-windows-…`
for 64-bit Intel/AMD Windows or `aarch64-apple-darwin` for Apple silicon.
The filename includes the commit prefix. The packaging step prints the
archive's SHA-256 digest. Review artifacts expire after seven
days. [Review branch runs](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml?query=branch%3Areview%2Fnative-ris-audits)

Extract the archive (for example, `tar -xzf <downloaded-filename>`) and open
a terminal in the extracted `recorddedu-…` directory. `BUILD.txt` identifies
the commit and compiler. `SHA256SUMS` gives the executable digest. These are
development builds without publisher signing or macOS notarization.

### Start a review with your own exports

In a build with guided setup, run **`.\dedup.exe start`** in Windows
PowerShell or **`./dedup start`** in macOS Terminal. At the prompts:

1. Enter the full path for your decision sheet in an existing project folder,
   for example a file named `Review choices.tsv`. Use the saved sheet's path
   to return to a review.
2. Give your first export a short label such as `catalog`, then paste its
   RIS file path on the next line. To copy a full path, use **Copy as path**
   in Windows Explorer or select the file in Finder and press **Option-Command-C**.
   Paste the path on one line, with spaces and accents as written. You can
   enclose a full path in single or double quotes. On macOS the guide also
   recognizes backslash-escaped spaces in an existing path. If it asks again
   about a new name, use plain spaces, or double-quote the full path to keep
   a literal backslash. For other shell escapes, copy the plain path from
   the file manager. Repeat for each export.
3. Press Enter at the next label prompt. Check the sheet, labels and paths
   shown, then type `yes`. Type `:cancel` at any prompt to stop.
4. For a new sheet, the terminal shows the candidate queue and every editable
   row starts pending. Open the sheet in a plain-text editor or
   [edit it in LibreOffice Calc](#edit-the-decision-sheet-in-libreoffice-calc).
   A key such as `catalog:2` means the second RIS record in the export labeled `catalog`.
   To reopen, run the guide again with the same labels and original files.
   It checks their fingerprints and shows pending decisions and keeper context.
   Optional comparisons appear in the full queue. To see them again, use
   `--resume-decisions` with the saved sheet and the same named sources.
5. After reviewing the pending rows, press Enter to finish, or enter a new
   folder path to preview an export. Choose a folder name inside an existing
   directory, which can be outside the project. Check the sheet path,
   destination, explicit duplicate count, and how many pending records will
   be kept. Type `write` to save the three
   files, or press Enter to cancel. [Check the RIS and mapping together](#reconcile-a-reviewed-export-before-screening)
   before opening `reviewed.ris` in your reference manager. Use
   `mapping.tsv` for individual decisions and `reconciliation.tsv` for
   counts by source.

Compare the source reports before editing a decision. Leave uncertain rows
pending. For a worked example of pair inspection, an explicit decision and
export, follow the [two-export walkthrough](#verify-a-copy-found-through-metadata).
To run that example, open Bash (Git Bash on Windows) in the extracted
directory and run `export PATH="$PWD:$PATH"`.

### Edit the decision sheet in LibreOffice Calc

1. Keep a backup of the last sheet that reopened successfully. Open the working
   `.tsv` as **Text CSV** in Calc. Select **Unicode (UTF-8)** and **Tab** as
   the only separator. Clear the string delimiter and turn off **Evaluate
   formulas**. Select all four preview columns (A through D) and set their
   column type to **Text**. Check that labels such as `0007` retain every zero.
2. Edit only the decision cells on occurrence rows. For a verified copy in
   a metadata-enabled sheet, enter `keep` in column B on the keeper's row.
   On the copy's row, enter, for example, `verified-duplicate-of=0007:1`
   in B and a single line explaining the evidence in C. Leave uncertain
   cases pending. Preserve the row order, occurrence keys, input rows and
   final `end` row.
3. Save back to the same `.tsv` using **Text CSV** with **Edit filter
   settings**. Select **Unicode (UTF-8)**, the **Tab** field delimiter and
   an empty text delimiter. Leave **Quote all text cells** off and keep the
   `.tsv` filename. Close Calc and reopen the sheet with `dedup start` and
   the original named sources to see pending decisions. Check the saved
   rationale in the reviewed export's `mapping.tsv` or with the
   [walkthrough's `--resume-decisions` command](#verify-a-copy-found-through-metadata).

In a synthetic trial with Calc 25.2.3.2 on Linux, Standard import converted
the source label `0007` to `7`. If the inputs or occurrence keys differ on reopen, restore
the backup and reapply the verified edits by key using Text columns. The
reader accepts trailing empty cells, LF or CRLF, and an optional UTF-8 BOM.
It checks the source fingerprints and keys. The reviewer checks the wording
of the rationale. [LibreOffice text import and export settings](https://help.libreoffice.org/latest/en-US/text/shared/guide/csv_params.html)

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
export PATH="$PWD/target/release:$PATH"
```

The executable is `target/release/dedup.exe` on Windows and
`target/release/dedup` on macOS. Python and Node.js are not required for this CLI
build.

## Usage

Use [guided setup](#start-a-review-with-your-own-exports) for your own files.
The [two-export walkthrough](#verify-a-copy-found-through-metadata) creates
synthetic inputs and shows how to verify a copy and reconcile the export.
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
- A `complementary_id_locator` triage row gives an inspection lead when one
  export supplies only a DOI and another supplies only a PMID. Both occurrences
  must be free of identifier issues and shared-ID groups. They must be the only
  two occurrences at the matching title and location, with the same full title
  of at least five words, the same listed authors in order, year, RIS type,
  journal, volume and final page range or article number. Issue and work type
  must agree as well. Use `--inspect-pair` to read each original. A protocol
  and follow-up abstract can share a heading and printed pages. Read the
  original notes (`N1`) and abstracts (`AB`) for differences in what each
  report describes. The lead adds no editable decision, and both records
  are retained by default.
- Each `source` row gives the original half-open byte range and a disposition.
  `review_candidate` means inspect its candidate groups and any `issue` rows.
  `review_input` means inspect the record's identifier issues.
  `retained` means the occurrence has no flagged issue or shared identifier.
  All three dispositions keep the record. Reports of the same study that
  share only a title, accession or study identifier stay separate.

The command accepts up to 128 named files. The combined limits are 16 MiB,
100,000 occurrences and 250,000 tagged fields. Input errors and exceeded limits
stop the run before it writes any report rows. An empty source counts as zero
occurrences. If a named source contains a fragment, an unclosed record, or a
malformed line, the command stops before creating a decision sheet. The error
identifies the first damaged occurrence by label, ordinal and byte range.
If an interrupted field runs into the next export's UTF-8 BOM and `TY`
header, the error also gives the byte offset of that BOM. Retrieve the
complete batch and supply each original export under its own label. With
no BOM or line break at the join, a `TY` header can read as field text and
two records can map to one occurrence.
Run `dedup review-ids --ris <file>` for a read-only diagnostic report.

### Check downloaded batches against a search snapshot

Run `--check-coverage` once per database search as it stood at export time.
Copy the expected total or inclusive batch ranges from that search log.
List the files actually received with `--ris-source`. Give a later search
update its own invocation, even if it uses the same database and batch labels.
The check reads the original RIS and prints pre-deduplication occurrence
counts and file fingerprints to standard output. You assign each file to its
snapshot using the search log.

For a synthetic walkthrough, run these Bash commands with `dedup` on your path.
The two batches deliberately include one report twice.

```sh
cd "$(mktemp -d)"
record() { printf 'TY  - JOUR\nTI  - %s\nER  -\n' "$1"; }
{ record Estuary; record Inlet; } > first.ris
{ record Inlet; record Delta; } > second.ris
record Inlet > short.ris
record Soundings > update.ris

survey() { dedup review-ids --check-coverage coast-day1 --expected-total 4 "$@"; }
planned() { survey --batch-range first 1-2 --batch-range second 3-4 "$@"; }
planned --ris-source first first.ris --ris-source second second.ris
dedup review-ids --check-coverage coast-update --expected-total 1 --ris-source first update.ris
```

The first check reports four parsed occurrences and `matched_expectations`.
The update independently reports one. Both commands exit with code 0.
When counts or ranges disagree, or batch labels share a file fingerprint,
exit code 2 accompanies a coverage report on standard output. Invalid
arguments or unreadable or malformed RIS also produce exit code 2, with a
diagnostic on standard error and no coverage report. Try the following checks
individually.

```sh
planned --ris-source first first.ris
planned --ris-source first first.ris --ris-source second short.ris
survey --batch-range first 1-2 --batch-range second 2-3 --ris-source first first.ris --ris-source second second.ris
planned --ris-source first first.ris --ris-source second first.ris
```

| Change | What the check reports |
|---|---|
| Omit the whole second file | `missing_batch` for positions 3–4 and two parsed out of four |
| Supply a shorter, well-formed second file | `batch_count_mismatch` for `second`, one parsed out of two |
| Declare overlapping ranges | `range_overlap` at position 2 and `range_gap` at position 4, even though four records parsed |
| Supply the first download for both ranges | `counts_and_ranges` still matches. `source_flag` reports `repeated_source_bytes` for labels `first,second`. |

Use `--expected-total` alone when the log has only a total. With
`--batch-range` alone, the check covers the window from the first declared
position to the last. Supply both to check the entire numbered interval.
A repeated-bytes flag means two or more labels in this snapshot share a
SHA-256 fingerprint of the original file bytes. Renaming a download preserves
its fingerprint. Changing its line endings changes the fingerprint even when
every normalized RIS field is identical. Two distinct batches can share
individual records and pass. Check the flagged labels against the search log.
An equally sized batch of different records can also pass. Counts and file
fingerprints alone cannot establish record identity. PRISMA-S lists records
retrieved per source and deduplication methods as separate reporting items. [PRISMA-S, items 15 and 16](https://doi.org/10.1186/s13643-020-01542-z)

### Bring a BibTeX export into a named RIS review

Suppose one database supplied `catalog.ris` and another supplied a UTF-8
`index.bib`. Keep both originals. With [bibutils](https://sourceforge.net/p/bibutils/home/Bibutils/)
installed, these Bash commands convert the BibTeX to a separate RIS file
and show the queue. Use a new conversion directory for each export.

```sh
mkdir converted && (
  set -o pipefail
  bib2xml -i utf8 index.bib | xml2ris -o utf8 > converted/index.ris
) && dedup review-ids --ris-source catalog catalog.ris --ris-source index converted/index.ris
```

Compare the index `input` occurrence count with the original BibTeX entry
count. Check the citation keys, titles and identifiers against the converted
RIS. In the tested bibutils 7.2 example, citation keys became RIS `ID`
values, a quoted comma survived in the title and a wrapped abstract became
one line. Naming a raw BibTeX file with a `.ris` suffix still produces a
fragment error in the named review. Use the converted file for this step.

Once the counts and fields reconcile, create the sheet with
`--init-decisions choices.tsv` before the same two `--ris-source` arguments.
Inspect candidate pairs, mark a keeper and enter `duplicate-of=<keeper key>`
only for a confirmed copy. Leave unresolved rows pending. Use
`--export-reviewed choices.tsv reviewed` with the same named RIS files.
Save the original BibTeX, its SHA-256, the converter version and the exact
converted RIS alongside the sheet. For `index`, the saved fingerprint and
the byte ranges in `mapping.tsv` refer to `converted/index.ris`. The RIS
`ID` links a converted record back to its BibTeX citation key in this example.

To reproduce the six-occurrence synthetic check with bibutils on your path,
run `python scripts/check_mixed_export.py target/release/dedup` (use
`dedup.exe` on Windows).

### Verify a copy found through metadata

This example creates a catalog article, an index copy missing its DOI, and
an index record with an extra subtitle at the same journal location. After
adding the extracted or source-built CLI to your path as shown above, run
the blocks in the same Bash session (Git Bash on Windows).

```sh
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
and escapes control characters for reading in a terminal. If a record lacks a
usable final page range or article number, the comparison identifies its source
and ordinal and tells you whether it supplied any pagination fields (SP/EP).
If the records give insufficient evidence, leave the decision pending. Edit
`choices.tsv` when you reach a decision, then rerun `--pending-decisions`.
Use `--resume-decisions` for the full queue, including input issues.

### Trace an exported occurrence to its originals

Take the occurrence key from the third column of a `map` row in
`mapping.tsv`. Give `dedup trace` that mapping, the key and the folders
containing the original downloads. In the metadata walkthrough above, the
downloads are in the current directory.

```sh
dedup trace reviewed/mapping.tsv idx:1 .
dedup trace reviewed/mapping.tsv idx:2 .
```

The first lookup shows the removed `idx:1` and its explicit keeper `cat:1`,
both at output record 1. The second shows the retained `idx:2` at output
record 2. Each row gives the matching file path, its whole-file SHA-256,
and the original half-open byte range (start included, end excluded).
The decision and output number come from that export's mapping, so you can
trace it even after editing the sheet or renaming a download. Trace verifies
these rows against the mapping you supply. Authenticating the mapping itself
requires independent evidence of its origin and integrity.

The command checks regular files directly inside each folder you supply.
It verifies the source fingerprint, record ordinal and byte range before
printing any rows. Byte-identical files can have multiple matching paths,
which it lists. Supply additional folders as separate arguments and quote
paths with spaces. The read-only lookup prints paths and provenance. Use
`--inspect-pair` when you also want to read the original RIS fields.

### Reconcile a reviewed export before screening

Before handing off a reviewed RIS file, compare its exact bytes with the output
fingerprint in the mapping from the intended run. Supply both paths, even if
you have renamed the files or moved them to different folders.

```sh
dedup verify-export reviewed/mapping.tsv reviewed/reviewed.ris
```

Exit 0 prints `matched_output_fingerprint`, both supplied paths and their
SHA-256 digests, and the RIS byte length. The mapping digest identifies the
mapping you supplied. This read-only check works with version 1 and 2 mappings
that contain an output fingerprint. A mismatch or an unreadable, missing or
ambiguous fingerprint gives exit 2 with a diagnostic and empty standard output.

Use a mapping whose origin and integrity you trust independently. The check
compares the selected RIS bytes with that mapping's output fingerprint. If two
runs produced byte-identical RIS files, either file will pass and the result
cannot distinguish their runs or verify the review decisions.

Open `reconciliation.tsv` in the export directory. It has one row per named
source and one total row. Every count represents input occurrences. Each
removed copy counts against the source it came from, including when another
source supplied the kept record. Use `mapping.tsv` for individual decisions,
source byte spans and output ordinals.

Compare each source's `input` count with the count recorded when you exported
that batch. A download that stops after a complete `ER` record can still be
valid RIS. A repeated page can also offset a missing page in the total. Keep
the original batch boundaries and check their ranges against the search log.

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

Run `python scripts/check_split_identifiers.py <baseline-cli> <current-cli>`
with the baseline CLI built at `059128ad57a5` to compare direct review pairs
on the frozen search, its index and update exports alone, the title-variant
cases and a fresh synthetic contrast. The contrast includes separate abstracts
with identical citation details and checks the unchanged sheet and export.
For the 164 pairs lacking final pagination, run the same script with
`--missing-pages`, a baseline CLI built at `417b9b491ea2`, and the current CLI.
It tests exact abstract text and publisher article URLs offline on the frozen
slice and a separate ahead-of-print and follow-up fixture.

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
