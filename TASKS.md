# Development Backlog & Next Tasks (`TASKS.md`)

This file tracks the ordered engineering tasks for **`record-deduplicator`**.

## Execution tracker (2026-11-01)

Follow [the full development plan](docs/08-development-plan.md) and
[Terra 6.1 runbook](docs/09-terra-execution.md). The B-batches are the executable
order; the M0-M9 inventory below remains for coverage. The
[engine contract](docs/10-engine-contract.md) overrides conflicting older rules.

| Batch | Status | Next evidence |
|---|---|---|
| B00 foundation/repository/plan | COMPLETE for preparation scope | Local checks and Windows/macOS foundation CI passed; see [preparation review](docs/11-preparation-review.md). |
| B01 source/model contracts | NEXT | Stable source IDs, namespaced IDs, ordered raw fields and tests. |
| B02 RIS decoding/import | Pending B01 | Loss-aware parser fixtures and preserved source bytes. |
| B03 safe exact matching | Pending B02 | Identifier/guard/bridge hard-negative tests. |
| B04 CLI alpha/export/replay | Pending B03 | Actual fixture import-to-export, counts, hashes and undo. |
| B05a-c remaining formats | Pending B04 | Per-format parser/export evidence. |
| B06 normalization/candidates | Pending B05a | Candidate recall, resource limits and stress results. |
| B07 fuzzy/cluster safety | Pending B06 | Guard regressions and cross-OS determinism. |
| B08 independent evaluation | Design can start; runner after B04 | Human reference labels, held-out splits, metrics and uncertainty. |
| B09 desktop shell | Pending B04 | Native Windows/macOS import/cancel smoke tests. |
| B10 review/recovery UI | Pending B07/B09 and B08 schema/protocol | Native undo/resume/export evidence; completed B08 evaluation gates beta acceptance. |
| B11 packaging | Pending B10 | Installers, clean-machine workflows, signing evidence. |
| B12 release evidence | Pending B08/B11 | Verified claims and independent evaluation; optional external LLM. |
| B13 expansion | Post-v1 | Measured benefit and no safety regressions. |

External gates: proprietary sample exports and independent human labels are not
supplied; signing credentials and native desktop test hosts are not yet assessed.
M0.3 is partial: Bramer, Jiang and Qi were checked via Aside, but remaining
specific manuscript attributions are pending. Do not block synthetic B01-B04 on
these gates. No new topic/keyword investigation is authorized.
> **Agent Note:** Read [`AGENT.md`](AGENT.md) first. Do **not** perform autonomous keyword/topic investigation (already completed in [`docs/06-benchmark-search-protocol.md`](docs/06-benchmark-search-protocol.md)). When accessing manuscripts or university library resources, use the **`aside` CLI**.

---

## Immediate Next Tasks (Start Here: Milestone M0 -> M1)

### Milestone M0: Repository Initialization, Workspace Scaffolding & Literature Verification

- [x] **Task M0.1 — Initialize Git & GitHub Repository (`SKang393` identity)**
  - Evidence: public `SKang393/record-deduplicator`, `main` pushed at `f529699`; GitHub maps both author and committer to `SKang393`. License detected as Apache-2.0.
  - Run `git init -b main` in `d:\Codex\Deduplicator`.
  - Verify `git config user.name` is `Sungwoo Kang` and `git config user.email` is `200436385+SKang393@users.noreply.github.com` (maps to `SKang393` on GitHub).
  - Add `LICENSE` (Apache-2.0) and `.gitignore` (Rust `/target`, Node `node_modules/`, `dist/`, `.DS_Store`, `Thumbs.db`, raw un-anonymized subscription exports if restricted, `.env`).
  - Create remote repository when ready: `gh repo create SKang393/record-deduplicator --public --source=. --remote=origin` and push commits **strictly as `SKang393`** (no `Co-authored-by` trailers).

