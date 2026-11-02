# record-deduplicator

An auditable local bibliographic record deduplicator for systematic reviewers.
The design emphasizes difficult database exports, preserved report relationships
and field provenance, with Windows and macOS as target platforms. The planned
benchmark spans ten core fields plus supplemental special education.

- **Repository:** `https://github.com/SKang393/record-deduplicator`
- **License:** [Apache-2.0](LICENSE)
- **Author:** SKang393

---

## Current status

**Foundation and development plan.** The Rust workspace builds and the CLI
exposes help/version information. Import, matching, review, export, the desktop
application, and accuracy benchmarks are not implemented yet.

**Current scope is Terra preparation only.** Collection has stopped at the user's
direction. Existing partial evidence is preserved; no field corpus is admitted.
Start from [the current Terra handoff](docs/14-terra-handoff.md).

## What we are building

Remove duplicate records while preserving distinct reports and explaining each
retained, merged, and linked record. The proposed product combines:

- Source-format handling backed by explicit per-dialect fixtures across the
  prespecified fields, including the user's six education databases.
- Separate duplicate clusters and protected report relationships, including
  protocols, dissertations, preprints, conference abstracts, and versions.
- Immutable sources, field-level provenance, reversible review, and reproducible
  outputs from versioned policies.
- One offline Rust engine for a CLI and Tauri 2 + React desktop application.

These are planned capabilities, not measured advantages. Existing products
already offer many individual features. See the source-backed
[competitor comparison and differentiation](docs/07-product-positioning.md).
The application will not use an LLM; optional LLM assistance belongs only to the
external human-supervised evaluation harness.

## Start development with Terra 6.1

1. Read [the current handoff](docs/14-terra-handoff.md), [AGENT.md](AGENT.md) and [TASKS.md](TASKS.md).
2. Follow the [full development plan](docs/08-development-plan.md), beginning
   with **B01: source and canonical record contracts**.
3. Use the [GPT-6.1 Terra runbook](docs/09-terra-execution.md) for the model command,
   bounded implementation prompt, and evidence handoff.
4. Apply the [engine contract](docs/10-engine-contract.md) when older design notes
   conflict. It defines identifier safety, clusters, preservation and determinism.

## Current foundation commands

Install a stable Rust toolchain with rustfmt/clippy and Python 3.10 or newer.
Run from the repository root:

```text
cargo run --locked -p dedup-cli -- --help
cargo run --locked -p dedup-cli -- --version
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python scripts/check_docs.py
```

The foundation has no matching tests or matching implementation yet. Unsupported
CLI operations fail explicitly instead of producing deduplicated output.

---

## Documentation & Handoff Index

- **[`AGENT.md`](AGENT.md) / [`AGENTS.md`](AGENTS.md):** Operating instructions and handoff guide for AI coding agents.
- **[`TASKS.md`](TASKS.md):** Ordered milestone backlog and immediate next engineering tasks.
- **[`DECISIONS.md`](DECISIONS.md):** Architectural and product decisions log.
- **[`docs/00-index.md`](docs/00-index.md):** Full index of methodology, rules, schemas, UI architecture, validation harness, and benchmark search protocols.
- **[Preparation review](docs/11-preparation-review.md):** Corrections, actual checks and remaining evidence gates.
- **[Collection evidence](docs/12-collection-evidence.md):** Database-use frequencies, journal/paper evidence, actual search counts and incomplete collection items.
- **[Data boundaries](testdata/README.md):** Synthetic fixtures versus private exports.
- **[External validation](validation/README.md):** Independent evaluation boundary.

Use the **Aside CLI** for manuscripts and Purdue or other university/subscription
resources in a future user-selected collection run. The prepared protocol covers
ten core fields and supplemental special education, using EBSCOhost and commonly
used native databases. Preserve original RIS, PubMed NBIB and the user-selected
Springer native CSV format. CSV collection and coverage remain unresolved.
Proprietary exports, credentials,
model payloads and generated run data are excluded from Git.

Copyright 2026 Sungwoo Kang. External datasets and referenced publications retain
their own rights and are not relicensed by this repository.
