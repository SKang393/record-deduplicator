# Product positioning and competitor evidence

Reviewed 2026-11-01 against official documentation. This is a capability review,
not a hands-on benchmark. The repository currently contains a foundation build.
The capabilities below for record-deduplicator are proposed until tested.

## Product promise

**Remove duplicate records while preserving distinct reports, with an inspectable
history of every retained, merged, and linked record.**

Serve librarians and systematic reviewers with source-aware imports, beginning
with the documented EBSCO export dialects. The revised benchmark covers ten
core fields plus supplemental special education; broad validation coverage is
planned, not established by the collection itself. Deliver a local desktop application and CLI
that work from the same engine, with no runtime LLM or network dependency.
The intended advantage is the integrated workflow and its published evidence,
not a claim that individual matching techniques are novel.

## Existing capabilities we must respect

| Comparator and primary source | Documented capability | Implication for this project |
|---|---|---|
| [Zotero](https://www.zotero.org/support/duplicate_detection) | Duplicate candidates from bibliographic fields, master selection, alternate field values, preserved tags/collections. | Field merging and desktop operation are established. The support page has an old revision date, so verify installed behavior before making limitation claims. |
| [EndNote 2025 Windows](https://docs.endnote.com/docs/endnote/2025/v1/windows/en/content/04search/finding_duplicaterefs.htm) | Configurable duplicate fields, highlighted differences, side-by-side review, Keep/Skip. | Configuration and comparison UI alone are not unique. This source covers a named edition, not all current editions. |
| [Rayyan](https://www.rayyan.ai/systematic-auto-resolver) | Exact/fuzzy rules, normalization, source priority, screening-decision protection, resolution tracking. | Explainability and automated review assistance are existing capabilities. Vendor accuracy rankings are not independent evidence. |
| [Covidence matching](https://support.covidence.org/knowledge_base/topics/how-does-covidence-detect-duplicates) and [PRISMA](https://support.covidence.org/knowledge_base/topics/export-prisma) | Automatic import deduplication using specified bibliographic criteria; duplicate totals in PRISMA output. | Online First/title variation is a comparative test hypothesis. PRISMA support is not unique. |
| [ASySD repository](https://github.com/camaradesuk/ASySD), [workflow](https://camaradesuk.github.io/ASySD/articles/basic-deduplication.html), [updates](https://camaradesuk.github.io/ASySD/articles/search-updates.html) | Open R package and Shiny application, automated matching and manual review, source preference and merged-ID traceability. | Open source, local execution, traceability, and updates already exist. Test native-install convenience and audit detail. |
| [SRA Deduplicator](https://help.sr-accelerator.com/docs/tools/deduplicator/) | Same-record duplicate definition, confidence groups, Not Dupe, Split Group, undo/redo, save/resume, metadata backfill and prior-library comparison. | Report distinction, reversibility, enrichment, and group repair are not standalone firsts. |
| [revtools](https://github.com/mjwestgate/revtools) | Bibliographic import, deduplication and interactive review in R, with RIS/BibTeX/CIW/CSV support. | Scripting and interactive bibliographic review are established. |
| [Deduklick](https://www.risklick.ch/deduklick) | Official page describes automated, transparent AI deduplication. | Current offline support, licensing, detailed rule access and exports are not established by this page. Unknown does not mean absent. |

## Four differentiators to build and prove together

| Proposed distinction | Concrete behavior | Required proof |
|---|---|---|
| Source-aware export fidelity | ERIC page counts, PsycINFO trailers, translated titles, truncated abstracts, ambiguous names, explicit vendor dialect status and reconciliation of search versus delivered records. | Synthetic regression cases plus representative exports; per-dialect parse-loss and matching results across the prespecified fields. |
| Protected report relationships | Separate duplicate clusters and typed relationships; full DOI identity; no trial-ID-only merge; retain protocols, dissertations, abstracts and versions. | Hard-negative fixtures, three-record bridge tests, and independent report-level labels. |
| Replayable field provenance | Immutable sources, selected and alternate field values, rule/guard reasons, config hashes, reversible decisions and deterministic results. | Replay after save/reload, undo recovery, source hash checks, cross-OS/thread-count golden hashes. |
| Practical local delivery | One core in CLI and native Windows/macOS app, no required account or network during import-to-export. | Clean-machine install and offline native workflow tests on both platforms. |

## Release scope and comparison protocol

V1 is a focused deduplication tool, not a replacement for reference managers,
screening platforms, full-text retrieval, study extraction, or meta-analysis.
Import user exports and return standard files and auditable decisions. Leave
automatic kin suppression, cloud collaboration, online metadata enrichment,
embeddings in production, and unvalidated discipline presets out of v1.

Use identical permitted datasets and record comparator version, edition,
configuration, automated versus reviewed mode, and operator time. First measure
false automatic report merges and missed duplicates, then review burden, format
loss, runtime and peak memory. Freeze a held-out set before tuning. Reproduce
baseline results without copying implementation code or assuming dataset rights
from a repository's software license. If access to a commercial comparator is
missing, mark that comparison untested.

Do not publish "first", "best", "more accurate", "zero false positives",
"50,000 records in two seconds", "PRISMA compliant", or "supports every vendor"
without the relevant evidence. A defensible present-tense statement is:

> The project is building an auditable local deduplication workflow with an
> explicit handling of source formats. Its report-protection and provenance
> requirements are specified, and comparative validation is planned.
