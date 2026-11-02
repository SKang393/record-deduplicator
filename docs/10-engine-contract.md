# Engine contract and specification corrections

Status: accepted design contract, 2026-11-01. Implementation is tracked in
[TASKS.md](../TASKS.md). This document resolves conflicts in the earlier research
and design notes. It describes required behavior, not implemented capabilities.

## 1. What the product identifies

- A **record** is an occurrence imported from a particular source file.
- A **report** is a publication represented by one or more records. Only records
  of the same report may enter a duplicate cluster.
- A **study** is the underlying investigation. Different reports may describe
  the same study, and one report may describe several studies.
- `KIN_REPORT` is a proposed, typed relationship between reports. It is not
  permission to suppress a report or proof of common study identity.

V1 retains every distinct related report, including preprints, dissertations,
conference abstracts, protocols, results, and updated versions. Remove the old
proposal for an automatic kin-suppression toggle from v1 scope. Relationship
edges contain type, supporting evidence, proposed/confirmed/rejected state,
review decision, and policy version. They do not create duplicate unions.

## 2. Input identity and preservation

Read input files without modifying them. Store each source's SHA-256, byte
length, logical source ID, supplied database, detected format, decoding choice,
warnings, and immutable original bytes in the local project bundle. Hashing and
copying must detect a source changed during import; do not accept a mixed read.

Record identity derives from stable logical source identity, source content hash,
and record ordinal. Logical source identities belong to the persisted input
manifest and distinguish independent database/search exports even when their
bytes are identical. Reimporting the same logical source must prompt/diagnose a
repeat; equal content alone must not erase a separately declared source or its
counts. Physical source bytes may be shared without merging provenance. Identical
records at different positions remain distinct input occurrences until explicitly
merged. Database identification is a suggestion:
RIS often cannot identify a database uniquely. Preserve unknown and user-corrected
source labels rather than guessing from the vendor.

Retain ordered raw fields, repeated tags, original byte ranges, and parsed values.
An `IndexMap<String, Vec<String>>` alone does not preserve interleaved tag order,
whitespace, encoding, or exact source bytes. Distinguish exact source recovery
from semantic export. RIS/CSV cannot represent every field; unsupported values
remain in the JSON/project sidecar and produce an export loss report.

Malformed records, undecodable bytes, limits, and unknown fields must produce
diagnostics. Nothing disappears because a parser could not interpret it. V1
blocks a final export until parse errors are fixed or explicitly acknowledged
as excluded, with separate counts. Limit XML nesting and disable external entity
resolution. Treat imported strings as data in the UI and escape CSV formula
prefixes in spreadsheet-safe exports while preserving raw values in JSON.

## 3. Identifiers and comparison evidence

| Evidence | Contract |
|---|---|
| DOI | Preserve the entire normalized DOI. Remove known resolver prefixes, trim and normalize case. Decode resolver URL encoding once; do not strip arbitrary trailing punctuation or suffixes from literal IDs. Preserve `.v1`, `.s001`, `.pub2`, and other suffixes. |
| DOI relationships | Any provider-specific base/version inference is optional relationship evidence only. Base DOI equality is never report identity. |
| PMID, PMCID, ERIC IDs | Validate conservatively without obsolete fixed-width assumptions. Preserve raw forms and warnings. Equality still requires sanity and guard checks. |
| AN, UID, UT and other accession IDs | Namespace by issuing database and ID type. The same vendor/tag/value in different databases is not identity. Unknown namespaces cannot authorize automatic merging. |
| Trial registry IDs | Candidate related-study/report evidence only. Never an automatic duplicate identity key. |
| ISBN, ISSN, journal, author, year | Supporting evidence, not unique publication identity. |
| Missing values | Unknown, not agreement. Empty strings never generate generic blocking keys or perfect similarity. |
| Conflicting valid report identifiers | Route to review or distinct. A high string score does not erase a contradiction. |

Use Unicode-preserving comparison forms. Case folding, diacritic folding, and
transliteration are different operations. Preserve non-Latin scripts; any
optional transliteration is secondary evidence. Detect editorial/type markers
before stripping whitelisted artifacts. A short initial prefix alone cannot
resolve conflicting full given names. Journal abbreviation/ISSN-L tables need
verified licenses, versioned snapshots, and unmatched fallbacks; no network lookup
occurs in the shipped matching path.

## 4. Versioned decision policy

The planned policy schema contains `schema_version`, `policy_version`, all
thresholds, normalization options, blocking rules, resource limits, and tie-break
rules. Store its canonical hash in every run. The initial conservative policy
will be committed with B03; numeric values in older notes are hypotheses until
that policy and its tests exist.

Decision order for every candidate:

1. Extract type/version/numbered-part markers and conflicting identifiers.
2. Enforce hard cannot-link constraints, including original versus correction
   notice, protocol versus results, differing parts, and incompatible versions.
