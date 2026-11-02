# 06 — Benchmark Search Protocol (Public Validation Datasets)

> Historical planning snapshot. The stored citation counts and yield probes have
> not been rerun in this preparation. They are not current counts or admitted
> benchmark datasets. Use [the engine contract](../10-engine-contract.md) and
> [development plan](../08-development-plan.md) for validation and data-rights gates.

> **Note for Coding Agents:**
> This protocol and all supporting journal trend/yield data have **already been gathered**. **Do NOT spend turns re-running keyword investigations or API yield tests by yourself.** When accessing manuscripts or university library subscriptions, **use the `aside` CLI** (see [`AGENT.md`](../../AGENT.md)).

---

## 1. Purpose & Design Rules

- **Goal:** Construct contemporary, high-yield, multi-database systematic review search sets across Special Education, Education, Psychology, Medicine, Nursing/Allied Health (plus optional Phase-2 disciplines) so we can publicly report real-world deduplication performance.
- **Strict AI Non-Overlap Rule:** Because "Artificial Intelligence / ChatGPT" overlaps across every discipline right now, **only General Education (`EDU-1`) uses an AI topic**. Every other field uses its strongest contemporary non-AI research topic.
- **Evidence Base:** Topics were selected by querying OpenAlex for the most-cited 2023–2026 articles and systematic reviews across 30 top-ranked journals (raw output preserved in [`reference-data/openalex_top_cited_2023_2026.txt`](openalex_top_cited_2023_2026.txt)) and verifying exact 2015–2026 search yields against the ERIC API (preserved in [`reference-data/eric_yield_probe_2015_2026.txt`](eric_yield_probe_2015_2026.txt)).

---

## 2. Primary & Reserve Benchmark Topics

### 2.1 Primary Topics (5 Core Fields)

| Code | Field | Primary Topic | Top-Journal Trend Evidence (2023–2026 Citations) | Verified ERIC Yield (2015–2026) | Deduplication Hard Cases Exercised |
|---|---|---|---|---|---|
| **`SPED-1`** | **Special Education** | **Reading & literacy interventions for students with or at risk for learning disabilities / dyslexia** | *J Learning Disabilities* "Reading Interventions for Students in Grades 3–12 With Significant Word Reading Difficulties" (2023); *Learning Disability Quarterly* morphological awareness reviews & dyslexia trials (2023–2024); *Review of Educational Research* Bayesian network meta-analysis on struggling readers (48 citations); *JLD* spelling interventions meta-analysis (2025) | **2,960** | ERIC `EJ` articles vs. `ED` reports (`15p.`), ProQuest dissertations later published in *JLD*/*EC*/*RASE*, PsycINFO copyright trailers, EBSCO triple-indexing across ASC / Education Source / Education Full Text |
| **`EDU-1`** | **General Education (Sole AI Field)** | **Generative AI / ChatGPT / LLMs & chatbots in teaching and learning** | *Computers & Education* "Does ChatGPT enhance student learning? A systematic review and meta-analysis of experimental studies" (533 citations); "Impact of AI assistance on student agency" (548); "Influence of ChatGPT on student engagement" SR (393); *Review of Educational Research* chatbot language learning meta-analysis (64); *Educational Psychology Review* "Looking Beyond the Hype" (153) | **4,287** | Massive volume of 2023–2026 Online First -> print issue transitions, `arXiv`/`EdArXiv` preprints -> journal articles, IEEE/ACM conference papers vs. extended journal versions |
| **`PSY-1`** | **Psychology** | **Social media / smartphone use and adolescent mental health, depression, anxiety & loneliness** | *Psychological Bulletin* "Feeds, feelings, and focus" short-form video meta-analysis (2025, 79 citations); *J Affective Disorders* "Social media use, mental health and sleep: A systematic review with meta-analyses" (186); *JAD* academic pressure & adolescent mental health SR (373); *Annual Review of Psychology* "Social Media and Morality" (119) | 311 *(PsycINFO & MEDLINE are primary)* | Heavy cross-indexing across APA PsycINFO, PsycARTICLES, MEDLINE/PubMed, and Academic Search Complete; APA copyright trailers; author initial vs. full name variations |
| **`MED-1`** | **Medicine** | **GLP-1 / incretin receptor agonists (semaglutide, tirzepatide, liraglutide, retatrutide) for obesity & weight management** | *NEJM* Semaglutide SELECT CV trial in obesity (3,233 citations); Retatrutide Phase 2 trial (1,174); Semaglutide in HFpEF & obesity (1,667); Tirzepatide in HFpEF & obesity (1,012); *JAMA* Tirzepatide weight maintenance (859) | n/a *(MEDLINE, Embase, CENTRAL primary)* | Embase conference abstracts (`S12` supplements) vs. full *NEJM*/*Lancet*/*JAMA* articles, Cochrane CENTRAL `NCT########` trial registry links, ISO journal abbreviations (`N Engl J Med` vs. full title), published errata |
| **`NUR-1`** | **Nursing & Allied Health** | **Nurse burnout, compassion fatigue, staffing & intention to leave / quiet quitting** | *J Advanced Nursing* "Nurses' motivations to leave the nursing profession" (140 citations); *Int J Nursing Studies* "What makes nurses flourish at work?" (108); "Costs and cost-effectiveness of improved nurse staffing levels" SR (82); *JAN* job burnout & quiet quitting in nurses (63) | 71 *(CINAHL & MEDLINE primary)* | CINAHL (EBSCO) <-> MEDLINE (PubMed) <-> APA PsycINFO <-> Academic Search Complete overlap; British vs. American spelling (`burn-out`, `organisation`) |

