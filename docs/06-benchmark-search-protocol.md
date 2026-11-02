# Benchmark search and RIS collection protocol

Protocol: RD-COLLECTION-2026-11-02-v2. Status: **execution stopped; preparation only**.
The latest user instruction supersedes earlier collection authority: do not run
searches, request exports, download files or retrieve pending export mail.
The procedure below is retained for a future user-selected collection run, not
as an instruction to execute it now. See [the current handoff](14-terra-handoff.md).

Earlier on 2026-11-02, the user confirmed the counting basis and date coverage,
and selected commonly used native databases alongside EBSCOhost.
The [EBSCO-only protocol](reference-data/search-protocol-2026-11-02-v1.md)
and its collected files remain pilot evidence. Expanded-panel counts determine
final field admission; existing pilot files are preserved, not silently reused
as the final expanded corpus. The previous five-field proposal is preserved as a
[historical snapshot](reference-data/search-protocol-2026-11-01.md). Its ERIC
probes and 2023-2026 topic counts do not establish this collection.

## Scope and fixed choices

| ID | Field | Role |
|---|---|---|
| MED | Medicine | Core |
| NEU | Neuroscience | Core |
| PH | Public health | Core |
| PSY | Psychology | Core |
| PHA | Pharmacy/pharmacology | Core |
| ENG | Engineering, including general, chemical, energy and materials | Core; record the journal-category mapping explicitly |
| HS | Health science | Core; distinguish allied health, nursing and rehabilitation from medicine |
| ENV | Environmental science | Core |
| EDU | Education and educational research | Core |
| CS | Computer science | Core |
| SPED | Special education | Supplemental; report separately from the core aggregate |

The target for each field is **10,000-20,000 raw records**, calculated by summing
counts from all individually searched native and EBSCO sources before our
cross-source deduplication. Native integrated collections can already suppress
records internally; record that behavior and do not claim recovery of their
pre-ingestion records. Search all records available through the observation date,
using all publication years. Preserve already indexed forthcoming issue dates.
The 2025-2026 window applies to topic-selection evidence, not the retrieved
benchmark records. Retain cross-database duplicates and distinct related reports.
No model validation, performance experiment or manuscript results are produced
in this phase.

## Database-use evidence

Build a documented purposive sample targeting ten completed systematic reviews
or meta-analyses per field. Prefer 2025-2026 publications; log older substitutions
and their reason. Save each discovery query, date, selection order and exclusion.
Exclude protocols, withdrawn reports and narrative reviews from the database-use
denominator. A narrative paper may still provide topical evidence separately.

For each eligible review, record DOI or stable ID, title, authors, journal, year,
field assignment, Methods/supplement URL, section/page/table locator, exact
reported database/platform labels, normalized labels, and observation timestamps.
Read manuscripts through **Aside CLI**. A search snippet alone cannot verify a
Methods extraction. Missing/inaccessible methods are unknown, not absence of a
database. Keep overlapping reviews identifiable across field samples.

Count a database at most once per review. Report n/N and the included review IDs
for every frequency. Call these frequencies in the sampled reviews, not
population-wide estimates of database use. Keep PubMed, MEDLINE, Embase and
Embase-hosted MEDLINE distinct. EBSCOhost, Ovid and ProQuest are platforms unless
a specific database is named. An unspecified EBSCOhost search cannot establish
use of Academic Search Complete or another particular database.

Map each commonly reported source to Purdue access through EBSCOhost or its
native interface. Do not relabel MEDLINE as PubMed, or a general index as Scopus,
Web of Science, Embase or CENTRAL. After frequency extraction and the user
amendment, the expanded panel includes named databases, bibliographic collections,
domain digital libraries and publisher search collections reported by at least
3/10 sampled reviews, plus explicitly identified EBSCO supplements and the
user-provided six-source education set. This is an analyst-selected rule for a
purposive sample, not a population prevalence threshold. Search engines remain
in the frequency table but are excluded from the primary enumerable export
corpus because stable complete result enumeration and bulk exports are not
established. Access and export gaps remain explicit and cannot become zero counts.
The source panels are fixed before expanded-panel yield refinement.

## Journal and recent-paper evidence

