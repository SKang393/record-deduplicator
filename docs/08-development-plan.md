# Full development plan

Owner: SKang393. Date: 2026-11-01. Execution target: GPT-6.1 Terra.
This is the execution order; [TASKS.md](../TASKS.md) retains the original M0-M9
task inventory. [Engine contract](10-engine-contract.md) resolves older design
conflicts. [Terra runbook](09-terra-execution.md) contains launch and handoff prompts.

## Starting state and delivery boundaries

The foundation has two Rust workspace members, a truthful help/version CLI,
documentation checks, and Windows/macOS CI. No parser, matcher, benchmark, or GUI
is implemented. See [preparation evidence](11-preparation-review.md) for actual
checks. Current scaffolding introduces no third-party runtime dependency.

| Delivery | Batches | User-visible result | Acceptance gate |
|---|---|---|---|
| CLI alpha | B01-B04 | RIS import, conservative exact-identifier matching, retained conflicts, reversible audit, RIS/JSON output. | Real end-to-end fixture run, source hashes unchanged, count reconciliation and no unsafe merges in adversarial suite. No fuzzy-accuracy claim. |
| Evaluation-ready CLI | B05-B08 | NBIB and broader formats, bounded fuzzy candidates, guarded clusters, independent evaluation. | Held-out metrics with uncertainty, deterministic artifacts on Windows/macOS, documented format coverage and limitations. |
| Desktop beta | B09-B10, B08 acceptance | Add files, inspect/resolve, undo/resume, export through the same engine. | Native smoke evidence on both platforms, recovery/accessibility checks and completed independent evaluation. |
| V1 release | B11-B12 | Installable artifacts, published evidence and documented operating limits. | Release gates below. Blocked signing/data/OS evidence remains explicitly blocked. |
| Later expansion | B13 | New discipline profiles and search-update workflow. | Measured incremental value without relaxing safety or contaminating held-out evaluation. |

No calendar estimate is asserted before B04 produces an actual vertical slice.
Use one bounded batch per Terra run, splitting a batch further if it cannot be
implemented and verified together. Do not start all milestones at once.

```mermaid
flowchart LR
  B00[Foundation] --> B01[Source and model]
  B01 --> B02[RIS import]
  B02 --> B03[Safe exact matches]
  B03 --> B04[CLI alpha and audit]
  B04 --> B05[NBIB and formats]
  B05 --> B06[Normalization and candidates]
  B06 --> B07[Fuzzy and cluster guards]
  B07 --> B08[Independent evaluation]
  B04 --> B09[Desktop shell]
  B09 --> B10[Review and recovery]
  B07 --> B10
  B08 -. Schema and protocol only .-> B10
  B08 --> B12
  B10 --> B11[Native packaging]
  B11 --> B12[Release evidence]
  B12 --> B13[Measured expansion]
```

Human-label design and permitted sample collection can proceed alongside B01-B07.
Missing proprietary data blocks vendor support and accuracy claims, not synthetic
implementation. Exact manuscript attributions remain gated by verification via
Aside; they do not block code that is clearly identified as our design choice.

## Batch contracts

Paths listed below are **planned ownership paths**, not claims those modules
already exist. Create only what the selected batch needs. Each batch adds named
regression tests to the existing Cargo suite or an explicitly introduced tool.

### B01: Source, canonical record and policy contracts

- Depends on: foundation. Maps to M1.1 and the early part of M5.1.
- Own: `crates/dedup-core/src/model.rs`, `source.rs`, `policy.rs`, corresponding
  tests, and synthetic fixture metadata. Lead integrates `lib.rs`, manifests and
  lockfile. Choose `serde`/`serde_json` and SHA-256 crate only after checking current
  stable APIs, compatible licenses, and edition/MSRV requirements.
- Implement immutable source snapshots, content/record identity, ordered raw
  fields, multi-valued namespaced IDs, raw versus comparison values, diagnostics,
  five classes, typed relation edges and field origins. Version schemas.
- Acceptance: identical bytes and logical source manifest produce stable IDs;
  reimport of that source is diagnosed; independently declared database exports
  with identical bytes retain separate occurrence IDs and source counts;
  identical records at different ordinals remain separate; same AN
  in two database namespaces does not compare equal; raw unknown tags survive.
- Validate: `cargo test --workspace --locked` plus format/lint gate. Save actual
  cases and result paths in TASKS before marking complete.

### B02: Loss-aware decoding and RIS import

