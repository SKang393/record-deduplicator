# 01 — Methodology, Nomenclature & Peer-Reviewed Literature

> **Operational Note for Agents:** When verifying full-text claims or accessing manuscripts / university library subscriptions for Milestone M0, **use the `aside` CLI** (see [`AGENT.md`](../AGENT.md)).

> **Evidence status, 2026-11-01:** This is a literature/design register. Specific
> attributions are provisional unless listed as verified in the
> [preparation review](11-preparation-review.md). Bramer's field sequence, Jiang's
> citation and general name handling, and Qi's identity were checked through
> Aside. Our numeric thresholds and name heuristics are design choices, not
> validated constants from these papers. Follow the [engine contract](10-engine-contract.md).

---

## 1. Formal Methodological Nomenclature

In information science, medical informatics, and evidence synthesis methodology, the `record-deduplicator` architecture is formally categorized as a **Hybrid Multi-Stage Deterministic and Fuzzy Record Linkage Pipeline** (or **Rule-Based Multi-Key Blocking with Approximate String Similarity Matching**).

It comprises four formal methodological components:

1. **Pre-Processing Normalization & Tokenization:**
   - Stripping database-specific indexing artifacts (e.g., whitelisted `[Abstract]` tags, trailing APA copyright notices), case folding, Unicode NFKC & diacritic normalization, HTML/MathML stripping, and resolving inverted author strings (`First Last` vs. `Last, First M.`) and hyphenated compound given names.
2. **Multi-Pass Deterministic Record Linkage & Union Blocking:**
   - Grouping candidate records via a union of shared blocking keys (normalized DOI, PMID/Accession, title prefix, rare title token stems, first-author surname + year window, ISSN-L + volume + start page) followed by sequential deterministic field combinations.
3. **Probabilistic / Approximate String Matching (Fuzzy Linkage):**
   - Computing character-level and token-level similarity metrics (normalized Levenshtein distance, Jaro-Winkler similarity, token-set Jaccard similarity, and token containment/overlap coefficients for truncated abstracts) across candidate pairs within blocks.
4. **Bibliographic Duplicate vs. Data Duplicate (Kin Study) Disambiguation:**
   - Distinguishing identical publications indexed differently across databases or publication stages (**bibliographic duplicates**, such as *Advance Online Publication / Online First* vs. final volume/issue/page assignment) from distinct publications reporting the same underlying study (**data duplicates / kin reports**, such as a conference abstract, preprint, dissertation, or working paper and a subsequent peer-reviewed journal article).

---

## 2. Core Peer-Reviewed Methodological Evidence

### 2.1 Foundational Deduplication Studies

1. **Bramer, W. M., Giustini, D., de Jonge, G. B., Holland, L., & Bekhuis, T. (2016).**
   *De-duplication of database search results for systematic reviews in EndNote.* Journal of the Medical Library Association, 104(3), 240–243.
   - **Role in Architecture:** Establishes the classic multi-stage sequential field sequence:
     1. `Author + Year + Title + Journal`
     2. `Author + Year + Title + Pages`
     3. `Title + Volume + Pages`
     4. `Author + Volume + Pages`
     5. `Year + Volume + Issue + Pages`
     6. `Title` (with manual/fuzzy verification)
     7. `Author + Year` (with manual/fuzzy verification)
   - **Verified interpretation:** The paper supplies an EndNote workflow, not a numerical zero-penalty rule. Differing pages can require manual comparison. Its 2017 correction clarifies manual review in Table 1 row C3. Missing/provisional pagination handling in our product is an explicit design choice.

2. **Hair, K., Bahor, Z., Macleod, M., Liao, J., & Sena, E. S. (2023).**
   *The Automated Systematic Search Deduplicator (ASySD): A rapid, open-source, interoperable tool to remove duplicate citations in biomedical systematic reviews.* BMC Biology, 21(1), 189. `https://doi.org/10.1186/s12915-023-01686-z`
   - **Role in Architecture:** Validates blocking pairs combined with string similarity thresholds (achieving 0.95–0.99 sensitivity and >0.99 specificity on biomedical benchmark datasets from ~1,800 to ~80,000 citations). Specifically addresses missing metadata fields and *Online First* discrepancies where publication years shift by 1–2 years between online release and print issue assignment.
   - **Open Benchmark Resource:** `https://github.com/camaradesuk/ASySD` (used as an external biomedical benchmark in M6; reimplement methodology cleanly in Rust without copying R code).

