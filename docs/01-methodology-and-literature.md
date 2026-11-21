# 01 — Methodology, Nomenclature & Peer-Reviewed Literature

These sources describe bibliographic record linkage and evaluations of other tools.
For the behavior of this CLI, see [Comparing records](02-deduplication-rules-and-pipeline.md).

## 1. Terms used in record linkage

Blocking uses selected fields to form a smaller set of record pairs for comparison.
A blocking rule can miss copies with differing metadata. Exact field comparisons,
string-similarity measures and human inspection can only assess the pairs they receive.

Two database records can represent one publication even if one gives the online-first
year and the other gives the print year. A protocol and a results article may share
a study identifier while representing separate publications. Reference labels for
deduplication therefore distinguish records of the same report from distinct reports
of the same study.

## 2. Core Peer-Reviewed Methodological Evidence

### 2.1 Foundational Deduplication Studies

1. **Bramer, W. M., Giustini, D., de Jonge, G. B., Holland, L., & Bekhuis, T. (2016).**
   *De-duplication of database search results for systematic reviews in EndNote.* Journal of the Medical Library Association, 104(3), 240–243.
   - **EndNote field sequence:** [Table 1](https://jmla.mlanet.org/ojs/jmla/article/view/24/18) compares seven sets of fields in order:
     1. `Author + Year + Title + Journal`
     2. `Author + Year + Title + Pages`
     3. `Title + Volume + Pages`
     4. `Author + Volume + Pages`
     5. `Year + Volume + Issue + Pages`
     6. `Title` (with manual verification)
     7. `Author + Year` (with manual verification)
   - The paper describes an EndNote workflow, not a numerical score for missing pages. Differing pages can require manual comparison. The [2017 correction](https://jmla.mlanet.org/ojs/jmla/article/download/128/257?inline=1) clarifies which references to check in Table 1, row C3.

2. **Hair, K., Bahor, Z., Macleod, M., Liao, J., & Sena, E. S. (2023).**
   *The Automated Systematic Search Deduplicator (ASySD): A rapid, open-source, interoperable tool to remove duplicate citations in biomedical systematic reviews.* BMC Biology, 21(1), 189. `https://doi.org/10.1186/s12915-023-01686-z`
   - Hair and colleagues reported sensitivity of 0.95 to 0.99 and specificity above 0.99 for ASySD on the biomedical datasets they evaluated. These are results for ASySD, not for this application. [ASySD software](https://github.com/camaradesuk/ASySD).

3. **Borissov, N., Haas, Q., Minder, B., Kopp-Heim, D., Gernler, M. V., Janka, H., Teodoro, D., & Amini, P. (2022).**
   *Reducing systematic review burden using Deduklick: A novel, automated, reliable, and explainable deduplication algorithm to foster medical research.* Systematic Reviews, 11(1), 172.
   - **Role in Architecture:** Demonstrates an automated pre-processing pipeline combining NLP normalization (case folding, stripping bracketed tags, normalizing author initials and reversed first/last names) with an explainable rule-based decision tree.

4. **Rathbone, J., Carter, M., Hoffmann, T., & Glasziou, P. (2015).**
   *Better duplicate detection for systematic reviewers: Evaluation of Systematic Review Assistant-Deduplication Module.* Systematic Reviews, 4(1), 6.
   - Across four searches, [Rathbone and colleagues](https://doi.org/10.1186/2046-4053-4-6) reported that their module found an average of 42.8% more duplicate records than EndNote's automatic deduplication. This is a relative increase in detections, not the percentage of all true duplicates that EndNote missed.

5. **Jiang, Y., Lin, C., Meng, W., Yu, C., Cohen, A. M., & Smalheiser, N. R. (2014).**
   *Rule-based deduplication of article records from bibliographic databases.* Database, 2014, **bat086**. [DOI](https://doi.org/10.1093/database/bat086).
   - The authors discuss reversed names and omitted initials. Their results do not establish that two hyphenated names sharing an initial identify the same author. See [Methods](https://pmc.ncbi.nlm.nih.gov/articles/PMC3893659/#sec2).

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
   *Find duplicates among the PubMed, EMBASE, and Cochrane Library databases in systematic review.* PLoS ONE, 8(8), e71838. [Article](https://doi.org/10.1371/journal.pone.0071838).

10. **Page, M. J., et al. (2021).**
    *The PRISMA 2020 statement: an updated guideline for reporting systematic reviews.* BMJ, 372, n71.
    - Defines the PRISMA 2020 flow diagram counts ("Duplicate records removed before screening" vs. records screened and reports linked to studies).

11. **Rethlefsen, M. L., et al. (2021).**
    *PRISMA-S: an extension to the PRISMA Statement for Reporting Literature Searches in Systematic Reviews.* Systematic Reviews, 10, 39.
    - **Item 16 (Deduplication):** Report how duplicates were removed and which software was used. Software-generated methods text cannot certify compliance.

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