---

### 2.2 Reserve Topics (If a Primary Topic Is Too Small or Too Large on Target Platforms)

| Code | Field | Reserve Topic | Top-Journal Evidence | Verified ERIC Yield (2015–2026) |
|---|---|---|---|---|
| **`SPED-2`** | Special Education | Mathematics interventions for students with learning disabilities / math difficulty | *J Special Education* model-based problem solving (26 citations); *RASE* math vocabulary interventions SR (2024); *LDQ* rational number interventions SR/MA (2024) | **1,634** |
| **`EDU-2`** | General Education (Non-AI) | Digital game-based learning & gamification in education | *Review of Educational Research* "Effect of Digital Game-Based Learning Interventions... A Meta-Analysis" (134 citations); *Educational Research Review* teacher scaffolding in game-based learning SR (83) | **6,694** |
| **`PSY-2`** | Psychology | Loneliness & social isolation interventions | *J Affective Disorders* emotional and social loneliness (160 citations); *Campbell Systematic Reviews* digital interventions to reduce social isolation and loneliness EGM (103) | **950** |
| **`MED-2`** | Medicine | Long COVID / Post-Acute Sequelae of SARS-CoV-2 (`PASC`) | *JAMA* Definition of Postacute Sequelae of SARS-CoV-2 Infection (851 citations) | 27 |

---

### 2.3 Phase-2 Expansion Disciplines (Optional, Based on Institutional EBSCO/Library Access)

| Code | Discipline | Topic | Top-Journal Evidence (2023–2026) | Target Databases | Specific Deduplication Stress Test |
|---|---|---|---|---|---|
| **`SPORT-1`** | Sport & Exercise Science | Sport-related concussion recovery & prevention | *British J Sports Medicine* Amsterdam Consensus Statement (1,239 citations) + 3 *BJSM* concussion SR/MAs (162–181 citations) | `SPORTDiscus` (EBSCO), `MEDLINE`, `Embase` | Consensus statements co-published across multiple journals simultaneously |
| **`CRIM-1`** | Criminology & Policing | Procedural justice and police legitimacy | *J Experimental Criminology* "A systematic review and meta-analysis of procedural justice and legitimacy in policing" (54 citations) | `Criminal Justice Abstracts` (EBSCO), `SocINDEX`, `PsycINFO` | Government agency reports (NCJRS) later published in criminology journals |
| **`SW-1`** | Social Welfare & Economics | Guaranteed basic income & cash transfer interventions | *Campbell Systematic Reviews* "Effects of guaranteed basic income interventions on poverty-related outcomes" SR/MA (2024) | `SocINDEX` (EBSCO), `EconLit` (EBSCO), `Business Source Complete` | NBER / SSRN / World Bank working papers circulating 2–5 years before journal publication |
| **`ENV-1`** | Environmental Health | Microplastics / nanoplastics and human health toxicity | *NEJM* "Microplastics and Nanoplastics in Atheromas" (1,221 citations); *Science of the Total Environment* reviews (355–506 citations) | `GreenFILE` (EBSCO), `MEDLINE`, `Web of Science`, `Scopus` | Embedded HTML tags (`<sub>`, `<i>`) and chemical formulas in titles |

---

## 3. Portable Boolean Search Strings (Title + Abstract Scope)

All strings avoid truncation inside quoted phrases so they behave consistently across EBSCOhost, Ovid, ProQuest, ERIC, PubMed, and Embase.

### `SPED-1`: Reading Interventions for Learning Disabilities / Dyslexia
```text
("learning disability" OR "learning disabilities" OR "reading disability" OR "reading disabilities" OR dyslexi* OR "reading difficulty" OR "reading difficulties" OR "struggling readers" OR "at-risk readers")
AND (intervention* OR instruction* OR tutoring)
AND (reading OR literacy OR decoding OR phonics OR "phonological awareness" OR fluency OR comprehension)
```