3. **Borissov, N., Haas, Q., Minder, B., Kopp-Heim, D., Gernler, M. V., Janka, H., Teodoro, D., & Amini, P. (2022).**
   *Reducing systematic review burden using Deduklick: A novel, automated, reliable, and explainable deduplication algorithm to foster medical research.* Systematic Reviews, 11(1), 172.
   - **Role in Architecture:** Demonstrates an automated pre-processing pipeline combining NLP normalization (case folding, stripping bracketed tags, normalizing author initials and reversed first/last names) with an explainable rule-based decision tree.

4. **Rathbone, J., Carter, M., Hoffmann, T., & Glasziou, P. (2015).**
   *Better duplicate detection for systematic reviewers: Evaluation of Systematic Review Assistant-Deduplication Module.* Systematic Reviews, 4(1), 6.
   - **Role in Architecture:** Quantifies the failure rate of strict exact-match tools (showing standard EndNote auto-deduplication misses up to 42.8% of true duplicates due to punctuation, capitalization, and author formatting variations) and validates iterative fuzzy field matching.

5. **Jiang, Y., Lin, C., Meng, W., Yu, C., Cohen, A. M., & Smalheiser, N. R. (2014).**
   *Rule-based deduplication of article records from bibliographic databases.* Database, 2014, **bat086**. [DOI](https://doi.org/10.1093/database/bat086).
   - **Verified scope:** General name variation, reversed names and omitted initials are discussed. The specific East Asian hyphenated given-name prefix rule is our hypothesis; the full text did not establish that attribution. See [Methods](https://pmc.ncbi.nlm.nih.gov/articles/PMC3893659/#sec2).

---

### 2.2 Additional Comparator, Evaluation & Reporting Standards Literature

6. **McKeown, S., & Mir, Z. M. (2021).**
   *Considerations for conducting systematic reviews: evaluating the performance of different methods for de-duplicating references.* Systematic Reviews, 10, 38.
   - Evaluates Ovid multifile search, EndNote, Mendeley, Zotero, Covidence, and Rayyan against a manual gold-standard benchmark set, documenting the specificity vs. sensitivity trade-offs across tools.

7. **Guimarães, N. S., Ferreira, A. J. F., Silva, R. C. R., de Paula, A. A., Lisboa, C. S., Magno, L., Ichihara, M. Y., & Barreto, M. L. (2022).**
   *Deduplicating records in systematic reviews: there are free, accurate automated ways to do so.* Journal of Clinical Epidemiology, 152, 110–115. `https://doi.org/10.1016/j.jclinepi.2022.10.009`
   - Benchmarks free automated/semi-automated deduplication tools (Rayyan, Mendeley, SR Accelerator) across systematic review datasets.

8. **Kwon, Y., Lemieux, M., McTavish, J., & Wathen, N. (2015).**
   *Identifying and removing duplicate records from systematic review searches.* Journal of the Medical Library Association, 103(4), 184–188. `https://doi.org/10.3163/1536-5050.103.4.004`
   - Evaluates time commitment, false positives (unique studies erroneously deleted), and false negatives across five deduplication workflows.

9. **Qi, X., et al. (2013).**
   *Find duplicates among the PubMed, EMBASE, and Cochrane Library databases in systematic review.* PLoS ONE, 8(8), e71838.

10. **Page, M. J., et al. (2021).**
    *The PRISMA 2020 statement: an updated guideline for reporting systematic reviews.* BMJ, 372, n71.
    - Defines the PRISMA 2020 flow diagram counts ("Duplicate records removed before screening" vs. records screened and reports linked to studies).

11. **Rethlefsen, M. L., et al. (2021).**
    *PRISMA-S: an extension to the PRISMA Statement for Reporting Literature Searches in Systematic Reviews.* Systematic Reviews, 10, 39.
    - **Item 16 (Deduplication):** Concerns reporting the process and software. The planned application will draft methods text for author review; it cannot certify methodological compliance.

12. **Higgins, J. P. T., et al. (Eds.).**
    *Cochrane Handbook for Systematic Reviews of Interventions* (v6.x), Chapter 4 (Searching for and selecting studies — Deduplication) & Chapter 5 (Collecting data — *Linking multiple reports of the same study*).
    - Establishes the authoritative distinction between removing duplicate *records* prior to screening vs. linking multiple *reports* of the same study (e.g., conference abstracts, trial registry entries, preprints, dissertations, and full journal articles).

13. **Fellegi, I. P., & Sunter, A. B. (1969).**
    *A theory for record linkage.* Journal of the American Statistical Association, 64(328), 1183–1210.
    - Theoretical foundation for probabilistic agreement/disagreement log-likelihood weights and three-zone classification (auto-link, clerical review, non-link).

14. **Christen, P. (2012).**
    *Data Matching: Concepts and Techniques for Record Linkage, Entity Resolution, and Duplicate Detection.* Springer.
    - Reference for blocking key design, pair completeness ($PC$), reduction ratio ($RR$), and clustering evaluation metrics.

15. **von Elm, E., Poglia, G., Walder, B., & Tramèr, M. R. (2004).**
    *Different patterns of duplicate publication: an analysis of articles used in systematic reviews.* JAMA, 291(8), 974–980; and **Tramèr, M. R., et al. (1997).** *Impact of covert duplicate publication on meta-analysis: a case study.* BMJ, 315, 635–640.
    - Demonstrates why covert/data duplicates and kin studies must be tracked so patient cohorts are not double-counted in meta-analyses.

---

## 3. Critical Engineering Corrections to Initial Gathered Notes

During architectural review, nine vital refinements were made to the raw gathered notes. Every agent must enforce these rules in code:

| # | Original Note / Assumption | Engineering Correction & Rule Enforced in `record-deduplicator` |
|---|---|---|
| **1** | Garbled regex `^\[.*?\ Indicate` for bracket stripping | Replace with an explicit, case-insensitive **Artifact Tag Whitelist** (`[Abstract]`, `(Abstract)`, `[Book Review]`, `[Editorial]`, `[Erratum]`, `[Published Erratum]`, `[Letter]`, `[Comment]`, `[Retraction]`, `[Translated Title]`). Never strip arbitrary `[...]` substrings. |
| **2** | "Strip bracketed text" vs. **Translated Titles** | MEDLINE and APA PsycINFO place translated titles **entirely inside square brackets** (e.g., `[Effects of intervention X on reading in struggling learners]`). Blindly stripping bracketed text deletes the entire title! **Rule:** If the entire title is enclosed in `[...]` (after stripping trailing punctuation), **unwrap** the outer brackets and mark `is_translated_title = true`. Also retain original-language title fields (`OT` / `TO` / `TT`) as secondary title candidates. |
| **3** | Suppress related reports or equate them with a study | Retain distinct reports in v1. Use typed, reviewable relationship edges, not automatic same-study connected components. No kin-suppression toggle. |
| **4** | DOI or base-DOI equality always proves identity | Preserve full normalized DOIs and suffixes. Check report-type and identifier conflicts, bibliographic sanity and cluster-wide constraints before any merge. |
| **5** | Rust or blocking guarantees 50k records in two seconds | 50k all-pairs comparisons involve about 1.25 billion pairs. Blocking can reduce work but has no universal recall or complexity guarantee. Measure real latency, candidate counts and memory with explicit overflow behavior. |
| **6** | East Asian & Hyphenated Author Names (`Kang, S. W. == Sungwoo Kang`) | Korean, Chinese, and Vietnamese given names are frequently hyphenated in one database (`Kang, Sung-Woo` -> initials `S. W.`) and unhyphenated in another (`Sungwoo Kang` -> initial `S.`). **Rule:** Generate both compound and collapsed initial representations (`S W` and `S`) and treat initial sets as compatible when one is a prefix of the other (`S` matches `S W`) alongside exact surname match. |
| **7** | Uniform year variance of $\pm 2$ years | $\pm 1\text{–}2$ years is ideal for *Online First* -> final print issue (`BIBLIOGRAPHIC_DUPLICATE`). However, `KIN_REPORT` linkages (e.g., NBER/SSRN working paper or ProQuest dissertation -> peer-reviewed journal article) routinely span **2–5 years**. **Rule:** Use decision-class-specific year tolerance windows ($\pm 2$ years for `BIBLIOGRAPHIC_DUPLICATE`, up to $\pm 5$ years for `KIN_REPORT`). |
| **8** | Strong literature attributions ("Bramer... explicitly prove", "Jiang... East Asian reversal") | In Milestone M0.3, verify exact wording against the full-text PDFs (using the **`aside` CLI**) before citing those specific claims in academic documentation or publications. |
| **9** | Tier 3 LLM Agent Validation Rule Column | Per user directive, the LLM agent rules are **strictly for our external validation harness (`validation/`)** to benchmark system output and build gold sets. They are **not** included in the shipped deduplicator application. |