- Depends on B01. Maps to M1.2-M1.3.
- Own: `encoding.rs`, `parsers/mod.rs`, `parsers/ris.rs`, parser tests, small
  synthetic RIS fixtures with provenance and expected records.
- Implement BOM-aware UTF-8/UTF-16, explicit warned legacy fallback, CRLF/LF/CR,
  continuation lines, repeated/unknown tags, malformed/truncated records,
  maximum record/field sizes, progress and cancellation hooks. Database origin
  can be unknown or supplied; do not infer database from a filename as fact.
- Acceptance: decode fixtures on both OSes; original bytes recoverable; ordered
  repeated tags retained; malformed inputs return useful diagnostics without
  panics or silent record loss. A parser cannot silently return zero records for
  a recognized broken file. Real dialect support waits for real sample evidence.
- Validate: Cargo suite with parser golden cases and source-hash assertions.

### B03: Conservative exact-identity pipeline

- Depends on B02. Maps to selected M2, M3.2, M4.2-M4.3.
- Own: identifier/text comparison modules, `guards.rs`, `classify.rs`,
  `cluster.rs`, and versioned conservative policy. No fuzzy auto-merge yet.
- Match full DOI/PMID/ERIC identifiers with compatible type and sufficient
  bibliographic sanity. All identifier conflicts and sparse cases remain for
  review. Trial IDs produce relationships only. Check every proposed union for
  cannot-link conflicts and preserve unresolved members.
- Acceptance fixtures: DOI URL versus literal; `.v1` versus `.v2`; original versus
  erratum/retraction notice; Part I versus Part II despite copied DOI; protocol
  versus results sharing NCT; cross-database AN collision; A-B/B-C bridge with
  A-C conflict; missing-title same DOI; retracted original duplicated across DBs.
- Validate: Cargo suite. No false automatic merge in these named hard negatives.

### B04: First useful CLI, merge, export and replay

- Depends on B03. Maps to M5.1-M5.3, initially RIS and JSON only.
- Own: `merge.rs`, `export/`, `audit.rs`, `project.rs`, CLI command modules and
  integration tests. Introduce a CLI parser dependency only with fit rationale.
- Implement `dedup run`, `inspect`, `stats`, `verify-determinism` and a documented
  decision-application/replay interface. Final flags and JSON schema must be
  committed with this batch; they are not available in today's help-only CLI.
- Read multiple sources into a versioned local project; emit retained and
  suppressed records, unresolved queue, relations, field provenance, canonical
  manifest and diagnostics. Handle output collisions, interrupted writes and
  source/output path overlap. No source mutation. Keep relative source identities
  out of machine-specific result bytes.
- Acceptance: two RIS files through real CLI produce correct records and k-1
  suppression counts; conflicts/kin/unresolved remain; decisions replay after
  reload; undo restores prior state; failures return nonzero exit codes; repeated
  runs yield identical canonical bytes; original source hashes unchanged.
- Validate: Cargo integration suite and one actual fixture import-to-export run.
  Record command, version/policy hashes, counts and artifacts. This is the first
  CLI alpha gate, not evidence of broad real-world accuracy.

### B05: Format coverage in three small sub-batches

- Depends on B04. Maps to M1.4 and missing WoS/TSV work in M1.
- B05a owns NBIB parser/export plus full/abbreviated author pairing, PMIDs,
  date variants, language/title alternatives and publication-status fields.
- B05b owns CSV/TSV mapping UI-independent schema and WoS tagged `.ciw` import.
  Resolve ambiguous headers visibly. Protect spreadsheet exports from formula
  execution. Test delimiters, quoting, line breaks, Unicode and empty columns.
- B05c owns EndNote XML and BibTeX import/export, entities, nesting/size limits,
  multiline/braced values and nonstandard fields. No external XML entities.
- Every sub-batch adds format-specific fixtures and reimport tests, with explicit
  loss reports for unrepresentable values. Unknown raw fields remain in JSON.
  Publish a tested format/dialect table, not a universal vendor-support claim.
- Validate: Cargo parser/export integration suite for that sub-batch. Do not
  release a listed format whose representative smoke import has not succeeded.

### B06: Normalization and bounded candidate generation

- Depends on B05a; B05b/c may proceed independently after interfaces settle.
  Maps to M2.1-M2.5 and M3.1.
- Own: `normalize/`, `blocking.rs`, normalization/candidate fixtures.
- Implement Unicode-preserving keys, marker-aware title cleanup, scoped abstract
  trailers, author ambiguity, Online First metadata, journal fallbacks and
  versioned optional lookup data with proven redistribution rights.
