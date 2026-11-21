# Desktop application boundary

The Tauri 2 + React application is planned in batches B09-B10 of the
[development plan](../docs/08-development-plan.md). No GUI is implemented yet.
To review records now, use [guided setup in the packaged CLI](../README.md#start-a-review-with-your-own-exports).

Use the same `dedup-core` policy and result contracts as the CLI. Do not add a
second matching engine in TypeScript. The external `validation/` harness must
never be bundled with the application.