- [x] **Task M0.2 — Scaffold Cargo Workspace & Directory Layout**
  - Evidence: both crates compile/lint locally; help/version and unsupported-command smoke checks pass. Windows, macOS and documentation CI passed at `f529699`. No native GUI or algorithmic acceptance is claimed.
  - Create root `Cargo.toml` workspace containing:
    - `crates/dedup-core` (Rust library crate: canonical data model, parsers, normalizer, blocker, scorer, guards, classifier, union-find clusterer, merger, exporter, PRISMA audit generator).
    - `crates/dedup-cli` (Rust binary crate: CLI runner with deterministic JSON/RIS/CSV outputs).
  - Create directory placeholders:
    - `app/` (Tauri 2 + React + TypeScript + Tailwind + `shadcn/ui` frontend, initialized in M7 or early skeleton).
    - `validation/` (External Python validation & LLM oracle harness — never shipped).
    - `testdata/fixtures/` (Synthetic unit/golden test fixtures committed to git) and `testdata/benchmarks/` (Real database exports provided by user).
    - `.github/workflows/ci.yml` (GitHub Actions running `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` on `windows-latest` and `macos-latest`).

- [ ] **Task M0.3 — Verify Methodological Claims Against Full-Text Manuscripts (via `aside` CLI)**
  - Before publishing formal manuscript claims in academic outputs, verify the specific attribution statements flagged in [`docs/01-methodology-and-literature.md`](docs/01-methodology-and-literature.md) (e.g., Bramer et al. 2016 exact sequence and Ahead-of-Print pagination notes; Jiang et al. 2014 East Asian author reversal handling; Qi et al. 2013; Fellegi & Sunter 1969).
  - **Tool instruction:** Use the **`aside` CLI** when accessing full-text manuscripts or university library subscriptions.

- [ ] **Task M0.4 — Ingest User-Provided Sample Database Exports**
  - Place user-provided sample exports (50–200 records per database across Academic Search Complete, Education Source, Education Full Text, ERIC, APA PsycINFO, APA PsycARTICLES, plus any medical/expansion samples) into `testdata/`.

---

### Milestone M1: Canonical Record Model & Multi-Format Parsers (`crates/dedup-core`)

- [ ] **Task M1.1 — Define Canonical Bibliographic Record Struct (`model.rs`)**
  - Implement `RawRecord` and `CanonicalRecord` in `crates/dedup-core/src/model.rs`:
    - `internal_id` (deterministic UUID/hash or sequential index), `source_file`, `source_db` (auto-detected vendor/database), `raw_format`.
    - Identifiers: `doi`, `pmid`, `pmcid`, `eric_id` (`EJ`/`ED`), `accession_number` (`AN`/`UID`/`UT`/`CN-`), `pii`, `trial_registry_ids` (`NCT`, `ISRCTN`, `ICTRP`), `isbn`, `issn_print`, `issn_electronic`, `issn_l`.
    - Titles: `title_primary`, `title_translated`, `title_original_language` (`OT`/`TO`).
    - Authors: `Vec<AuthorName>` storing raw string, parsed family name, given names, initials, ORCID/Author ID (`AI`).
    - Publication metadata: `journal_full`, `journal_abbrev`, `pub_year`, `pub_date_raw`, `volume`, `issue`, `start_page`, `end_page`, `article_number`, `page_count`, `pub_type` (`PT`/`TY`/`M3`), `peer_reviewed_flag`, `language`.
    - Content: `abstract_text`.
    - Preservation: immutable source bytes/hashes and record offsets plus ordered repeated raw fields. Parsed maps are derived views; semantic export and byte-exact source recovery are separate guarantees.

- [ ] **Task M1.2 — Encoding & Line-Ending Normalization Layer (`encoding.rs`)**
  - Auto-detect UTF-8 (with/without BOM), UTF-16LE/BE (common in EndNote exports), and Windows-1252 fallback using `encoding_rs`.
  - Normalize mixed `CRLF` / `LF` / `CR` line endings before parsing.

- [ ] **Task M1.3 — Multi-Dialect RIS Parser (`parsers/ris.rs`)**
  - Parse standard RIS (`TY  - ` ... `ER  - `) plus vendor dialects:
    - EBSCOhost (`AN`, `TI`, `T1`, `AU`, `A1`, `AB`, `N2`, `SO`, `JO`, `JF`, `T2`, `PY`, `Y1`, `VL`, `IS`, `SP`, `EP`, `DO`, `SN`, `M3`, `N1`).
    - Ovid / APA PsycNET (`UID`, `DO`, `TI`, `OT`, `AU`, `AB` with copyright trailer, `YR`/`PY`).
    - ERIC (`AN` with `EJ######`/`ED######`, `SP` page counts like `15p.`).
    - Embase, Scopus, Web of Science, ProQuest Dissertations.
  - Handle multi-line continuation fields properly.

