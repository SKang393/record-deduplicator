# Terra handoff: preparation only

Updated 2026-11-02. The user's latest instruction is: **"do not download export
files yourself. your job is to set up the system for terra."** Earlier collection
instructions do not authorize continued execution. No collection or validation
run should start automatically from this handoff.

## Ready for the user's development run

The public repository is [SKang393/record-deduplicator](https://github.com/SKang393/record-deduplicator).
Its current implementation is a Rust core/CLI foundation with help/version
commands. Parsers, matching, export, GUI and accuracy results are not implemented.
The [product comparison](07-product-positioning.md) proposes an integrated
advantage: source-format fidelity, protection of distinct related reports,
replayable provenance, and an offline CLI/desktop workflow. These are development
requirements, not measured superiority or claims of unique individual techniques.

Use **GPT-6.1 Terra, max reasoning** and the exact launch command in
[the runbook](09-terra-execution.md). The first selected batch is **B01**:
immutable source snapshots, occurrence identity, ordered raw fields, namespaced
identifiers, field origins and versioned policy/model contracts. It can use
synthetic fixtures without collecting another database record.

Copy this prompt into Terra when ready:

```text
Read AGENTS.md, AGENT.md, TASKS.md, docs/08-development-plan.md,
docs/10-engine-contract.md and docs/14-terra-handoff.md.
Implement only B01 using GPT-6.1 Terra with max reasoning.
Before editing, identify files to inspect/change, runtime impact and checks.
Preserve unrelated work. Use synthetic fixtures and existing local evidence.
Do not run library searches, exports, downloads, mailbox retrieval, collection
watchers, LLM validation or manuscript writing. Do not resume old Aside sessions.
Meet every B01 acceptance case and run Cargo formatting, lint and tests.
Update TASKS.md with actual commands/results and the next batch. Stop after B01.
Use only Sungwoo Kang <200436385+SKang393@users.noreply.github.com> for commits,
with no agent attribution trailers.
```

Follow dependency order **B01-B06, B08a, B07, B09-B11, B08b, B12, B13**, one bounded
batch at a time. B08a prepares human-reference rules and freezes family-disjoint
splits before corpus-based tuning. B08b evaluates the completed B11 production
candidate. The [development plan](08-development-plan.md) gives ownership paths,
acceptance cases and validation for each batch. Implementing a validation harness
is distinct from authorizing actual model experiments.

## Stopped collection and preserved evidence

Five task-owned Aside sessions were stopped and archived. The receipt watcher
exited. The browser inventory fell from 37 to 3 tabs during cleanup; the remaining
tabs were pre-existing or not proven task-owned. Do not resume old sessions or
queued instructions. When a future browser task ends, close its owned tabs at
the phase boundary and leave unrelated user tabs alone.

Private originals and logs remain in `testdata/benchmarks/2026-11-02/` and `.local/`,
both excluded from Git. The [public evidence report](12-collection-evidence.md)
contains allowlisted research/process metadata, not raw bibliographic exports.
The local approval plan is marked `execution_revoked_preparation_only`; its
historical query choices must not be treated as runnable approvals.

- Research metadata covers 110 field assignments from 109 distinct sampled
  reviews, ten assignments per field. These are purposive sample frequencies.
  One pharmacy journal's eligible 2025-2026 topic-paper slot remains unresolved.
- Ten field panels reached the raw-count target in historical observations.
  Computer science did not: CS-X03 had 25,354 and CS-X04 had 3,978. No next
  refinement was executed after the stop. Counts are not completed corpora.
- Six EBSCO-only MED/NEU pilot files contain 31,240 records and remain separate.
  Fourteen expanded EBSCO files contain 20,872 records with unresolved applied
  expander settings. They are historical evidence, not final corpus inputs.
  Replacement `R02` jobs were planned before collection stopped.
- Existing native receipts are incomplete. PubMed NBIB originals are retained
  without conversion. Some WoS, Embase, CENTRAL and ScienceDirect files exist;
  source-level receipt ledgers retain their exact batch coverage. A Scopus
  browser completion event is not a verified corpus file.
- Previously acknowledged or ambiguous export requests may still deliver mail.
  Do not retrieve it or retry sends during preparation. A future selected run
  must reconcile saved acknowledgments before making any additional request.
- Springer native CSV is the chosen format, but no CSV was collected. Its
  observed search export cap was 1,000; individual RIS is another route. Full
  coverage, possible partitioning and other supported routes remain unresolved.
  The current integrity helpers support RIS/NBIB, not CSV.

All field corpora remain **not admitted**. Original-byte and receipt checks do
not establish production parser fidelity or deduplication accuracy.

## Future collection run, only when selected by the user

Use the [prepared protocol](06-benchmark-search-protocol.md) and existing ledgers
before choosing the smallest unresolved field/source batch. Do not redo the
sampled-review or journal work without a documented reason. Use **Aside CLI** for
all manuscript and Purdue/library actions.

The fixed design is ten core fields plus supplemental special education,
10,000-20,000 records as the sum of individual source counts before cross-source
deduplication, and all publication years through the search date. Keep platform
internal deduplication, source field-scope differences and exported losses
explicit. Record UTC/Eastern times, exact expressions, applied settings, source
counts, requests, receipts, batch ranges, original filenames, sizes and SHA-256.

Resolve EBSCO applied settings and the CS topic scope before final admission.
Preserve original RIS, PubMed NBIB and any later native Springer CSV. Establish
CSV integrity handling before claiming a CSV source complete. Never replace a
missing original with reconstructed records or infer delivery from a request.

## Later validation and manuscript

Use the [validation conditions](13-future-validation-conditions.md) and
[validation design](05-validation-and-llm-oracle.md). The five requested provider,
model, effort and context settings remain unverified planned conditions.
Independent human-adjudicated references, frozen splits, source rights and the
completed program precede reported model/engine results. Keep all LLM activity
outside the shipped application in `validation/`.

Only after completed validation, write the APA manuscript from the recorded
methods and actual outputs. Apply the Humanizer skill while preserving evidence,
numbers, uncertainty and citations. No manuscript or validation result exists yet.

## Handoff checks recorded on 2026-11-02

- `python scripts/check_docs.py`: passed local links in 45 Markdown documents.
- Python compilation of the repository helpers: passed. They inspect local
  files; none of the six collection helpers initiates acquisition.
- `build_collection_report.py`, `summarize_expanded_counts.py` and
  `index_original_receipts.py`: exit 0 against existing private evidence.
- `verify_original_exports.py`: exit 2, correctly indicating incomplete
  collection. Forty indexed original files passed receipt/hash/boundary checks;
  zero field corpora were admitted. The verified ScienceDirect file remains in
  its separate native ledger until a physical-source adapter is added.
- Public-file review found no raw export files, private recipient addresses,
  known browser session IDs or signed-download credentials in the publish set.

Detailed local command logs are preserved in `.local/preparation-final/` and the
private integrity ledger. The first expanded-integrity run exposed an adapter
omission for Embase component ranges; mapping the recorded component range keys
resolved it. No original data was changed. Program runtime files were unchanged,
so these checks do not claim new Rust, GUI or accuracy acceptance.