For every field, record ten ranked journals with ranking source, metric,
completed metric year, subject area/category, rank, journal name, ISSN and source
URL. Prefer a reproducible latest completed-year SCImago SJR table; a JCR
category is acceptable where it matches the field better. Disclose metric changes
and category limitations. Do not infer a top-ten rank from reputation, an impact
factor on a publisher page, or a hand-picked list.

For umbrella fields, document how component categories were combined, resolve
repeated journals by ISSN, and rank by the same metric. Keep narrow categories
explicit rather than calling them an entire discipline. A special-education
ranking requires an appropriate category or a disclosed alternative sampling
frame. Preserve blocked ranking access as an unresolved item.

Inspect 2025-2026 papers from these journals and target one supported topic seed
per journal. Record the article title, DOI, publication date, citation/read
metric, metric provider, observation date, within-source sorting/selection rule,
and keywords supported by the paper. Use citation and readership measures as
separate metrics. A paper without a verifiable popularity metric may be recorded
as topic evidence, but cannot be called highly cited/read. Save exceptions when a
journal has no eligible paper or source metrics are inaccessible.

Record first-online and issue/volume dates separately when they differ. A
2025-2026 issue-year paper can provide a seed even if first online in 2024, but
that exception must be explicit. The OpenAlex date-filtered selections and a
publisher's "Most Read in 2025" list have different selection frames. Neither
should be described as the same citation ranking or first-online cohort.

Choose one coherent topic per field after reviewing this evidence. Record why it
fits the field and the database coverage. Do not mix unrelated topics simply to
hit a numerical target. The original proposal's AI-only-in-education rule is not
a requirement of this revised collection. Cross-field topic overlap is allowed
but must be identified when preparing later evaluation splits.

## Purdue native and EBSCOhost search procedure

Use **Aside CLI browser** for every manuscript and Purdue/library action, including
EBSCOhost, native source searches, publisher abstracts and downloads. Record the actual interface and institutional route. Never put
session cookies, signed URLs, institutional account details or the recipient
email address in public documentation.

1. Select the prescribed native/EBSCO sources and record exact names, collections
   and interface IDs. Save the selection before the first query. WoS Core
   Collection is one source with its selected editions recorded. Embase.com
   integrated Embase/MEDLINE/Classic is one source; never add overlapping
   membership counts as separate source totals.
   Label any added EBSCO subject index as an analyst-selected supplement, separately
   from a database named in the sampled reviews. Native access supplies common
   sources missing from EBSCO; the supplement is not their equivalent.
2. Record search mode and every expander/limiter. For EBSCO, prefer Boolean/Phrase mode,
   explicit title/abstract fields, no full-text-only limiter, no language,
   peer-review or publication-type restriction, and no publication-year cutoff.
   If the interface prevents a setting, record the actual behavior.
3. Start with a broad concept/synonym OR block. Add a coherent AND block such as
   population, intervention/material, objective/outcome, setting or experiment
   type only when the raw summed count exceeds 20,000. Record every intermediate
   query and result, including zero or low-yield attempts. Do not discard failed
   attempts or retrospectively invent a search sequence.
4. Translate the shared concept blocks into actual source-specific field codes or
   interface selectors. Save the exact executed expression, not merely a portable
   sketch. Record unequal field scope: PubMed tiab also includes other abstract
   and author-keyword material; a native title/abstract/keyword or full-text search
   is not identical to EBSCO TI OR AB. Check how
   truncation, phrases, nesting and automatic subject expansion behave.
5. Capture EBSCO's per-database facet counts and the combined displayed count.
   Record whether the facet is before or after suppression; do not assume this.
   Run the same expression and settings individually in each selected database
   to establish independent raw counts when facet semantics are uncertain.
6. Calculate `raw_total = sum(selected_native_and_EBSCO_source_counts)` only when
   every selected source has an observed count. A blocked source stays unknown.
   Also retain EBSCO's
   combined total and observed duplicate-removal setting. A difference between
   totals alone is not proof of how EBSCO deduplicates.
7. Stop narrowing when the raw sum is 10,000-20,000. If a coherent topic remains
   below 10,000, broaden the most recent concept or record a reasoned alternative
   supported by the trend evidence. Recheck counts immediately before export.
