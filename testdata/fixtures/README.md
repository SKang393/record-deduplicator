# Public fixtures

Synthetic fixtures demonstrate specific behavior with generated records.
See the [development plan](../../docs/08-development-plan.md).

## Zotero RIS handoff

Import [reviewed-cafe-marsh.ris](reviewed-cafe-marsh.ris) into an empty
collection. The expected result is two items. The file is the reviewed output
from a synthetic search about one fictional marsh study.

| Zotero item type | Title | Authors in stored order | Pages | Archive Location (RIS `AN`) |
| --- | --- | --- | --- | --- |
| Journal Article (`JOUR`) | Café Marsh canopy results | Ada Linné, Sol Marin | 21-26 | RD-CAT-01 |
| Conference Paper (`CPAPER`) | Café Marsh canopy pilot | Sol Marin, Ada Linné | 7-9 | RD-CONF-02 |

**RecordDedU export.** The catalog supplied the article in UTF-8 with a BOM
and CRLF line endings. The index used CR line endings and supplied the
conference paper plus an article copy without a DOI or PMID. Pair inspection showed the conference
paper had a different title, type and pages despite carrying the article's
DOI. The reviewer kept the catalog article, verified the index copy against
it and left the conference paper pending. The mapping assigned `cat:1` and
`idx:2` to output 1 and `idx:1` to output 2. Both input hashes stayed
unchanged. The exported bytes matched the two retained source records after
removing the file BOM and converting line endings to LF.

**Zotero import.** Zotero Desktop 10.0.8 on Linux opened the local RIS file
in a fresh profile with sync disabled. Its library went from zero to two
items. The stored titles, creator names and order, types and accessions are
shown above. Both items retained the DOI `10.0000/cafe-marsh`. The article's
RIS `UR` contained a PubMed link ending in `/999999991/`. Zotero stored that
link in URL and left the separate PMID absent. Check where the reference
manager stores the identifier if a later step expects a dedicated PMID field.

The tested file has SHA-256 `0ea88114916154b13184f685cb7023dd360618e786c769519dc7be47e9d87ea4`.

## Version 1 reviewed export

The `reviewed-export-v1` mapping and RIS were exported together by commit
`a4b736491c3d6822d4f59b20a22ed346604a1c69`. The inputs were the synthetic
walkthrough's catalog record and its index copy with the DOI field omitted.
Both occurrences were retained. The package smoke checks these frozen files
against the current CLI.
