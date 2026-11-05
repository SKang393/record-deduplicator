# Planned validation conditions and manuscript handoff

Status: user-requested design, 2026-11-02. **Preparation only; collection and
validation are not being run.** The settings below are requested
conditions, not independently verified provider specifications or account limits.
Do not silently substitute another model or effort setting.

| Interface supplied by user | Requested model | Requested effort | Requested context ceiling |
|---|---|---|---|
| [Meta developer interface](https://dev.meta.ai/) | Wonder 1.0 | max | 1,000,000 tokens |
| [Google AI Studio](https://aistudio.google.com/) | Gemini 4 Argon | high | 1,000,000 tokens |
| [ChatGPT Work](https://chatgpt.com/) | GPT-6 Astra | max | 500,000 tokens |
| [Claude](https://claude.ai/chats) | Opus 5.5 | max | 1,000,000 tokens |
| [xAI build interface](https://x.ai/build) | Grok 4.7 | xhigh | 500,000 tokens |

At experiment setup, verify the actual product, model identifier/version, effort
control, effective input/output limits, availability, data handling and quota in
each interface. Save dated evidence. Mark unavailable requested conditions
BLOCKED until the user selects a change. A display name alone does not establish
an immutable model version. Do not send proprietary data to these services during
collection.

The received EBSCO export email on 2026-11-02 restricts downloaded content to
personal, non-commercial use and prohibits publication or sharing. Keep the RIS
files private. Before later external model submission or dataset release,
establish an applicable permission or an independently permitted corpus; the
repository's software license does not grant rights to these exports. This is a
specific observed data-use condition, not a claim that the future experiments
have been approved or executed.

## Development before evaluation

Follow the bounded implementation batches in the
[development plan](08-development-plan.md). Record completed checks and
outstanding gates in the [development tracker](../TASKS.md).

Complete the production program and its meaningful parser, safety, replay,
determinism and native workflow checks before asserting measured performance.
The production engine remains offline and has no runtime LLM dependency.

## Pre-experiment decisions to freeze

- Freeze original per-source RIS/NBIB/CSV hashes, record occurrence IDs, source counts,
  field assignments, platform export losses, schema and policy versions.
- Define independent human labels and adjudication. Model consensus does not
  replace human reference truth. Keep model agreement separate from engine
  accuracy. Preserve uncertain labels and model abstentions.
- Split by review/report family and cross-field overlap before tuning. Do not put
  copies of one report in development and test because they came from different
  fields or databases.
- Use the same record evidence, labeling rubric, output schema and decision units
  across providers. Predefine batching, record ordering and boundary overlap so
  duplicate pairs spanning chunks are not silently untestable.
- Report a matched-budget comparison separately from a provider-specific
  maximum-context condition. The requested ceilings differ, and tokenizers are
  not interchangeable. Count actual tokens with each supported tokenizer or
  interface report; do not equate characters or record counts with tokens.
- Reserve space for instructions, reasoning where accounted for by the service,
  and output. Reject silent truncation. Preserve batch membership, prompt bytes,
  response bytes, schema failures, retries, timestamps, costs and actual settings.
- Predeclare repeats and failure/retry rules. Fixed temperature or effort does
  not guarantee reproducibility in a hosted model. Never select the best run
  retrospectively without reporting all runs.
- Evaluate candidate completeness, automatic precision/recall, cluster errors,
  incorrect report merges, review-assisted outcomes and time/memory separately.
  Report per-core-field and macro/micro summaries with uncertainty. Present SPED
  as supplemental rather than silently adding it to the primary aggregate.

The actual executable experiment protocol will be finalized after corpus
collection and program development. No prompts, response logs or metrics are
claimed to exist at this stage.

## APA manuscript after completed validation

Retain an evidence ledger linking every Methods detail and Results number to a
dated source, collection event, file hash, software revision or analysis output.
Report the journal/review selection procedure, sampling limitations, corpus-size
tuning, expanded source-panel selection, platform-specific search semantics,
native internal deduplication, export losses and unresolved data. Preserve the
earlier EBSCO-only files as pilots, outside the final expanded corpus.
Do not describe this benchmark collection as an exhaustive systematic review of
any substantive topic.

Write the manuscript in APA format only after the program and validated analysis
artifacts exist. Apply the installed **Humanizer** skill to the prose, preserving
technical meaning, uncertainty, citations, dates and all numeric results. Do not
invent completed procedures, human participants, approvals or results.
