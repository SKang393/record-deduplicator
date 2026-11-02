# Data boundaries

- `fixtures/`: committed synthetic or explicitly redistributable fixtures, each
  with origin, license, expected interpretation, and producing rule/version.
- `samples/`: private real database exports, ignored by Git.
- `benchmarks/`: private real evaluation collections, ignored by Git.

The 2026-11-02 collection is **stopped at the user's direction**. Preserve existing
source files, query history, timestamps, counts and export hashes in the private
`benchmarks/2026-11-02/` directory. Do not fetch missing exports or copy more files.
The [prepared protocol](../docs/06-benchmark-search-protocol.md) is for a future
user-selected collection run, not an automatic development step. Future
manuscript and Purdue/subscription access requires `aside`.
Hashing an accession number does not automatically make a dataset redistributable.
