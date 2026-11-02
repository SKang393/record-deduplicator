# Collection evidence and current limitations

Collection date: 2026-11-02. Status: **stopped at the user's direction; partial evidence only**.
Current work is Terra preparation. No searches, exports, downloads or mailbox
retrieval may continue under the earlier instructions. Existing originals and
logs are preserved. No final field corpus is admitted. See [the current handoff](14-terra-handoff.md).
The [execution-state record](reference-data/collection-2026-11-02/execution-state.json)
supersedes any historical export-approval labels in the dated tables.
The [collection protocol](06-benchmark-search-protocol.md) fixes the raw-count
target, all-years coverage, source selection and audit requirements.

## Database use in the sampled reviews

The Methods sample contains **110 field assignments and 109 distinct reviews**,
ten per field. One review contributes to both education and special education;
its DOI and both assignments remain in the ledger. This is a purposive,
accessible-full-text sample, not a probability sample or a systematic estimate
of database use across a discipline. Methods statements were inspected through
Aside CLI. The source lists are authors' reported searches, not independent
verification that those original searches were executed.

| Field | Frequently named databases in the ten sampled reviews | Other commonly named sources, kept distinct |
|---|---|---|
| Medicine | PubMed 8/10; Embase 7/10; CENTRAL 4/10; MEDLINE 3/10 | Web of Science, unspecified indexes, 6/10 |
| Neuroscience | PubMed 10/10; Embase 5/10; PsycINFO, CENTRAL and Scopus each 2/10 | Web of Science 6/10 |
| Public health | Embase and Scopus each 6/10; MEDLINE and PubMed each 5/10 | Web of Science 6/10 |
| Psychology | PubMed 8/10; PsycINFO and Embase each 4/10; MEDLINE and Scopus each 3/10 | Web of Science 9/10 |
| Pharmacy/pharmacology | Embase 9/10; MEDLINE 5/10; PubMed 4/10 | Web of Science 4/10 |
| Engineering | Scopus 9/10; PubMed 2/10; Embase 1/10 | Web of Science 8/10; ScienceDirect 6/10 |
| Health science | Embase and PubMed each 7/10; CINAHL 6/10; Scopus 5/10 | Web of Science, unspecified indexes, 6/10; Core Collection explicitly named in another review |
| Environmental science | Scopus 8/10; Academic Search Complete, Environment Complete and GreenFILE each 1/10 | Web of Science 5/10; Google Scholar 4/10 |
| Education/educational research | ERIC 5/10; PsycINFO and Scopus each 4/10 | Web of Science 3/10 |
| Computer science | Scopus 7/10; PubMed 2/10 | Web of Science and IEEE Xplore each 6/10; ScienceDirect 4/10; ACM Digital Library and SpringerLink each 3/10 |
| Special education, supplemental | ERIC 7/10; PsycINFO and Scopus each 6/10; PubMed 3/10 | Web of Science 4/10 |

Every denominator, review ID, DOI, source label and Methods locator is available
in the [review sample](reference-data/collection-2026-11-02/review-sample.csv),
[source-use extraction](reference-data/collection-2026-11-02/review-source-use.csv)
and [complete frequencies](reference-data/collection-2026-11-02/source-frequencies.csv).
One engineering review has inconsistent source lists; the frequency table
separates reported and unambiguous counts. Unspecified Cochrane Library does not
imply CENTRAL, PubMed is not relabeled MEDLINE, and a combined PubMed/MEDLINE
label does not count as two separately established searches.

## Rankings, topics and coverage

The [journal table](reference-data/collection-2026-11-02/journal-rankings.csv)
records ten SJR 2025 journals for each core field. Medicine uses the broad
Medicine area. Engineering is an explicitly constructed union of Engineering,
Chemical Engineering, Energy and Materials Science. Health science is a union
of Health Professions and Nursing. These custom unions are not official field
rankings. Other mappings and observation times are retained in the table.

Broad journal classifications contain some papers that do not fit the intended
field. Four highly cited papers from journals classified under Environmental
Science were retained as ranking evidence but excluded as environmental topic
seeds. The [paper ledger](reference-data/collection-2026-11-02/topic-paper-evidence.csv)
distinguishes citation metadata, publisher readership selections, inspected
abstracts/full texts, topic-only selections, date qualifications and exclusions.
A high citation count is not evidence of a paper's methodological quality.

