# External evaluation boundary

This directory is for the planned Python benchmark and optional LLM assistance
in B08 and B12 of the [development plan](../docs/08-development-plan.md).
No benchmark runner or model integration is implemented yet.

Human-adjudicated reference labels establish test truth. LLM suggestions are
advisory, can abstain, and must not become gold labels without human review.
Do not send proprietary exports to an external service without explicit data
authorization. Private payloads, caches, and results use the ignored `private/`,
`cache/`, and `results/` directories. Reviewed, redistributable summaries may be
published under `docs/evidence/` after they exist.

This is not a Cargo workspace member, runtime dependency, or desktop resource.
See [validation methodology](../docs/05-validation-and-llm-oracle.md).
