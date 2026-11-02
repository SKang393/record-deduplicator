# Documentation Index (`record-deduplicator`)

This `docs/` directory records all domain knowledge, methodological evidence, algorithmic rules, database schemas, UI specifications, and validation protocols gathered for **`record-deduplicator`**.

## Start here for implementation

- [Current preparation-only Terra handoff and next batch](14-terra-handoff.md)
- [Product positioning and competitor evidence](07-product-positioning.md)
- [Full development plan and release gates](08-development-plan.md)
- [GPT-6.1 Terra launch and handoff prompts](09-terra-execution.md)
- [Engine contract and specification corrections](10-engine-contract.md)
- [Preparation review and verification evidence](11-preparation-review.md)
- [Prepared database research and original-export protocol](06-benchmark-search-protocol.md)
- [Dated collection evidence, source tables and remaining gaps](12-collection-evidence.md)
- [Requested later model conditions and APA manuscript handoff](13-future-validation-conditions.md)

The original 01-05 documents below are research/design notes, not proof of
implementation or validated accuracy. The engine contract resolves conflicts.
The two original text files in reference-data are historical planning snapshots.
The dated collection subdirectory contains observed research and process metadata.
Document 06 is the prepared collection protocol; execution has stopped at the
user's direction. Its previous versions remain historical in reference-data.

## Contents

1. **[`01-methodology-and-literature.md`](01-methodology-and-literature.md)**
   - Formal methodological nomenclature in information science and evidence synthesis.
   - Core peer-reviewed studies validating the multi-stage pipeline (Bramer 2016, Hair 2023 [ASySD], Borissov 2022 [Deduklick], Rathbone 2015 [SRA-DM], Jiang 2014).
   - Additional comparator and reporting literature (McKeown & Mir 2021, Guimarães 2022, Kwon 2015, Qi 2013, PRISMA 2020, PRISMA-S, Cochrane Handbook Ch. 4–5, Fellegi–Sunter 1969, Christen 2012, von Elm 2004, Tramèr 1997).
   - Critical review corrections to initial assumptions and manuscript verification notes (using `aside` CLI).

2. **[`02-deduplication-rules-and-pipeline.md`](02-deduplication-rules-and-pipeline.md)**
   - End-to-end algorithmic specification:
     - **Tier 1:** Pre-processing normalization & tokenization (case folding, whitelisted bracketed artifacts vs. unwrapped translated titles, APA & publisher copyright trailers, DOI normalization, author name inversion & East Asian hyphenated given-name compatibility, pagination normalization).
     - **Tier 2a & 2b:** Multi-key union blocking and deterministic + fuzzy similarity scoring (Levenshtein, Jaro-Winkler, Jaccard, abstract containment coefficient, ISSN-L / journal abbreviation resolution, confirming-only weights for Volume/Issue/Pages).
     - **False-Positive Guards:** Protection against errata, replies, boilerplate titles, multi-part papers, protocols, and updated living reviews.
     - **5 Decision Classes, Clustering, Master Selection, Field-Level Merge & PRISMA Outputs.**

3. **[`03-database-schemas-and-formats.md`](03-database-schemas-and-formats.md)**
   - Target database indexing & metadata tag schemas for:
     - Core Education & Psychology (EBSCOhost: Academic Search Complete, Education Source, Education Full Text; Ovid / APA PsycNET / ProQuest: PsycINFO, PsycARTICLES, PsycEXTRA, ERIC, ProQuest Dissertations).
     - Medical & Allied Health expansion (MEDLINE via PubMed `.nbib` and Ovid, Embase, Cochrane CENTRAL, CINAHL).
     - Cross-disciplinary expansion (Social Welfare/Criminology/Campbell, Computer Science/IEEE/ACM/arXiv, Environmental Science/CEE, Economics/Management/NBER/SSRN, Sport Science/SPORTDiscus, Scopus, Web of Science).
   - Supported export file formats and character encodings.

4. **[`04-architecture-and-ui-spec.md`](04-architecture-and-ui-spec.md)**
   - Rust workspace architecture (`crates/dedup-core`, `crates/dedup-cli`) and Tauri 2 desktop app (`app/`).
   - Modern, minimalist, chic zero-learning-curve 3-step UI/UX specification (React + TypeScript + Vite + Tailwind CSS + `shadcn/ui` + TanStack Virtual).
   - Windows Native + macOS cross-platform readiness checklist.

5. **[`05-validation-and-llm-oracle.md`](05-validation-and-llm-oracle.md)**
   - External validation harness (`validation/`) — strictly separated from the shipped program.
   - Tier 3 LLM Agent Validation Rubric (used as an external benchmarking oracle paired with human dual-labeling and Cohen's $\kappa$).
   - Benchmark datasets (ASySD, Education/Psychology gold sets, Synthetic perturbation sets), evaluation metrics, and target accuracy thresholds.

6. **[`06-benchmark-search-protocol.md`](06-benchmark-search-protocol.md)**
   - Ten core fields plus supplemental special education; sampled database-use evidence and ranked-journal topic selection.
   - Raw sums of 10,000-20,000, all publication years, exact EBSCOhost search iterations and original RIS export verification.
   - Raw supporting data in [`reference-data/`](reference-data/).