### `EDU-1`: Generative AI in Teaching & Learning
```text
("generative artificial intelligence" OR "generative AI" OR GenAI OR ChatGPT OR "large language model" OR "large language models" OR chatbot*)
AND (education OR teaching OR learning OR student* OR teacher* OR classroom* OR university OR school*)
```

### `PSY-1`: Social Media & Adolescent Mental Health
```text
("social media" OR "social networking" OR Instagram OR TikTok OR Snapchat OR "short-form video" OR "smartphone addiction" OR "problematic smartphone use")
AND (adolescen* OR teen* OR youth OR "young people")
AND (depress* OR anxiety OR "mental health" OR "psychological distress" OR "well-being" OR wellbeing OR loneliness OR "self-harm" OR suicid*)
```

### `MED-1`: GLP-1 Receptor Agonists for Obesity
```text
("glucagon-like peptide-1" OR "GLP-1" OR semaglutide OR tirzepatide OR liraglutide OR retatrutide OR cagrilintide)
AND (obesity OR obese OR overweight OR "weight loss" OR "weight management" OR "body weight")
```
- *Optional Clinical Trial Filter (if unfiltered yield exceeds 10,000 export limit in PubMed/Embase):*
  ```text
  AND (randomized OR randomised OR placebo OR trial)
  ```

### `NUR-1`: Nurse Burnout & Intention to Leave
```text
(nurse OR nurses OR nursing)
AND (burnout OR "burn-out" OR "compassion fatigue" OR "intention to leave" OR "intent to leave" OR "turnover intention" OR "turnover intentions" OR "quiet quitting")
```

---

## 4. Platform-Specific Syntax Translations

1. **EBSCOhost (`Academic Search Complete`, `Education Source`, `Education Full Text`, `ERIC`, `PsycINFO`, `PsycARTICLES`, `CINAHL`, `SPORTDiscus`, `Criminal Justice Abstracts`, `SocINDEX`, `EconLit`, `GreenFILE`):**
   - Wrap each concept block $B_i$ in `(TI (B_i) OR AB (B_i))` so the search is strictly Title + Abstract across all EBSCO databases.
   - **Important EBSCO Setting:** Before running or exporting, **uncheck / disable** EBSCO's automatic "Linked Full Text" or "Apply equivalent subjects" expansion, and **disable** EBSCO's built-in multi-database duplicate removal (export each database separately so we have exact per-database PRISMA counts!).
2. **ERIC Native (`eric.ed.gov`):**
   - Paste the generic string with `publicationdateyear:[2015 TO 2026]`.
3. **PubMed (`pubmed.ncbi.nlm.nih.gov`):**
   - Append `[tiab]` to every term/phrase and add `AND 2015:2026[dp]`. Export in both **PubMed (`.nbib`)** and **RIS** formats (up to 10,000 records per file).
4. **Embase (`embase.com`):**
   - Append `:ti,ab` to every term/phrase (using single quotes for phrases) and `[2015-2026]/py`. Keep all publication types enabled (especially *Conference Abstract* and *Article in Press*).

---

## 5. Reproducible Export & File Naming Protocol

1. **Date Window:** Export all databases for a given topic within a **7-day window**.
2. **Publication Year Filter:** `2015–2026` (no language, document-type, or peer-review restrictions — we want raw, realistic noise).
3. **Export Content:** Always choose **"All fields" / "Full record + Abstract"** (never Citation-only).
4. **Directory Structure in Workspace:**
   ```text
   testdata/
   ├── samples/                              # Small 50-200 record samples for M1 parser unit tests
   └── benchmarks/
       ├── SPED-1_reading_ld/
       │   ├── academic_search_complete/
       │   ├── education_source/
       │   ├── education_full_text/
       │   ├── eric/
       │   ├── psycinfo/
       │   └── psycarticles/
       ├── EDU-1_genai/
       ├── PSY-1_social_media_youth/
       ├── MED-1_glp1_obesity/
       ├── NUR-1_nurse_burnout/
       └── search_log.csv                    # Date, platform, database, exact query, raw hit count
   ```
5. **Public Distribution & Licensing Rule:**
   - Raw abstracts from proprietary databases (PsycINFO, Embase, EBSCO, Scopus, WoS) are **not** committed to the public GitHub repository.
   - Instead, `.gitignore` excludes raw proprietary exports in `testdata/benchmarks/`, while we publicly release:
     - Synthetic perturbation datasets (`testdata/fixtures/synthetic/`),
     - Public open-access benchmarks (ERIC / PubMed / OpenAlex IDs),
     - Anonymized record-ID pair lists (`DOI`, `PMID`, `AN`/`UID` hashes), our system's classification output, and the full verification metrics report.

Archive note: relative document links were rebased for this archival location.
