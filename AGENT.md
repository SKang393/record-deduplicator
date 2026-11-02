# Agent Handoff & Operating Instructions (`record-deduplicator`)

Welcome! This file (`AGENT.md` / `AGENTS.md`) is the primary entry point for any AI coding agent taking over development of **`record-deduplicator`**. Read this document and [`TASKS.md`](TASKS.md) before making changes.

## Current execution contract (revised 2026-11-02)

- **Preparation only:** the user's latest instruction stops research execution
  and export acquisition. Finish the Terra setup and handoff. Do not initiate
  searches, exports, downloads, mailbox retrieval or file-copy watchers. Preserve
  existing partial evidence. See [the current handoff](docs/14-terra-handoff.md).
- The repository is a foundation build, not a working deduplicator. Follow the
  bounded batches in [the development plan](docs/08-development-plan.md) and
  [Terra 6.1 runbook](docs/09-terra-execution.md).
- [The engine contract](docs/10-engine-contract.md) supersedes contradictory
  algorithm, validation, performance and reporting claims in the original notes.
- Start every user response with `Thread: [short title]` and a one-sentence
  `Summary:`. Be concise and avoid em dashes. Final reports list files changed,
  implementation, commands, verification and exact remaining blockers.
- Before editing, identify inspection/change paths, runtime impact and one
  validation method. Preserve unrelated work and originals. No success claim
  without artifacts or command evidence.
- For broad batches use a lead integrator and available nonoverlapping subagents;
  this environment supports up to three alongside the lead. The lead owns shared
  files, integration and final QA. Use waves when needed. If unavailable, continue
  single-agent and disclose it. Do not revert others' work.
- Run relevant checks after the integrated batch. Do not rerun unchanged failing
  commands without a new hypothesis or fix. Distinguish source/build checks from
  native workflow, accuracy and release acceptance.
- Do not delete files unless confirmed unused or explicitly required. Search
  references first. Do not move assets without updating references and validating.
  Install missing tools when needed within the authorized task, and report exact
  blockers if installation or access fails.

---

## 1. Project Mission & Scope

**`record-deduplicator`** is a high-speed, offline-first, explainable bibliographic record deduplication engine and desktop application for systematic reviews and meta-analyses across the ten core benchmark fields, with special education reported as a supplemental field.

- **Core Architecture:** Hybrid Multi-Stage Deterministic and Fuzzy Record Linkage Pipeline (Rule-Based Multi-Key Blocking + String Similarity Matching + False-Positive Guards + Provenance-Preserving Merge).
- **Tech Stack:**
  - **Backend / Engine:** Rust workspace (`crates/dedup-core`, `crates/dedup-cli`).
  - **Desktop GUI:** Tauri 2.0 + React + TypeScript + Vite + Tailwind CSS + `shadcn/ui` + TanStack Virtual (`app/`).
  - **Target OS:** Windows Native (WebView2) first, with **macOS (WKWebView, Universal x86_64/arm64) compatibility enforced from day one**.
- **License:** Apache-2.0
- **Target GitHub Repository:** `https://github.com/SKang393/record-deduplicator` (Public)

---

## 2. Mandatory Operational Rules (MUST FOLLOW)

1. **Git & Commit Attribution Policy (`SKang393` ONLY):**
   - All commits and pushes MUST be recorded **exclusively as `SKang393`**.
   - Keep git identity set to `Sungwoo Kang <200436385+SKang393@users.noreply.github.com>` (GitHub maps this noreply email directly to the `SKang393` account, matching all previous commits).
   - **NEVER** add `Co-authored-by:`, `Signed-off-by:` for an AI/bot, or any agent attribution trailers in commit messages or PR descriptions.
2. **LLM Validation Is External Only (NOT Part of the Program):**
   - The shipped `record-deduplicator` binary and `dedup-core` / `dedup-cli` crates are **100% deterministic, rule-based + fuzzy algorithmic**, and **offline-first**.
   - The LLM validation step exists **only** in the external `validation/` harness (never shipped to end users) to benchmark and validate our system's accuracy against gold sets.
