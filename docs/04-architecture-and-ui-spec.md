# 04 — System Architecture & Minimalist UI/UX Specification

> Target architecture, not current implementation. Use the
> [development plan](08-development-plan.md) and [engine contract](10-engine-contract.md)
> for scope, acceptance and deterministic serialization. Native UI parity and
> performance require evidence on both operating systems.

---

## 1. Technology Stack & Workspace Layout

- **Core Engine & CLI:** **Rust** (`cargo` workspace, Edition 2021/2024).
  - Data parallelism via `rayon`.
  - Fast string metrics via `rapidfuzz` / `strsim`.
  - Unicode normalization via `unicode-normalization`.
  - Encoding detection via `encoding_rs`.
  - Serialization via `serde`, `serde_json`, `csv`, `quick-xml`.
- **Desktop Application:** **Tauri 2.0** (`app/`).
  - Uses native OS webviews (**Edge WebView2** on Windows, **WKWebView** on macOS) without bundling Chromium. Installer size and minimum supported OS versions must be measured/verified during packaging.
  - **Frontend Stack:** **React + TypeScript + Vite + Tailwind CSS + `shadcn/ui` (Radix Primitives) + `@tanstack/react-virtual`**.

```text
record-deduplicator/
├── AGENT.md / AGENTS.md        # Handoff instructions & mandatory rules for coding agents
├── TASKS.md                    # Ordered engineering backlog & milestone checklist
├── DECISIONS.md                # Architectural and product decision log
├── README.md                   # Project overview
├── Cargo.toml                  # Rust workspace manifest
├── crates/
│   ├── dedup-core/             # Pure Rust library (no OS-specific or network dependencies)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── model.rs        # CanonicalRecord, Provenance, DecisionClass
│   │   │   ├── encoding.rs     # UTF-8 / UTF-16 / Windows-1252 auto-decoder
│   │   │   ├── parsers/        # ris, nbib, wos, csv, endnote_xml, bibtex
│   │   │   ├── normalize/      # text, identifiers, authors, abstract_text, journal
│   │   │   ├── blocking.rs     # 6-key union blocking engine
│   │   │   ├── scoring.rs      # Tier 2a deterministic & Tier 2b fuzzy metrics
│   │   │   ├── guards.rs       # False-positive guards (G1–G7)
│   │   │   ├── classify.rs     # 5-class decision classifier
│   │   │   ├── cluster.rs      # Deterministic Union-Find + bridge validation
│   │   │   ├── merge.rs        # Master selection + field-level enrichment merge
│   │   │   └── export.rs       # RIS/CSV/JSON exporters + PRISMA 2020 / PRISMA-S text
│   │   └── tests/              # Golden-file parser & pipeline unit tests
│   └── dedup-cli/              # Command-line binary used by users & validation harness
│       └── src/main.rs
├── app/                        # Tauri 2 + React + TypeScript + Tailwind + shadcn/ui
├── validation/                 # External Python benchmark & LLM Oracle harness (NOT SHIPPED)
├── testdata/                   # Synthetic perturbation fixtures & database export samples
├── docs/                       # Complete methodological & technical documentation
└── .github/workflows/          # Cross-platform CI (Windows + macOS)
```

---

## 2. UI/UX Design Philosophy: Modern, Minimalist, Chic, Zero Learning Curve

The desktop app must **not** look like a legacy, cluttered bibliographic spreadsheet manager with dozens of cryptic toolbars. It should feel like a modern, focused studio tool (inspired by Linear / Raycast / modern `shadcn/ui` aesthetics):

### 2.1 Visual Language
- **Aesthetic:** Clean typography (`Inter` / `Segoe UI Variable` on Windows, `SF Pro` on macOS, `JetBrains Mono` for DOIs/identifiers/scores), generous whitespace, subtle `1px` borders, soft neutral surfaces (`zinc`/`slate`), and a single calm accent color, with automatic OS Light/Dark mode support.
- **No Configuration Wall:** Sensible, literature-validated defaults run out of the box with zero setup. An optional *"Advanced Rules & Discipline Profile"* drawer is tucked neatly behind a secondary button for power users.

