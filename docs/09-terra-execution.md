# GPT-6.1 Terra execution runbook

Use model **`gpt-6.1-terra`**, preserving the requested model. Official OpenAI
documentation gives `codex -m gpt-6.1-terra` and requires Codex CLI 0.159.0 or
later: [model documentation](https://learn.chatgpt.com/docs/models), checked
2026-11-01. Account availability must be checked in the actual client; a model
name in this document is not proof of successful model execution.

## Start in Codex desktop

Open this project, select GPT-6.1 Terra and **max** reasoning, and paste the
bounded-batch prompt below when the user starts development. Read the
[current handoff](14-terra-handoff.md); collection completion is not a B01 gate.
The first implementation batch is B01. The local model catalog checked on
2026-11-02 lists max as a supported reasoning setting.

## Start with the CLI

Check `codex --version`. If below the documented minimum, run `codex update`
and check again. The preparation review records the local outcome. From PowerShell:

```powershell
codex -C 'D:\Codex\Deduplicator' -m gpt-6.1-terra -c 'model_reasoning_effort="max"' 'Read AGENTS.md, AGENT.md, TASKS.md, docs/08-development-plan.md, docs/10-engine-contract.md, and docs/14-terra-handoff.md. Implement only B01 with its acceptance cases and checks. Do not search library databases, export or download records, resume collection queues, run model validation, or write the manuscript. Preserve unrelated work. Update TASKS.md with actual evidence and the next batch.'
```

For another checkout, replace the path with that checkout. For a noninteractive
run, use the same arguments with `codex exec`; inspect the resulting diff and
validation evidence before advancing the next batch. Do not bypass safeguards
or change global configuration to make a model switch.

## Reusable bounded-batch prompt

```text
Work in the existing record-deduplicator checkout using GPT-6.1 Terra at max reasoning.
Read AGENTS.md, AGENT.md, DECISIONS.md, TASKS.md, docs/08-development-plan.md,
docs/10-engine-contract.md, docs/14-terra-handoff.md, and only the specs needed
for the selected batch.

Select the first incomplete unblocked B-batch using dependencies, not numeric order:
B01-B06, B08a, B07, B09-B11, B08b, B12, then B13.
Freeze family-disjoint development/test splits in B08a before corpus-based tuning.
Final corpus and LLM evaluation in B08b follows the completed production candidate.
State its ID, files to inspect/change, runtime impact, and validation method.
Implement that batch end to end. If too large, choose its smallest useful
sub-batch with a concrete input-to-output acceptance case and record the split.

Use one lead and nonoverlapping subagents for broad parallel work when available.
Lead owns shared models, manifests, lockfiles, integration and final verification.
Do not revert other contributors' changes. Do not silently change model.

Follow the engine contract: full DOI identity, namespaced accession IDs,
trial IDs only for relationships, protected distinct reports, cluster-wide
cannot-link checks, immutable inputs, field provenance, reversible decisions,
bounded work and explicit incomplete status. Keep runtime offline and LLM-free.
Collection is stopped. Do not search manuscripts/library databases, request
exports, download files, retrieve mailbox attachments or resume old Aside queues.
Do not run model validation or write the manuscript in a development batch.
Future user-selected collection uses Aside CLI. Keep private exports ignored.

Add meaningful tests for actual failure modes. Run the batch checks once after
integration; rerun only when changed code or a resolved failure justifies it.
Capture exact commands, results and artifact paths. Do not substitute build
success for native workflow, accuracy, data rights or release evidence.

Update TASKS.md and relevant docs with implemented versus planned behavior.
Keep all commits attributed solely to Sungwoo Kang
<200436385+SKang393@users.noreply.github.com>, without agent trailers.
Do not publish a release, overwrite source data, or expand into another batch.

Start responses with Thread and Summary. Finish with files changed,
implementation, commands, verification results and exact remaining blockers.
```

## Handoff at every batch boundary

Record selected batch/sub-batch, source commit, changed files, schema/policy
versions, actual validation commands/results, output artifacts, unresolved
failures and next startable batch in TASKS. Mark a check complete only when its
acceptance evidence exists. A missing real database sample, human labeler, macOS
host or signing credential blocks the corresponding claim; it must not be
replaced with invented fixtures/results or a request to replan everything.
