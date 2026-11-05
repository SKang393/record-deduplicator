# Record Deduplicator

[![CI](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)
[![Windows build checks](https://img.shields.io/badge/Windows-build%20checks-0078D4)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)
[![macOS build checks](https://img.shields.io/badge/macOS-build%20checks-555555)](https://github.com/SKang393/record-deduplicator/actions/workflows/ci.yml)

An offline bibliographic record deduplication project for systematic reviews and
meta-analyses, built around a shared Rust engine. The goal is to identify duplicate
database records while preserving distinct reports, original metadata, and the
reasoning behind each decision.

**In development:** the public `main` branch currently builds a CLI foundation
with help and version commands. End-to-end deduplication and the desktop
application are not yet published in this branch. There are no prebuilt release
downloads. CI checks the foundation on Windows and macOS.

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

See the [engine contract](docs/10-engine-contract.md) for the technical rules and
the [development tracker](TASKS.md) for implementation progress.

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

The currently published CLI supports these commands from the repository root:

```sh
cargo run --release --locked -p dedup-cli -- --help
cargo run --release --locked -p dedup-cli -- --version
```

You can also run the built executable directly:

```powershell
# Windows PowerShell
.\target\release\dedup.exe --help
```

```sh
# macOS
./target/release/dedup --help
```

These commands inspect the foundation build. Import, matching, review, and export
commands will be documented when their implementation is published.

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
- [Development plan](docs/08-development-plan.md)
- [Architecture decisions](DECISIONS.md)

## Contributing

[Open an issue](https://github.com/SKang393/record-deduplicator/issues) to discuss
a bug, format requirement, or proposed change. Include your operating system,
the command you ran, and the expected and observed behavior. Use synthetic or
redistributable examples rather than private or licensed database exports.

Keep changes focused, follow the [engine contract](docs/10-engine-contract.md),
and run the relevant checks above.

## License

Licensed under the [Apache License 2.0](LICENSE).
