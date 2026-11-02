# Agent Handoff & Operating Instructions (`record-deduplicator`)

> **Note:** This file mirrors [`AGENT.md`](AGENT.md) so that tools scanning for either `AGENTS.md` or `AGENT.md` load the same instructions.

See [`AGENT.md`](AGENT.md) for the full operating rules, documentation map, and handoff instructions, and [`TASKS.md`](TASKS.md) for the ordered development backlog.

Start every response with `Thread: [Short Title]` and `Summary: [one sentence]`.
Use the [Terra runbook](docs/09-terra-execution.md) and
[development plan](docs/08-development-plan.md) for bounded execution.
The [engine contract](docs/10-engine-contract.md) resolves older specification
conflicts. Current implementation is a foundation only; next batch is B01.

## Quick Rules Summary
1. **Git Attribution:** Commit and push strictly as `SKang393` (`Sungwoo Kang <200436385+SKang393@users.noreply.github.com>`). Never include `Co-authored-by` or AI/agent trailers.
2. **LLM Validation Separation:** The LLM validation oracle lives strictly in `validation/` for benchmarking our system and is **never** part of the shipped deduplicator program.
3. **Preparation Only:** The latest user instruction stops collection. Prepare the system for Terra; do not execute searches, exports, downloads, mailbox retrieval, watchers or validation. Preserve existing partial evidence. Follow [the current handoff](docs/14-terra-handoff.md). Future collection is a separate user-selected run, never an automatic development prerequisite. Close completed task-owned tabs and leave unrelated user tabs alone.
4. **Manuscript & University Library Access:** Always use the **`aside` CLI** when accessing manuscripts or university library databases.
5. **Cross-Platform:** Rust + Tauri 2 + React (`shadcn/ui`), supporting Windows and macOS from day one.
