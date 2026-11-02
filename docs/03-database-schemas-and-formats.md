# 03 — Target Database Indexing, Metadata Schemas & Export Formats

> Design inventory, not verified parser support. Vendor mappings and detection
> claims require representative permitted fixtures. Follow the
> [engine contract](10-engine-contract.md) for namespaced IDs, unknown database
> labels, decoding warnings and preservation. No parsers are implemented yet.

---

## 1. Core Education, Special Education & Psychology Databases

The six primary education and psychology databases operate across two major vendor ecosystems:
- **EBSCOhost:** Hosts *Academic Search Complete*, *Education Source*, *Education Full Text (H.W. Wilson)*, and frequently *ERIC* and *APA PsycINFO* / *PsycARTICLES*.
- **Ovid / APA PsycNET / ProQuest / IES Native:** Alternative hosts for *APA PsycINFO*, *PsycARTICLES*, *PsycEXTRA*, *ERIC* (`eric.ed.gov`), and *ProQuest Dissertations & Theses Global*.

### 1.1 Standardized Field Mapping Table (Education & Psychology)

| Standardized Field | Standard RIS Tag | EBSCOhost Internal Tag | APA (`PsycINFO` / `PsycARTICLES`) Specifics | `ERIC` Specifics |
|---|---|---|---|---|
| **Database Unique ID** | `AN` / `ID` | `AN` (Accession Number) | `UID`: `YYYY-NNNNN-NNN` (e.g., `2023-12345-001`) | ERIC Number: `EJ######` (Journal Article) or `ED######` (Document / Grey Lit Report) |
| **Digital Object ID (DOI)** | `DO` / `DI` | `DI` / `DO` | `DO` (Standardized DOI; highly reliable post-2000) | `DO` (Frequently missing on `ED` reports and older `EJ` records) |
| **Article Title** | `TI` / `T1` | `TI` | Sentence case in APA; includes `[Translated Title]` or whole-title brackets `[...]` + original title in `OT` | Title case or ALL-CAPS in legacy ERIC records |
| **Authors** | `AU` / `A1` | `AU` | `Last, First M.` (Often includes `AI` Author ID / ORCID metadata) | `Last, First` (Occasionally missing middle initials) |
| **Abstract** | `AB` / `N2` | `AB` | Structured or unstructured; **automatically appends `(PsycInfo Database Record (c) YYYY APA, all rights reserved)` at the end of `AB`** | Standard abstract; `ED` documents may have extended multi-paragraph summaries or truncated legacy abstracts |
| **Journal / Source** | `JO` / `T2` / `JF` | `SO` / `JN` | Full journal title (`JT`); rarely abbreviated | Full journal title (CIJE indexing) |
| **Pub Year / Date** | `PY` / `Y1` | `DT` / `PY` | 4-digit year (`YR`) + Publication Date (`PD`) | 4-digit year (`PY`) |
| **Volume & Issue** | `VL` / `IS` | `VI` / `IP` | `VL` and `IS` (Often blank for Advance Online Publication) | `VL` and `IS` (Frequently absent for `ED` reports) |
| **Pagination** | `SP` / `EP` | `PG` | Start Page (`SP`) and End Page (`EP`) or Article ID (`AR`) | Page count (`15p.`) on `ED` documents rather than start–end page ranges |
| **ISSN / ISBN** | `SN` | `IS` / `IB` | Print ISSN and Electronic ISSN (`eISSN`) | ISSN for `EJ`; ISBN for `ED` books/reports |
| **Document Type** | `TY` / `M3` | `PT` / `DT` | `Journal Article`, `Dissertation Abstract`, `Chapter` | `Journal Articles (080)`, `Reports - Research (143)`, `Speeches/Meeting Papers (150)` |
| **Peer-Reviewed Flag** | `N1` (Notes) | `RV` (Reviewed) | Strictly peer-reviewed journals in `PsycARTICLES`; mixed in `PsycINFO` | Explicit `Peer-Reviewed: Yes/No` metadata flag |

### 1.2 Critical Engineering Quirks for APA & Education Databases
1. **APA Copyright Trailer Penalty:**
   - APA *PsycINFO* and *PsycARTICLES* automatically append `(PsycInfo Database Record (c) 2026 APA, all rights reserved)` to the end of the `AB` field.
   - Our Tier 1 normalizer **must** strip this trailer before computing abstract similarity; otherwise identical abstracts between ERIC and PsycINFO suffer an artificial similarity penalty.