After the user completed Purdue authentication, the special-education ranking
was verified in JCR's Education, Special category, using 2025 JIF in the 2026
release. The table preserves the first ten displayed positions plus an eleventh
journal tied at the cutoff. The category roster displayed 63 entries; the
cutoff journal profiles reported rank 10/62. These denominators are retained as
observed rather than silently reconciled. Competition ranks calculated from
displayed rounded JIF values are labeled as derived, not official ranks.

OpenAlex's unauthenticated daily quota was exhausted during collection.
Crossref citation counts supplied later selections with their own provider,
query frame and observation dates. Citation counts across providers are not
directly interchangeable. Topic-paper evidence is present for 109 of 110 primary
journal slots, with an additional special-education tie entry. Eligibility of a
2025-2026 paper in the NTP Technical Report Series pharmacy slot remains
unresolved; the inspected official series list ended in 2024 but may be stale.
That is an unresolved slot, not proof that no eligible report exists.

Purdue EBSCOhost exposed 125 selectable databases. Several common sources from
the review samples, including Scopus, Web of Science, Embase and CENTRAL, were
absent from that picker. The user subsequently authorized their native
interfaces. The [expanded source panels](reference-data/collection-2026-11-02/expanded-source-panels.csv)
include named databases, digital libraries and publisher collections reported
in at least 3/10 sampled reviews, plus explicit EBSCO supplements and the user's
six-source education set. This analyst-selected rule was fixed before expanded
yield refinement. An added subject index remains a coverage supplement, not an
equivalent replacement or evidence that the sampled reviews used it. These
panels do not reproduce every database combination in the review sample.

## Executed searches and exports

The **preserved EBSCO-only pilot** [search ledger](reference-data/collection-2026-11-02/search-trials.csv)
contains exact executed expressions, settings, individual database counts,
combined totals and observation times. The [export-request ledger](reference-data/collection-2026-11-02/export-requests.csv)
records provider acknowledgments separately from received files.
The [received-file ledger](reference-data/collection-2026-11-02/received-exports.csv)
contains verified counts, hashes, sizes, receipt/download times and any request
discrepancy. The [tag inventory](reference-data/collection-2026-11-02/ris-tag-profile.csv)
records tag occurrence counts without publishing record contents.

