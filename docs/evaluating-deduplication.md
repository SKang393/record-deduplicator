# Evaluating bibliographic deduplication

## What counts as a match

Label pairs by whether both records describe the same report. A later article
can report on the same participants as a conference paper and still warrant
its own record. Give independent reviewers the citation evidence, blind them
to the program's decision and adjudicate disagreements. Allow an unresolved
label when the evidence is insufficient.

To evaluate an update to a matching rule, keep copies of a report and related
versions together when splitting development and evaluation data. Record the
input versions, source counts, program revision and settings. When an export is
delivered in batches, reconcile the ranges and delivered counts within each
search snapshot before counting deduplication removals.

## Candidate coverage and review effort

Pair recall is the fraction of known same-report pairs surfaced for review. For
a report with several copies, also check whether the suggested pairs connect
all of its occurrences. Count isolated copies and connections between distinct
reports separately. A single cross-report connection can put several unrelated
pairs in the same group.

A sample drawn only from suggested pairs cannot reveal missed candidates.
When sampling, identify which pairs were eligible for selection, record the
inclusion probabilities and report uncertainty. Report the number of comparisons
and the time reviewers actually spent as separate measures.

## Reporting the handoff

Distinguish pairs suggested, copies explicitly removed, records forwarded for
screening and records actually screened. Report the number of distinct studies
only when study-level reference labels support that count.
[Methodology and literature](01-methodology-and-literature.md)