2. **Education Full Text (H.W. Wilson) & Education Source:**
   - Both frequently index book reviews, commentaries, and conference abstracts alongside empirical articles under identical `TY  - JOUR` RIS tags.
   - Abstract-length checks, short-title boilerplate guards, and whitelisted title-artifact checks (`[Book Review]`, `[Abstract]`) are essential to prevent merging a book review or commentary with an article of the same name.
3. **ProQuest Dissertations & Theses Global:**
   - Primary source of doctoral dissertations and master's theses in education and psychology. When a dissertation is published 1–5 years later as a journal article under an identical or slightly revised title, the pipeline must classify the pair as **`KIN_REPORT`** (linked study report), not a bibliographic duplicate.

---

## 2. Expansion to Medical, Nursing & Allied Health Databases

Biomedical and allied health researchers represent the largest user base for systematic review software. Supporting medical synthesis requires handling four core databases:

| Database | Host / Format | Unique Identifiers & Metadata Tags | Deduplication Engineering Requirements |
|---|---|---|---|
| **MEDLINE** | PubMed (`.nbib` / MEDLINE text) or Ovid | `PMID` (8-digit integer), `PMC` (`PMC#######`), `PII` (Publisher Item ID), `AID` (`[doi]` / `[pii]`), `FAU` (Full Author Name) alongside `AU` (Abbreviated Name, e.g., `Kang SW`), `TA` (ISO Journal Title Abbreviation, e.g., `J Appl Behav Anal`), `JT` (Full Journal Title), `TT` (Transliterated/Vernacular Title), `PT` (`Published Erratum`, `Retracted Publication`, `Clinical Trial`, `Comment`). | Must map `TA` (ISO abbreviation) <-> `JT` (Full title) and `ISSN` <-> `eISSN` via `ISSN-L` so PubMed records match EBSCO/PsycINFO records that only export full journal names. Must unwrap `[Translated Title]` brackets. |
| **Embase** | Elsevier (Embase.com or Ovid) | Embase Accession Number, `PUI` (Publisher Unique Identifier), Emtree drug/disease headings, specific Publication Type tags (`Conference Abstract`, `Conference Paper`, `Article in Press`). | **Single largest source of "conference abstract vs. full paper" collisions in medicine.** Must inspect `PT`/`M3` (`Conference Abstract`), supplement volume/issue (`Suppl`, `S12`), and short page counts to classify abstract-vs-article pairs as `KIN_REPORT`. |
| **Cochrane CENTRAL** | Wiley / CRS | Accession Number (`CN-########`), Trial Registry IDs (`NCT########` from ClinicalTrials.gov, `ISRCTN########`, `EUCTR`, `WHO ICTRP`), source origin tags (`MEDLINE`, `Embase`, `ClinicalTrials.gov`). | Extract Trial Registry IDs (`NCT...`) from ID tags, URLs, and abstract text to link trial registry entries and multiple trial reports as `KIN_REPORT` or `EXACT_DUPLICATE` (when CENTRAL re-indexes a PubMed/Embase record). |
| **CINAHL** | EBSCOhost | Expected to share common EBSCOhost tags (`AN`, `TI`, `AU`, `SO`, `DO`, `SN`). | Shared parser support is planned; verify dialect behavior using permitted CINAHL fixtures before claiming compatibility. |

---

## 3. Cross-Disciplinary Expansion Markets & Schemas

Beyond Education, Psychology, and Medicine, five major academic domains enforce strict systematic review / meta-analysis standards:

