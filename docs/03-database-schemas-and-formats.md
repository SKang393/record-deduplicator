# RIS fields and source formats

The saved review accepts named UTF-8 RIS exports, including files with a
UTF-8 byte-order mark and LF, CRLF or CR line endings. Each record starts
with `TY` and ends with `ER`. Give each download its own source label so
that an occurrence can be located by label and record number.

| Source fields | What to check during inspection |
|---|---|
| `TI`, `T1`; `AU`, `A1` | Alternate title tags, multiple authors, spelling and author order |
| `PY`, `Y1`, `DA` | The supplied date and its tag. Online-first and print records can give different dates. |
| `JO`, `JF`, `T2`; `VL`, `IS`, `SP`, `EP` | Venue, volume, issue, pages or article number. An abbreviated venue or provisional pagination can obscure a match. |
| `TY`, `M3`; `AB`, `N2`; `KW` | Publication type, abstract and keywords available on the selected occurrence |
| `ID`, `AN` | A reference or accession number as supplied by the source. The review uses the named source and ordinal for its occurrence key. |

The same tag can have different meanings across exporters. For example, Scopus
uses `C2` for both PMID and PMCID. The inspector shows the original text. A
bare number in that field supplies no confirmed identifier for matching.
[Scopus RIS mapping](https://www.elsevier.support/scopus/answer/what-ris-format-mapping-does-scopus-use-when-exporting-results-when-exchanging-citation-information)

Keep the original when converting from another format. Verify titles, creators
and identifiers in the converted file before starting the review. A standalone
two-column DOI/PMID TSV can produce an identifier report.

- [Convert a BibTeX export](../README.md#bring-a-bibtex-export-into-a-named-ris-review)
- [Review the DOI/PMID projection](../README.md#review-a-two-column-tsv-projection)