- [ ] **Task M1.4 — PubMed / MEDLINE (`.nbib`), CSV, EndNote XML & BibTeX Parsers**
  - `parsers/nbib.rs`: Parse NLM MEDLINE tagged format (`PMID`, `OWN`, `STAT`, `TI`, `TT`, `PG`, `LID`/`AID` `[doi]`/`[pii]`, `AB`, `FAU` + `AU`, `LA`, `PT`, `DEP`, `TA`, `JT`, `IS`, `VI`, `IP`, `DP`, `PMC`).
  - `parsers/csv.rs`: Auto-map column headers for Scopus CSV, Web of Science CSV, and Covidence CSV.
  - `parsers/endnote_xml.rs` & `parsers/bibtex.rs`: Parse EndNote XML (`<record>`) and BibTeX entries.
  - Add golden-file tests in `crates/dedup-core/tests/parsers_test.rs`.

---

### Milestone M2: Tier 1 Pre-Processing & Normalization (`crates/dedup-core/src/normalize/`)

- [ ] **Task M2.1 — Text, Unicode, HTML & Artifact Whitelist Normalizer (`normalize/text.rs`)**
  - Apply Unicode NFKC, diacritic folding (`unicode-normalization`), HTML entity decoding (`&amp;`, `&#x...;`), and inline HTML/MathML tag stripping (`<i>`, `</i>`, `<sup>`, `<sub>`).
  - Expand common Greek symbols (`α` -> `alpha`, `β` -> `beta`, `γ` -> `gamma`).
  - **Translated Title Rule (D-007):** If a title is wholly enclosed in `[...]` (e.g., `[Effects of X on Y]`), unwrap the outer brackets and flag `is_translated_title = true`.
  - **Artifact Whitelist Rule (D-007):** Strip only whitelisted bracketed/parenthetical database tags (`[Abstract]`, `(Abstract)`, `[Book Review]`, `[Editorial]`, `[Erratum]`, `[Published Erratum]`, `[Letter]`, `[Comment]`, `[Retraction]`, `[Translated Title]`) while recording corresponding document-type flags for false-positive guards!

- [ ] **Task M2.2 — Abstract Trailer & Structured Header Normalizer (`normalize/abstract_text.rs`)**
  - Strip APA copyright trailer: `\(PsycInfo Database Record \(c\) \d{4} APA, all rights reserved\)` (case-insensitive, including `PsycINFO` / `PsycARTICLES` variants).
  - Strip Elsevier, Wiley, Springer, Taylor & Francis, and SAGE trailing copyright statements (`© 20\d{2}...`, `Copyright (C)...`).
  - Strip leading `Conference abstract.` or structured labels (`BACKGROUND:`, `OBJECTIVE:`, `METHODS:`, `RESULTS:`, `CONCLUSIONS:`) when building normalized comparison tokens.

- [ ] **Task M2.3 — DOI, Identifier & Pagination Normalizer (`normalize/identifiers.rs`)**
  - DOI: normalize known resolver prefixes/encoding and case while preserving the full identifier and all suffixes. Provider-specific relationship inference must not create DOI equality.
  - Pages: expand elided ranges (`102-18` -> `102-118`), distinguish supplement pages (`S12`, `Suppl 1`), article numbers (`e0123456`), and ERIC document page counts (`15p.`).

- [ ] **Task M2.4 — Author Name Parser & Permutation Generator (`normalize/authors.rs`)**
  - Split multi-author strings on `;`, ` and `, `&`, or repeated tags; strip ORCID/affiliation footnotes.
  - Parse `Last, First M.` and `First M. Last` into canonical components.
  - Generate both forward and reversed token permutations (`Smith, John` <-> `John Smith`) and **East Asian hyphenated/compound given-name initial prefixes** (`Sung-Woo Kang` -> `S W` and `S`; `Sungwoo Kang` -> `S`, treating `S` as prefix-compatible with `S W`).
  - Handle nobility particles (`van der`, `de la`, `von`) and suffixes (`Jr.`, `III`).