- Use full-identifier, title, author/year, rare-token and journal/page candidates
  with measured block sizes. Relation-only candidates stay separate. Specify
  exact year boundary behavior, missing fields, stable pair ordering, resource
  limits, progress and cancellation. Never silently drop excess candidates.
- Acceptance: non-Latin titles remain nonempty; initials do not override
  conflicting full names; `15p.` is page count; translated titles are preserved;
  unknown-year/generic-title/common-author stress data returns bounded or explicit
  incomplete results; known positives measure candidate recall independently.
- Validate: Cargo suite and a recorded release-build candidate stress run with
  input size, candidate count, peak memory and elapsed time. No 2-second promise.

### B07: Fuzzy scoring and constrained clusters

- Depends on B06. Maps to M4 and remaining M5 behavior.
- Own: `scoring.rs`, policy thresholds, guard/classifier/cluster extensions and
  adversarial fixtures. Do not alter parsers in this batch unless a tested defect
  blocks these cases.
- Implement per-field scores, missing-evidence handling, abstract containment,
  author alignments, conflict-aware Online First matching and explained review
  routing. Thresholds are tuned only on development data. Formalize score
  rounding, stable reductions and deterministic master selection.
- Acceptance: all B03 negatives remain protected; missing abstracts are not
  perfect matches; copied titles with disjoint authors retain records; generic
  titles, supplements, versions and multipart studies do not bridge clusters.
  Thread counts and OSes produce identical canonical fixture outputs.
- Validate: Cargo suite plus serial/parallel and Windows/macOS golden artifact
  comparisons. A matching failure blocks accuracy claims, not just a test badge.

### B08: Independent benchmark harness

- Depends on B04 for runner, B07 for final evaluation. Maps to M6.1.
- Own only `validation/`, human-label templates, permitted dataset manifests and
  reviewed evidence reports. No production dependency on Python or model APIs.
- Create independent reference labels, review/family-level splits, blinded dual
  labeling and adjudication, disagreement/abstention records, sampling strata and
  inclusion probabilities. Pin comparator versions/settings and dataset hashes.
- Measure automatic precision and recall, candidate completeness, cluster errors,
  incorrect report merges, retained kin reports, review-assisted outcomes, unique
  records/cluster queue size, reviewer minutes, parse losses, runtime and memory.
  Include uncertainty and sample sizes; report unevaluated strata.
- Acceptance: validate metric calculations against a tiny hand-calculated
  confusion matrix including a blocking miss and an erroneous cluster bridge.
  Frozen held-out data is not used to choose thresholds. A report is reproducible
  from CLI outputs and saved reference labels without an LLM call.
- Validate: harness self-check and frozen-fixture evaluation using commands
  introduced/documented in this batch. These scripts do not exist yet.

### B09: Native desktop shell and import

- Depends on B04 stable contracts. Maps to first part of M7.
- Own `app/` frontend and Tauri host. Lead owns shared Cargo configuration.
- Initialize current Tauri 2/React/TypeScript/Vite/Tailwind/shadcn/TanStack Virtual
  with lockfile and explicit dependency licenses. Check official prerequisites
  for MSVC/WebView2 on Windows and Xcode tools/WKWebView on macOS.
- Bridge directly to core using versioned commands/events. Run work off the UI
  thread; show progress, cancellation, parse warnings and source confirmation.
  Restrict Tauri capabilities and filesystem scope; render imported markup as
  text. No network capability or telemetry is needed for core workflows.
- Acceptance: native Windows and macOS app imports the B04 fixture with the same
  IDs/counts; cancel leaves a recoverable project; unsupported files fail clearly;
  keyboard focus, scaling and light/dark views are checked with evidence.
- Validate: frontend type/build checks and native smoke artifacts. Browser-only
  screenshots do not establish native webview or packaged-app acceptance.

### B10: Review, recovery and reporting UI

- Implementation depends on B07/B09 and B08 schema/protocol contracts, not
  unavailable final human labels. Completed B08 evaluation gates desktop-beta
  acceptance and release. Maps to remaining M7.
- Own review/export UI, project persistence integration, accessibility tests.
- Implement side-by-side field differences, conflict/guard explanations,
  merge/keep/link/skip, undo/redo, session resume, source/field provenance, virtual
  tables, keyboard shortcuts and draft PRISMA counts/methods. Bind decisions to
  input/policy versions so stale decisions cannot silently apply to changed data.
- Acceptance: save, close, reopen, replay, undo and export preserve the same
  results; crash/interrupted-write recovery does not lose sources; unresolved
  pairs remain in screening output; counts reconcile; Ctrl/Cmd shortcuts do not
  fire inside text inputs; 50k-row interaction is measured, not assumed.