The medicine pilot was narrowed from 44,929 raw results to **16,091**:
MEDLINE 12,638, CINAHL 3,036 and PsycINFO 417. Its topic is GLP-1 therapies for
obesity/weight loss, supported by the inspected 2025 Nature Reviews Drug
Discovery paper [GLP-1-based therapies for diabetes, obesity and beyond](https://doi.org/10.1038/s41573-025-01183-8).
This was a purposeful relevant selection, third by the captured journal-specific
OpenAlex citation count, not the highest-cited paper in that journal.

EBSCO's September 2026 help states that the top-line total includes duplicates;
the unique total appears after all results have loaded. Independent database
searches are still retained to establish this collection's raw sum. Its October
2026 help explains that metadata and identifiers influence duplicate suppression.
See [hit-count guidance](https://connect.ebsco.com/s/article/How-are-Hit-Counts-Calculated?language=en_US)
and [duplicate handling](https://connect.ebsco.com/s/article/How-are-duplicate-records-identified-and-removed-from-search-results-in-EBSCOs-Classic-and-New-User-Interfaces?language=en_US).

The observed bulk-export dialog limits requests to 25,000 records each and
100,000 per 24 hours. It warns that delivered counts may be lower because of
duplicates or publisher restrictions. Separate database exports preserve the
opportunity to measure cross-database duplication, but a successful request
does not prove delivery or complete record coverage.

All six medicine and neuroscience pilot files arrived through Gmail and were
downloaded through Aside. Their combined **31,240 TY and ER boundaries** match
the requested counts: medicine 16,091 and neuroscience 15,149. Every record
carries its expected database label. Original bytes and SHA-256 hashes are
recorded. These pilots remain outside the final expanded-panel corpus.

The expanded [observation ledger](reference-data/collection-2026-11-02/expanded-search-observations.csv)
contains actual source-native expressions and settings. Its [count summary](reference-data/collection-2026-11-02/expanded-count-status.json)
uses a complete sum only when every selected source has an observed count.
Replicate measurements are retained but never added twice. The
[approved queries](reference-data/collection-2026-11-02/approved-source-queries.csv)
identify historical source jobs selected before execution was stopped. This
filename does not confer current authority to export or download anything.
The [revision plan](reference-data/collection-2026-11-02/query-revision-plan.csv)
records the concept blocks, decision times and reasons. The
[platform amendments](reference-data/collection-2026-11-02/source-translation-amendments.csv)
retain rejected-expression corrections and substantive scope differences.
For example, ScienceDirect rejects `*` and more than eight Boolean connectors
in its combined field. Its CS-X03 translation omitted the broad generative-AI
alternative to fit that limit; this is explicitly a narrower source query,
not an equivalent rewrite. CS-X04 restores that alternative because the
narrower attack-method expression fits.

At the first export approval, 2026-11-02 19:44:20 UTC, the following complete
source panels met the target. Counts alone do not admit a corpus.

| Field | Accepted trial | Sources | Raw summed records | Export state at approval |
|---|---|---:|---:|---|
| Medicine | MED-X02 | 5 | 19,794 | Pending |
| Public health | PH-X02 | 7 | 15,605 | Pending |
| Health science | HS-X02 | 5 | 11,437 | Pending |
| Environmental science | ENV-X02 | 4 | 12,673 | Pending |
| Special education, supplemental | SPED-X01 | 9 | 12,743 | Pending |

Later count observations, including ten in-range field panels and the unresolved
computer-science panel, remain in the dated count summary. Refinement has stopped.
Native exports preserve RIS where offered and original PubMed NBIB, as the user
confirmed. Web of Science's observed RIS dialog permits 1,000 Full Record entries
per batch. Embase page exports produced seven 200-record batches, while its
full-set export route remained unresolved. These are partial observations, not
proof of full-source coverage. ScienceDirect's first 25-record file was verified.
A Scopus request reached a browser download-complete event before the stop, but
its file was not copied or verified; it is excluded from verified totals.

Springer search-results export exposed native CSV with a 1,000-record limit;
individual articles offer RIS. The user selected native CSV. Complete coverage
and possible alternative routes remain future tasks. No CSV file was collected.
ACM institutional access and its Title OR Abstract search echo were observed
before the stop. Authentication and access must not be presumed to persist.

An EBSCO settings audit found that an absent URL expander parameter does not prove
the applied setting. Fourteen earlier expanded EBSCO files are preserved as
historical evidence with unknown applied settings, outside the final corpus.
Replacement job IDs ending in `R02` were planned before the stop, not completed.
Equal counts with an expander on and off do not establish identical records.

The [expanded receipt ledger](reference-data/collection-2026-11-02/expanded-received-exports.csv)
and [source status](reference-data/collection-2026-11-02/expanded-export-status.csv)
track delivered files independently of requests. The [field admission status](reference-data/collection-2026-11-02/expanded-corpus-status.json)
requires all selected sources, accepted totals and original file checks.

## Reproduce the public index

These commands only inspect existing local evidence and regenerate metadata.
They do not access a browser, retrieve mail or download records. With the private
evidence present, they can be used to audit this stopped snapshot:

```text
python scripts/build_collection_report.py testdata/benchmarks/2026-11-02 docs/reference-data/collection-2026-11-02
python scripts/check_docs.py
python scripts/verify_ris_exports.py testdata/benchmarks/2026-11-02/ris > testdata/benchmarks/2026-11-02/evidence/ris-integrity.json
python scripts/reconcile_ris_receipts.py testdata/benchmarks/2026-11-02 docs/reference-data/collection-2026-11-02
python scripts/summarize_expanded_counts.py testdata/benchmarks/2026-11-02 docs/reference-data/collection-2026-11-02
python scripts/index_original_receipts.py testdata/benchmarks/2026-11-02
python scripts/verify_original_exports.py testdata/benchmarks/2026-11-02 docs/reference-data/collection-2026-11-02
```

The index generator validates ten unique review DOIs per field and arithmetic
for observed raw sums. It publishes an allowlist of bibliographic and process
metadata. Private RIS contents, mailbox identifiers, recipient addresses, signed
download links and browser session state remain excluded from Git. The private
input hashes in [the index](reference-data/collection-2026-11-02/research-index.json)
identify the exact evidence snapshots behind the public tables.

RIS/NBIB checks measure original bytes, hashes, decoding, record boundaries,
batch ranges and receipt provenance. The receipt adapter covers the dated
EBSCO, PubMed/Embase/CENTRAL and WoS schemas; other vendor receipts require an
explicit adapter before inclusion. CSV verification is not implemented yet.
Exit code 2 from the expanded verifier
means the collection remains incomplete. These checks
do not establish parser fidelity, duplicate accuracy, native application behavior
or an LLM result. Final validation follows program completion under the
[development sequence](08-development-plan.md) and
[requested later conditions](13-future-validation-conditions.md).