- [ ] **Task M2.5 — Journal Title & ISSN-L Resolution (`normalize/journal.rs`)**
  - Normalize ISSN / eISSN and map to linking `ISSN-L` + embedded NLM `J_Medline` / ISO 4 abbreviation lookup table.

---

### Milestone M3: Multi-Key Blocking & Tier 2a Deterministic Linkage (`crates/dedup-core/src/blocking.rs`)

- [ ] **Task M3.1 — Multi-Pass Union Blocking Engine**
  - Generate candidate pairs using multiple blocking keys with measured completeness and explicit resource/overflow limits. Pair completeness is a target, not a guarantee; common keys can still create quadratic blocks.
    1. Exact full normalized DOI, preserving all suffixes.
    2. Exact PMID / ERIC ID / database-namespaced accession number. Trial Registry IDs generate relationship candidates only.
    3. Normalized Title Prefix (first 30 alphanumeric characters) + Year window ($\pm 2$, or $\pm 5$ for kin candidates).
    4. Sorted 3 rarest title token stems ( handles word-order/subtitle shifts and translated title variations).
    5. First Author normalized surname + Year ($\pm 2$).
    6. Normalized Journal (`ISSN-L` or abbreviated stem) + Volume + Start Page.
- [ ] **Task M3.2 — Sequential Deterministic Passes (Bramer-inspired + ASySD)**
  - Implement fast deterministic passes over blocked pairs before expensive string similarity calls.

---

### Milestone M4: Tier 2b Fuzzy Scoring, False-Positive Guards & Clustering (`crates/dedup-core/src/scoring.rs`)

- [ ] **Task M4.1 — Per-Field Similarity Metrics**
  - Title: Normalized Levenshtein ratio (`rapidfuzz` / `strsim`) + token-set Jaccard + subtitle split awareness.
  - Authors: Hungarian / greedy best-match alignment across author lists using Jaro-Winkler on surnames + initial-prefix compatibility.
  - Abstract: Token cosine similarity + **containment coefficient** ($\frac{|A \cap B|}{\min(|A|, |B|)}$) to handle truncated legacy ERIC/EBSCO abstracts without penalty.
  - Volume / Issue / Pages: missing/provisional pagination must not veto supported Online First matches; contradictory final pagination remains visible evidence and may require review.
- [ ] **Task M4.2 — False-Positive Guards (`guards.rs`)**
  - Block auto-merging on:
    1. Errata / Corrigenda / Retractions (`Correction to:`, `Erratum:`, `Retraction of:`, `PT = Published Erratum`).
    2. Replies / Commentaries / Letters (`Reply to`, `Comment on`, `Re:`, `Author response`).
    3. Boilerplate / Generic Titles (`Editorial`, `Introduction`, `Book Review`, `Preface`, `Letter to the Editor`, `Foreword`, `In This Issue`).
    4. Multi-part studies (`Part I` vs. `Part II`, `Study 1` vs. `Study 2`, Roman/Arabic numeral differences).
    5. Study Protocol vs. Empirical Trial Results.
    6. Updated Living / Cochrane Systematic Reviews (same title, different year/DOI version).
    7. High title similarity but completely disjoint author sets (unless DOI/PMID match + verified).
- [ ] **Task M4.3 — Decision Classifier & Union-Find Clustering (`classify.rs`, `cluster.rs`)**
  - Classify candidate pairs into:
    1. `EXACT_DUPLICATE` (Auto-merge)
    2. `BIBLIOGRAPHIC_DUPLICATE` (Online First <-> Final Print, Cross-DB indexing variant; Auto-merge)
    3. `PROBABLE_DUPLICATE` (Grey zone -> Human Review Queue)
    4. `KIN_REPORT` (Conference abstract <-> Journal article, Preprint/Working paper <-> Journal article, Dissertation <-> Article; Link & Retain Both by default)
    5. `DISTINCT`
  - Check all applicable cannot-link and identity constraints across proposed cluster members before accepting any union. Retain ambiguous or resource-limited clusters for review.