### 2.2 Three-Screen Linear Workflow
The entire user journey fits into three self-explanatory steps:

1. **Step 1 — Drop Files (`Import`)**
   - A large, centered drag-and-drop card accepting `.ris`, `.nbib`, `.ciw`, `.csv`, `.xml`, and `.bib` files (single or multiple batches at once).
   - As files are dropped, display clean pill cards showing auto-detected database/vendor (e.g., *APA PsycINFO via EBSCO*, *ERIC*, *PubMed*), encoding, and record count.
   - Single primary CTA button: **`Deduplicate X,XXX Records →`**.

2. **Step 2 — Resolve Grey-Zone Pairs (`Review`)**
   - `EXACT_DUPLICATE` and `BIBLIOGRAPHIC_DUPLICATE` pairs are already auto-merged in the background; `DISTINCT` records are untouched.
   - The user is **only** shown the small subset of **`PROBABLE_DUPLICATE`** pairs (and optionally **`KIN_REPORT`** links) that genuinely benefit from a quick human glance.
   - **Side-by-Side Diff View:**
     - Left card (Master candidate) vs. Right card (Comparison record).
     - Identical text is muted; **only character/word differences in Title, Authors, Year, Journal, Volume/Pages, and Abstract are softly highlighted**.
     - **Plain-Language Reason Banner** at the top of each pair (e.g., *"Likely Online First vs. Print issue — 96% title match, same authors & journal, published 2024 vs. Vol 58(2) in 2025"* or *"Kin Study: Conference Abstract vs. Full Journal Article"*). Technical numeric scores (`S_title = 0.96`) appear on hover or toggle.
   - **Effortless Keyboard Triage:**
     - `→` or `M`: **Merge as Duplicate**
     - `←` or `K`: **Keep Both (Distinct)**
     - `L`: **Link as Kin Report (Keep Both, Linked)**
     - `↓` or `Space`: **Skip for Now**
     - `Ctrl/Cmd + Z`: **Undo** at any time.

3. **Step 3 — Export & PRISMA Summary (`Export`)**
   - **Visual PRISMA 2020 Flow Summary Card:**
     - Records identified per database/file (`N = ...`)
     - Total records before deduplication (`N = ...`)
     - Duplicate records removed (`N = ...` broken down by Exact vs. Bibliographic vs. Reviewed)
     - Unique records exported for screening (`N = ...`)
     - Related kin reports linked (`N = ...`)
   - **Copy Draft PRISMA-S Methods Paragraph:** Requires author review against the actual search and deduplication workflow.
   - **One-Click Exports:**
     - `Export Deduplicated Records (.ris / .csv)`
     - `Export Audit Log & Removed Duplicates (.csv / .json)`

---

## 3. Windows + macOS Cross-Platform Requirements (Enforced from Day One)

1. **Path & Filesystem Portability:**
   - Use `std::path::{Path, PathBuf}` everywhere in Rust and `@tauri-apps/api/path` in TypeScript. Never concatenate path strings with `\` or `/`.
2. **Deterministic Multi-Threading:**
   - When using `rayon::par_iter()` for blocking and pair scoring, always sort candidate pairs and clusters by canonical `(min_id, max_id)` before Union-Find clustering and export so outputs are **byte-for-byte identical** across Windows (x86_64) and macOS (Apple Silicon ARM64 / Intel x86_64).
3. **WebView Compatibility:**
   - Avoid Chromium-only CSS features so layouts render identically in Windows **Edge WebView2** and macOS **WKWebView (Safari engine)**.
4. **Keyboard Modifiers:**
   - Detect platform in React (`navigator.platform` / Tauri OS plugin) and display/bind `Ctrl` on Windows/Linux and `⌘ Cmd` on macOS.
5. **CI Matrix:**
   - GitHub Actions must compile and run `cargo test` on both `windows-latest` and `macos-latest` on every push.
