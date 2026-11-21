# Engine contract

A **record** is one occurrence in a named export. A **report** is a publication
that may appear in several records. A **study** can have several distinct reports,
such as a protocol, a conference paper and a results article.

## Source identity

An occurrence key combines a source label and a one-based ordinal. The decision
sheet records the source fingerprints. Reopening a sheet checks the original
bytes, including when the file has moved. Two independently labeled exports
can have identical bytes and still contribute separate occurrences.

## Review and export

A duplicate decision identifies an eligible copy and an explicitly kept target.
Pending occurrences and occurrences without decision rows are retained. The
ordinary reviewed RIS carries each survivor's own fields in source order.
It uses LF line endings and omits the leading byte-order mark. The mapping
records the original source spans and the corresponding output ordinals.

For each source, the number of input occurrences equals explicit identifier
removals plus explicit metadata removals plus retained occurrences. Three copies
assigned to one keeper count as three removals. Retained occurrences can still
include unresolved copies as well as distinct reports.

Supply the reviewed RIS, mapping and reconciliation to `dedup verify-export` to
check their agreement. The check uses the supplied mapping as its reference.
Trust in the mapping's origin must come from the review's own audit trail.

- [Trace a removal to its source files](../README.md#trace-an-exported-occurrence-to-its-originals)
- [Reconcile an export](../README.md#reconcile-a-reviewed-export-before-screening)