3. Classify proposed report relationships without unioning or removing reports.
4. Apply exact full-identifier rules with title/type sanity and sufficient evidence.
5. Apply supported multi-field/fuzzy rules, otherwise route plausible pairs to
   review. Sparse metadata must not manufacture confidence.
6. Record class, rule ID, guard outcomes, missing evidence, and field scores.

`EXACT_DUPLICATE` and `BIBLIOGRAPHIC_DUPLICATE` are the only automatic merge
classes. `PROBABLE_DUPLICATE` stays unresolved and retained. `DISTINCT` describes
an evaluated pair, not every ungenerated pair. A retracted original article is
different from its retraction notice: two database records of that same original
may be duplicates while preserving its retracted status.

Missing or provisional pagination must not veto a well-supported Online First
match. Contradictory final pagination is still visible evidence and may trigger
review. The earlier blanket zero-penalty rule for all mismatches is superseded.

## 5. Bounded candidates and safe clusters

Use the union of multiple blocking strategies; do not promise a universal
complexity or recall bound. Common surnames and generic titles can create
quadratic blocks. Record generated pairs, maximum block sizes, candidate coverage
estimates, overflow events, and cancellation state. Resource exhaustion yields
an explicit incomplete result with unresolved records retained, never a successful
silent truncation. Candidate recall is measured separately from classifier recall.

Specify actual year-distance checks: ordinary publication candidates use a
configurable initial distance of two years and related-report candidates five.
Do not widen these accidentally through overlapping expanded buckets. Unknown
years use separately bounded strategies; they are not wildcards over all input.

Sort pairs canonically before clustering. Before accepting any cluster union,
check applicable constraints across the proposed combined members, including
pairs absent from the original candidate set. V1 requires compatible report
identity and no forbidden/ambiguous cross-member pair. If the required checks
exceed a resource cap, retain the proposed cluster for review. A-B and B-C passing
does not permit A-C to violate a guard. Partitioning and chosen master must be
stable across input discovery order and thread counts.

Manual merge decisions show conflicts and preserve all inputs and decisions.
V1 does not offer a silent override of a hard report-identity guard. Undo must
restore field alternatives, cluster membership, relationship edges, and counts.

## 6. Merge, audit, project state, and exports

Choose a master only inside an accepted same-report cluster. Completeness and
source preference are versioned tie-break rules, not permission to merge a
dissertation into an article. Backfill only compatible missing values; retain
conflicts as alternatives. Each chosen field stores source record, raw field,
normalization steps, selection rule, and prior alternatives.

Persist source manifests, canonical records, policy, review event log, relation
edges, and export manifest in a versioned local project directory. Write state
atomically, detect interrupted writes, and support schema migrations with a
backup. Use stable event sequence numbers for replay; timestamps are diagnostic.
Do not overwrite source files or an existing export without explicit choice.

Canonical outputs use UTF-8 without BOM, LF, explicit stable ordering, fixed
numeric serialization, and deterministic IDs. No absolute paths, timestamps,
elapsed times, random IDs, or nondeterministic reductions enter canonical result
bytes. Keep those in a separate run diagnostics file. Floating-point threshold
boundaries require a specified rounding/quantization rule tested on both OSes.
Determinism means the same source bytes, logical source manifest, policy, engine
version, and decision history produce the same canonical output bytes.

Reconciliation counts occurrences, not pair edges:

`accepted input records = duplicate occurrences suppressed + retained records`

`recognized source records = accepted input records + rejected/acknowledged records`

A duplicate cluster of size k contributes k-1 suppressions. Unresolved and related
reports are retained and included in retained counts, not extra additive counts.
Repeated-source warnings and unparseable fragments are separate diagnostics.
Exported screening counts must match retained records in the selected output;
other filters must be disclosed separately. PRISMA counts and methods text are
draft reporting assistance requiring author review, never a compliance guarantee.

## 7. Evidence and release gates

Tests start with adversarial synthetic cases but public accuracy claims require
independent human-adjudicated held-out data. Split by review and report/study
family before tuning, including all perturbations of a seed in one split. Keep
LLM suggestions separate and blind human reviewers to engine predictions.
Temperature zero does not make an LLM deterministic or authoritative.

Report automatic precision/recall, candidate completeness, review-assisted recall,
incorrect report merges, cluster metrics, parser losses, distinct-report retention,
and human minutes separately. For sampled pairs, save strata and inclusion
probabilities, calculate weighted estimates and uncertainty, and describe what
was not sampled. A random nonblocked sample alone does not prove recall. Report
study loss only if reference study labels support that measure.

No speed, memory, queue-size, installer-size, or accuracy target is currently
achieved. Hardware, dataset, comparator settings, versions, and full methods
must accompany measurements. See the [development plan](08-development-plan.md)
for evidence required before a CLI alpha, desktop beta, or v1 release.