3. **Setup scope and stopped collection, latest instruction 2026-11-02:**
   - Earlier collection instructions are historical, not continuing authority.
     Do not resume prior Aside sessions, queues, exports or receipt watchers.
   - The [search protocol](docs/06-benchmark-search-protocol.md) is a prepared
     procedure for a future user-selected collection run. B01 and subsequent
     development batches do not authorize acquisition or validation experiments.
   - Preserve all existing files and label partial, superseded and unverified
     evidence accurately. No validation or manuscript results have been produced.
   - Close completed task-owned tabs at phase boundaries. Reuse active tabs;
     do not leave idle agent tabs or close unrelated user tabs.
4. **Use `aside` CLI for Manuscript & University Library Access:**
   - Whenever you need to access a manuscript (e.g., full-text PDF/HTML to verify literature claims in M0) or access university library / subscription database resources, **use the `aside` CLI**.
   - If `aside` is not on the default shell `PATH` in a sandboxed session, check user binary locations or run outside the sandbox / ask the user for the `aside` binary path before attempting workarounds.
5. **Cross-Platform Hygiene (Windows + macOS):**
   - Never hardcode Windows path separators (`\`); always use `std::path::Path` / `PathBuf` in Rust and Tauri path APIs in TypeScript.
   - Never depend on Windows-only crates in `dedup-core` or `dedup-cli`.
   - Map keyboard shortcuts cleanly across `Ctrl` (Windows/Linux) and `Cmd` (macOS).

---

## 3. Documentation Map (`docs/`)

All gathered research, domain schemas, algorithmic specifications, and architectural plans are preserved in `docs/`:

| File | Contents |
|---|---|
| [`docs/00-index.md`](docs/00-index.md) | Master index and overview of all project documentation. |
| [`docs/01-methodology-and-literature.md`](docs/01-methodology-and-literature.md) | Formal nomenclature, peer-reviewed methodological evidence (Bramer 2016, ASySD 2023, Deduklick 2022, SRA-DM 2015, Jiang 2014, McKeown & Mir 2021, Guimarães 2022, PRISMA 2020, Cochrane Handbook), and critical review corrections. |
| [`docs/02-deduplication-rules-and-pipeline.md`](docs/02-deduplication-rules-and-pipeline.md) | Complete specification of Tier 1 Normalization, Multi-Key Blocking, Tier 2a Deterministic & Tier 2b Fuzzy Scoring, False-Positive Guards, the 5 Decision Classes, Clustering, Master Selection, Field-Level Merge, and PRISMA Audit Outputs. |
| [`docs/03-database-schemas-and-formats.md`](docs/03-database-schemas-and-formats.md) | Target database metadata schemas & RIS/internal tags across Education, Special Education, Psychology, Medicine/Allied Health, and expansion disciplines, plus file format & encoding rules. |
| [`docs/04-architecture-and-ui-spec.md`](docs/04-architecture-and-ui-spec.md) | Rust workspace layout, crate responsibilities, CLI JSON contract, Tauri 2 + React minimalist chic UI specification, and macOS readiness checklist. |
| [`docs/05-validation-and-llm-oracle.md`](docs/05-validation-and-llm-oracle.md) | External `validation/` harness design, Tier 3 LLM Oracle rubric, human dual-labeling + Cohen's kappa protocol, benchmark datasets, and target accuracy metrics. |
| [`docs/06-benchmark-search-protocol.md`](docs/06-benchmark-search-protocol.md) | Prepared field-specific research, raw-count refinement and original-export audit procedure; execution stopped. |
| [`docs/14-terra-handoff.md`](docs/14-terra-handoff.md) | Current scope, B01 launch, stopped collection state and future execution gates. |
| [`docs/reference-data/`](docs/reference-data/) | Raw OpenAlex 2023–2026 top-cited journal outputs and ERIC API yield probe logs collected during planning. |

---

## 4. How to Pick Up & Continue Development

1. Check [`DECISIONS.md`](DECISIONS.md) to review locked-in product and technical decisions.
2. Open [`TASKS.md`](TASKS.md) to find the current milestone and the exact next task (`M0` -> `M1` -> `M2` ...).
3. Execute the B-batch order in [the development plan](docs/08-development-plan.md),
   using the M0-M9 inventory for coverage. Keep `TASKS.md` updated with actual
   evidence, not intended results. The next implementation batch is B01.