---

### Milestone M5: Master Selection, Field-Level Merge, PRISMA Audit & CLI (`crates/dedup-cli`)

- [ ] **Task M5.1 — Master Record Selection & Field-Level Metadata Enrichment (`merge.rs`)**
  - Rank cluster members by publication stage (`Peer-Reviewed Final Journal Article` > `Advance Online Publication` > `Preprint / Working Paper / Report` > `Conference Abstract`), then by metadata completeness score (presence of DOI, full pagination, complete abstract, full author given names, ISSN).
  - Enrich the master record by backfilling any missing fields (e.g., DOI, abstract, ORCID, ISSN, pages) from merged duplicates.
  - Record full provenance (`merged_from_ids`, `source_databases`, `matched_rules`, `similarity_scores`).
- [ ] **Task M5.2 — Exporters, Audit Log & PRISMA 2020 / PRISMA-S Generator (`export.rs`)**
  - Export:
    - Deduplicated records (`.ris`, `.csv`, `.nbib`, `.xml`, `.bib`).
    - Removed duplicates file.
    - Human review queue file (`PROBABLE_DUPLICATE`).
    - Kin study linkage table (`KIN_REPORT`).
    - Machine-readable `audit_log.json` and human-readable `audit_log.csv`.
    - Auto-generated PRISMA 2020 flow counts (records per DB, total before dedup, exact/bibliographic duplicates removed, records remaining for screening, linked kin reports) + draft PRISMA-S Item 16 methods paragraph.
  - Verify **100% determinism** (identical input + config + version = byte-identical output).
- [ ] **Task M5.3 — Build `dedup-cli` (`crates/dedup-cli`)**
  - Provide subcommands: `dedup run`, `dedup inspect`, `dedup stats`, `dedup verify-determinism`.

---

### Milestone M6: External Validation Harness & LLM Oracle (`validation/`)

- [ ] **Task M6.1 — Benchmark Runner & Pairwise/Cluster Metrics (`validation/`)**
  - Build Python evaluation scripts in `validation/` that invoke `dedup-cli` and compute candidate completeness, automatic/review-assisted recall, precision, cluster errors, incorrect report merges, review burden, and runtime with uncertainty. Claim study losses only when reference study labels exist.
- [ ] **Task M6.2 — External LLM Validation Oracle & Human Dual-Labeling Agreement**
  - Implement optional advisory LLM suggestions strictly inside `validation/` for explicitly authorized data, with structured output, abstention, cached response provenance and agreement analysis. Human-adjudicated labels remain authoritative; temperature zero is not determinism. See [`docs/05-validation-and-llm-oracle.md`](docs/05-validation-and-llm-oracle.md).

---

### Milestone M7–M9: Tauri 2 Minimalist GUI, Packaging & Cross-Discipline Profiles

- [ ] **Task M7.1 — Tauri 2 + React + `shadcn/ui` 3-Step Desktop App (`app/`)**
  - Implement the zero-learning-curve 3-step workflow specified in [`docs/04-architecture-and-ui-spec.md`](docs/04-architecture-and-ui-spec.md):
    1. **Step 1: Add Files** (Drag-and-drop zone, auto-detected DB & record counts).
    2. **Step 2: Review Queue** (Clean side-by-side diff cards for `PROBABLE_DUPLICATE` and `KIN_REPORT` only, word-level diff highlighting, plain-English explanations, `←` Keep Both / `→` Merge / `↓` Skip keyboard navigation, full Undo stack).
    3. **Step 3: Export & PRISMA Summary** (One-click RIS/CSV export + copyable PRISMA 2020 flow numbers & PRISMA-S methods text).
- [ ] **Task M8.1 — Cross-Platform Packaging & CI Releases**
  - Configure Tauri bundler for Windows (`.exe` / `.msi` via WebView2) and macOS (`.dmg` / `.app` Universal binary).
- [ ] **Task M9.1 — Discipline-Specific Profiles**
  - Add tuned presets for Education/Psychology, Biomedical/Allied Health (MEDLINE/Embase/CENTRAL/CINAHL), Computer Science (arXiv/IEEE/ACM), and Economics/Social Policy (NBER/SSRN/Campbell).