8. Freeze the concept blocks, every translated expression, source panel, settings
   and query-history IDs. All source exports for one field use that finalized
   concept set; document syntax/field-scope translations and time gaps. Re-run
   all affected sources after a substantive query revision, then check the total
   before exporting. No field is accepted from a partial source sum.

The search is a deliberate construction of deduplication test data. It is not
claimed to be an exhaustive clinical or disciplinary systematic-review search.
Tuning to corpus size must be disclosed in the later validation manuscript.

## RIS export and receipt

Prefer a separate original **RIS** export for each source so inter-source duplicates
remain available and source provenance is explicit. Use the actual metadata
options exposed by EBSCO and record whether abstracts/all fields are included.
Do not claim a full-record export if the interface provides no such guarantee.
In a future collection run explicitly selected by the user, use EBSCO email
exports to the connected user's Gmail and verify receipt there. Do not retrieve
mail or download exports during setup. Native platforms may provide direct downloads instead; record
that delivery route. PubMed's observed original is NBIB, not RIS. The user
explicitly chose to retain original NBIB alongside the other sources' RIS files;
do not convert it for this collection. Never call a converted file an original
vendor RIS.
The user selected native CSV for Springer Nature Link after its search-results
export exposed a 1,000-record limit and individual-article RIS. Preserve native
CSV if that future route is used. Full-coverage partitioning or another supported
bulk route remains unresolved; no CSV has been collected. Individual RIS is an
alternative, not an executed complete-set export. Never relabel CSV as RIS.
A source with no established complete export route remains an explicit coverage gap.

The observed EBSCO bulk dialog on 2026-11-02 states a maximum of 25,000 records
per export and 100,000 in 24 hours. It also warns that delivered counts can be
lower because of duplicates or publisher restrictions. These are observed UI
limits, not a guarantee of delivery. The former EBSCO-only panel would exceed
100,000 at the eleven-field minimum. Under the expanded panel, only its EBSCO
portion consumes that quota; the 31,240 requested pilot records still count in
the current window. Track every request and native-platform batch limit. Use
additional windows if needed; do not change accounts or bypass limits.

For each export record field/query ID, database, selected/requested count,
request UTC and Eastern time, format, metadata settings, confirmation text,
email receipt time/message ID, attachment or download-link identity, download
time, original filename, local path, byte size, SHA-256, decoded encoding, RIS
start/end record counts, and any discrepancy. Keep expiring links private.
Download only observed provider links or real attachments through the supported
access path. Never manufacture a RIS file from snippets to replace a missing
export. A sent request is not a delivered or verified file.

The admitted dataset consists of verified original per-source exports with
explicit original-versus-derived format status. Record
raw UI counts, requested counts, delivered records and excluded/unavailable
records separately. Preserve original bytes. Any combined convenient copy must
have its own manifest and must not replace the individual sources.

## Audit trail and completion

Private evidence and RIS files live in `testdata/benchmarks/2026-11-02/`, which
is ignored by Git. `collection-manifest.json` records the protocol choices;
`events.jsonl` is append-only. Research extracts and browser evidence are in
`evidence/`, originals in `ris/`. Public documentation may contain reviewed
bibliographic citations, methods, query strings, counts and artifact hashes,
without proprietary abstracts, private mail identifiers or session URLs.

Every observed event needs UTC and America/New_York timestamp, actor/tool,
source/locator, action, field/database/query/export IDs, result and evidence path.
Distinguish observation time from an article date or an email's sent time.
Corrections append a superseding event rather than erasing the original event.

A field is complete only when database-use evidence, ten-journal ranking, recent
paper/topic evidence, every search trial, final in-range all-source raw counts,
and verified source exports with the recorded format decisions are reconciled. Otherwise name the incomplete element and exact
access, quota, evidence or delivery issue. Never fill a missing count with zero.

After collection, revise the development and evaluation plans against observed
RIS dialects, metadata gaps, scale and platform losses. Keep a frozen copy of the
collection manifest before tuning algorithms. Later model comparisons follow
[the planned conditions](13-future-validation-conditions.md). The APA manuscript
will be written after program and validation results exist, with the Humanizer
skill applied to prose while preserving sources, numbers and limitations.