- Validate: meaningful frontend tests plus a native import-review-undo-resume-
  export scenario on both platforms, with artifact hashes and screenshots.

### B11: Packaging, installation and supply-chain checks

- Depends on B10. Maps to M8.
- Own release workflow, bundle config, installation docs and release manifest.
- Build Windows installer and macOS arm64/x86_64 or verified Universal bundles.
  Verify actual CPU slices. Pin release tools/dependencies, inspect licenses,
  package contents and SBOM, exclude `validation/`, private data and credentials.
  Produce checksums, source revision and policy/schema versions.
- Code-sign/notarize when credentials exist; otherwise label unsigned review
  builds and mark public distribution gate blocked. Never fabricate signing or
  install evidence. Release credentials do not enter the repo or forks' CI.
- Acceptance: install, offline import/review/export, upgrade preserving projects,
  and uninstall without destroying user data on clean Windows and macOS systems.
- Validate: both native builds plus recorded install workflows and file hashes.
  CI compilation alone does not satisfy installation or human workflow checks.

### B12: Release evidence and optional LLM assistance

- Depends on B08 and B11. Maps to M6.2 and publication/release closeout.
- Re-run a frozen evaluation only for changed engine/policy or unresolved defects.
  Publish versioned metrics, methods, limits, comparator conditions and admitted
  public fixtures. Audit every README claim against evidence.
- Optional external LLM label suggestions live only in `validation/`, are opt-in
  for authorized data, use structured output, allow abstention, and preserve
  provider/model/prompt/response/cost provenance. Temperature zero is not a
  reproducibility guarantee. Human-adjudicated truth remains authoritative.
- Acceptance: claims trace to evidence; no unreviewed proprietary data or raw
  manuscript text is published; production artifacts and dependencies contain no
  oracle/API keys; all release gates below are satisfied or release stays blocked.

### B13: Post-v1 expansion

- Depends on measured need after v1. Maps to M9.
- Add discipline profiles only with separate held-out improvement and no safety
  regression. Add explicit search-update/import-history workflows with stable
  prior decisions and well-defined counting. Consider additional formats and
  provider relations individually. Cloud services, semantic production matching,
  study-level synthesis and report suppression require new decisions and scope.

## Release gates and external blockers

| Gate | Required evidence | Current state |
|---|---|---|
| Source preservation | Original-byte recovery, hashes unchanged, undo and replay | Not implemented |
| Report safety | Every hard-negative fixture passes; zero known unsafe auto-merges in held-out safety cases | Not implemented |
| Matching quality | Proposed auto-merge precision >=0.995 and candidate completeness >=0.995, with denominators, intervals and per-stratum results | Targets, unmeasured; report uncertainty and do not release on inadequate evidence |
| Recall and workload | Automatic and review-assisted recall separately; target combined recall >=0.96; measured review minutes and queue size | Targets, unmeasured; old <2% queue goal is exploratory |
| Determinism | Same canonical hashes across repeated runs, thread counts and Windows/macOS | Not implemented |
| Native delivery | Actual offline workflows, recovery and installer checks on both OSes | Not implemented |
| Data rights | Admitted fixtures, permitted comparator inputs, reviewed public artifacts | Proprietary samples and human reference labels not supplied |
| Distribution | Correct identity, licenses, checksums, signing/notarization evidence | Repository license prepared; release credentials not assessed |

Threshold point estimates alone are insufficient; if confidence intervals or
rare high-impact subgroups are inconclusive, expand evaluation or narrow the
release claim. Never claim study-loss rates from report-only labels.

## Integration and validation discipline

For a broad batch, use one lead integrator and up to three available subagents:
data/parsers, matching/safety, and validation/UI as relevant. Assign nonoverlapping
files; shared models, manifests, lockfiles, TASKS and commits belong to the lead.
Use waves instead of concurrent edits to shared files. If delegation is
unavailable, continue single-agent and state that limitation.

Existing checks, runnable from the repository root:

```text
python scripts/check_docs.py
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Run the appropriate suite after an integrated batch, not after every line.
Future batch-specific commands must be created, run and recorded before a task
is checked off. On failure, record exact command, error, likely cause and files.
Stop only the dependent work; continue independent authorized tasks. Do not
publish a release or claim a feature merely because a scaffold builds.

Architecture references: [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html),
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Resolve actual
dependency versions and APIs at implementation time and commit their lockfiles.