| Discipline | Governing Review Body | Primary Databases Used | Specific Deduplication Pain Points & Rules |
|---|---|---|---|
| **Multidisciplinary Citation Indexes** | All bodies (PRISMA / Cochrane / Campbell) | **Scopus**, **Web of Science Core Collection** (`SCI-EXPANDED`, `SSCI`, `AHCI`, `ESCI`) | WoS exports `UT` (`WOS:000...`) accession numbers and plain-text tagged format (`PT J`, `AU`, `AF`, `TI`, `SO`, `DE`, `AB`, `DI`, `SN`, `EI`, `PY`, `VL`, `IS`, `BP`, `EP`, `AR`, `ER`). Scopus exports RIS and rich CSV. |
| **Social Welfare, Criminology & Public Policy** | **Campbell Collaboration** (MECCIR standards) | *Criminal Justice Abstracts* (EBSCO), *SocINDEX* (EBSCO), *Sociological Abstracts* (ProQuest), *PAIS Index*, *HeinOnline*, *NCJRS* | High volume of government working papers and agency evaluations (e.g., NCJRS technical reports) published 1–4 years later as peer-reviewed journal articles -> classify as `KIN_REPORT`. |
| **Computer Science & Software Engineering** | **Kitchenham / IEEE / ACM Guidelines** | *IEEE Xplore*, *ACM Digital Library*, *Scopus*, *Web of Science*, *arXiv*, *DBLP* | Heavy reliance on `arXiv` preprints (`arXiv:YYMM.NNNNNv#`) and full peer-reviewed Conference Proceedings (`In Proceedings of...`) later extended into journal versions with different DOIs -> link preprint/conference-to-journal as `KIN_REPORT`, while merging identical IEEE/ACM/Scopus indexings of the same conference paper as `BIBLIOGRAPHIC_DUPLICATE`. |
| **Environmental Science, Ecology & Agriculture** | **Collaboration for Environmental Evidence (CEE)** | *AGRICOLA*, *CAB Abstracts*, *BIOSIS Previews*, *Web of Science*, *GreenFILE* (EBSCO) | Multi-institutional technical reports and taxonomic/chemical formatting variations (`<i>Escherichia coli</i>`, `CO<sub>2</sub>`, `PM<sub>2.5</sub>`) embedded inside article titles -> stripped cleanly in Tier 1 HTML/tag normalization. |
| **Economics, Management & Finance** | **Academy of Management / NBER / Campbell Business** | *EconLit* (EBSCO), *Business Source Complete* (EBSCO), *RePEc / IDEAS*, *SSRN*, *ABI/INFORM* (ProQuest) | Working paper series (*NBER Working Papers*, *SSRN*, *World Bank Policy Research*, *IZA*) circulate for 2–5 years before formal journal publication under identical or slightly revised titles -> require $\pm 5$-year `KIN_REPORT` window on `FirstAuthor + Title`. |
| **Sport & Exercise Science** | **BJSM / Cochrane Rehabilitation** | *SPORTDiscus* (EBSCO), *MEDLINE*, *Embase* | International consensus statements co-published simultaneously across 3–6 journals under identical titles and authors with different DOIs/journals -> flag for reviewer (`KIN_REPORT` / co-publication) rather than silent loss of citation context. |

---

## 4. Supported Export File Formats & Character Encodings

`crates/dedup-core` must auto-detect and ingest:
1. **RIS (`.ris`, `.txt`):** All vendor dialects (EBSCOhost, Ovid, APA PsycNET, ERIC, Embase, Scopus, Web of Science, ProQuest, Zotero, EndNote, Mendeley, Covidence).
2. **PubMed / MEDLINE Tagged (`.nbib`, `.txt`):** NLM 4-letter tagged format (`PMID- `, `TI  - `, `FAU - `, `AU  - `, `LID - `, `AID - `, `TA  - `, `JT  - `).
3. **Web of Science Plain Text (`.ciw`, `.txt`):** `FN Clarivate Analytics Web of Science` / `VR 1.0` / `PT J` ... `ER` / `EF`.
4. **CSV / TSV (`.csv`, `.tsv`):** Header-mapped tabular exports from Scopus, Web of Science, Covidence, Rayyan, and custom spreadsheets.
5. **EndNote XML (`.xml`):** `<xml><records><record>...</record></records></xml>`.
6. **BibTeX (`.bib`):** Standard BibTeX / BibLaTeX entries (`@article`, `@inproceedings`, `@techreport`, `@phdthesis`).

### Character Encoding Rules
- Auto-detect BOM and byte patterns via `encoding_rs`:
  - `UTF-8` (with or without BOM `EF BB BF`)
  - `UTF-16LE` and `UTF-16BE` (frequently produced by Windows/macOS EndNote exports)
  - `Windows-1252` / `ISO-8859-1` fallback for legacy database exports
- Normalize all line endings (`\r\n`, `\n`, `\r`) before tokenization.
