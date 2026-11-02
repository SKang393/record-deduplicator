# Repository preparation and evidence review

Date: 2026-11-01. Scope: public repository foundation, documentation review,
product differentiation and an executable GPT-6.1 Terra development plan.

## Starting evidence and changes

The initial folder contained 14 documentation/reference files (159,823 bytes),
no `.git`, no Cargo or frontend manifests, no implementation, and no real sample
exports. The README described future capabilities as implemented. The initial
`gh repo view SKang393/record-deduplicator` returned repository not found.

The preparation adds a licensed Rust workspace, truthful help/version CLI,
Git exclusions and cross-platform CI. It establishes competitor evidence,
bounded B01-B13 development batches, a Terra runbook, and the normative engine
contract. Existing research and topic protocols are retained with corrections
and status labels. No topic searches or yield probes were rerun.

Principal corrections: full DOI identity rather than base DOI, database-namespaced
accessions, relation-only trial IDs, cluster-wide cannot-link checks, immutable
source recovery, field alternatives and replay, independent human reference
labels, explicit sampling uncertainty, and draft rather than guaranteed PRISMA
reporting. Native performance, accuracy and format claims remain unmeasured.

## Bounded manuscript verification through Aside

Access method: `aside guide`, then `aside exec --effort high` with a read-only
request for three existing citations. Aside session `PGMV7imwBo5Vh4b4` inspected
public full text and a table image. Purdue subscription access was unnecessary;
its authenticated access has not been tested. No manuscripts were committed.

| Claim | Result | Primary evidence |
|---|---|---|
| Bramer 2016 seven-pass field order | Confirmed, Table 1 rows A-G. | [Table 1](https://pmc.ncbi.nlm.nih.gov/articles/PMC4915647/table/mlab-104-03-14-t01/) |
| Bramer mandates zero penalty for every pagination mismatch | Contradicted as an attribution. This is an EndNote workflow; differing pages can require manual examination. | [Discussion](https://pmc.ncbi.nlm.nih.gov/articles/PMC4915647/#s3), [2017 correction](https://pmc.ncbi.nlm.nih.gov/articles/PMC5234466/) |
| Jiang 2014 article identifier `bau086` | Corrected to **bat086**, DOI 10.1093/database/bat086. | [Article metadata](https://pmc.ncbi.nlm.nih.gov/articles/PMC3893659/) |
| Jiang specifically validates East Asian hyphenated initial-prefix equivalence | Not verified. General reversed-name/omitted-initial handling is discussed; the specific proposed rule remains our testable hypothesis. | [Methods](https://pmc.ncbi.nlm.nih.gov/articles/PMC3893659/#sec2), [Sequence matching](https://pmc.ncbi.nlm.nih.gov/articles/PMC3893659/#sec5) |
| Qi et al. citation and year | Confirmed: PLOS ONE, 2013, 8(8), e71838, DOI 10.1371/journal.pone.0071838. | [Article metadata](https://pmc.ncbi.nlm.nih.gov/articles/PMC3748039/) |

Other specific numerical or methodological attributions in the literature register,
including the planned Fellegi-Sunter check, remain pending full-text verification
before formal scholarly claims. M0.3 is therefore partial, not complete.

## Competitor evidence

The [comparison](07-product-positioning.md) uses official documentation for eight
tools. It records confirmed capabilities and unknowns without treating missing
documentation as a missing feature. No commercial tools were installed or
benchmarked. Our four proposed differentiators require the tests in the plan.

## Tooling and validation record

- GitHub CLI account explicitly switched to `SKang393`; `gh api user` returned
  login `SKang393`, ID `200436385`. Local Git identity is Sungwoo Kang with that
  account's noreply email; credential helper uses the selected GitHub CLI account.
- Rust: rustc 1.99.0 and cargo 1.99.0; rustfmt and clippy installed. Workspace
  minimum declared Rust 1.85 is not independently tested as an MSRV yet.
- Codex: upgraded from 0.144.6 to **0.166.0**, verified by `codex --version` and
  installed package metadata. This exceeds the documented Terra minimum 0.159.0.
  The updater reported an EPERM cleanup warning for the old running executable;
  the new command works. No manual deletion of that old directory was performed.
- A read-only, ephemeral `codex exec -m gpt-6.1-terra` smoke run completed with
  exit 0 and the requested response. Local logs are under the ignored
  `.local/preparation/` directory. The CLI reported three pre-existing global
  configuration keys as ignored (`computer_use.windows.always_allowed_app_ids`,
  `features.child_agents_md`, `network_access`); unrelated global settings were
  not edited. The smoke run confirms access, not implementation quality.

| Executed check | Result |
|---|---|
| `cargo generate-lockfile` | PASS; lockfile contains only the two local crates. |
| `cargo fmt --all --check` | PASS. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS. |
| `cargo test --workspace --locked` | PASS; **zero tests**, consistent with a foundation-only build. No algorithmic validation is claimed. |
| `cargo run --locked -p dedup-cli -- --help` and `--version` | PASS; help discloses unimplemented features; version is `0.1.0-dev`. |
| `target/debug/dedup.exe run` | Expected failure, exit 2; unsupported command produces no deduplicated result. |
| `python scripts/check_docs.py` | PASS; all local Markdown file links resolve. External links/anchors are outside this check. |
| `git check-ignore` on samples, benchmarks, validation payloads/cache, `.env`, `.local` | PASS for each private-path sentinel. |
| Credential-pattern scan | No matches in reviewed project content; a bounded scan is not a universal security audit. |
| `gh repo create` and `gh repo view` | Repository created under SKang393, PUBLIC, nonempty, default branch `main`, Apache-2.0 detected. |
| `git push -u origin main` | PASS; initial commit `f529699a7bbea24d880212a3d37eee68c10646c8` published. GitHub reports both author and committer as `SKang393`. |
| [Foundation CI run](https://github.com/SKang393/record-deduplicator/actions/runs/41365406312) | PASS at `f529699`: Windows Rust job, macOS Rust job, and documentation job all completed successfully. Both OS jobs ran formatting, lint, Cargo tests and CLI help smoke checks. This verifies the foundation, not native GUI or matching behavior. |

Follow-up documentation commits record review corrections and this evidence. Check the
[current workflow runs](https://github.com/SKang393/record-deduplicator/actions)
for the latest branch status; the row above is tied to the stated initial commit.

The first staged whitespace check flagged existing blank lines at EOF in the two
historical reference snapshots. Rather than modifying those source snapshots,
`.gitattributes` preserves their original bytes and scopes the EOF exception to
those files. The subsequent staged check passed. SHA-256 comparison against the
original snapshot manifest confirms both working files and committed blobs are
byte-identical to the starting reference files.

Read-only integration review confirmed the positioning against official sources
and found a source-identity edge case: independent exports can have identical
bytes. The contract and B01 acceptance cases now preserve their separate logical
source identities and counts. A green foundation build does not establish
deduplication accuracy or a working desktop application.

## Remaining evidence gates

- B01 is the next implementation batch. No production parsers or matching exist.
- Real vendor exports and independently adjudicated reference labels are absent.
- The remaining manuscript claims must be checked with Aside when needed.
- Native UI, installers, signing/notarization, performance and accuracy are future
  work. No release artifacts or external model-based labeling were produced.
