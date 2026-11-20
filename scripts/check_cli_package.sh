#!/usr/bin/env bash
# Run the four documented first-review blocks using the extracted executable.
set -euo pipefail
work=$(mktemp -d)
trap 'cd / && rm -rf "$work"' EXIT
tar -xzf "$1" -C "$work"
cd "$work"/recorddedu-*
if command -v sha256sum >/dev/null; then
    sha256sum -c SHA256SUMS
else
    shasum -a 256 -c SHA256SUMS
fi
export PATH="$PWD:$PATH"
if [[ $# == 2 ]]; then
    # A Linux cross-build can be exercised under Wine.
    runner=$2
    windows_binary="$PWD/dedup.exe"
    dedup() { env -u TMPDIR "$runner" "$windows_binary" "$@"; }
fi
dedup --version
awk '
    /^### Verify a copy found through metadata$/ { walkthrough = 1 }
    /^### Reconcile a reviewed export before screening$/ { exit }
    walkthrough && /^```sh$/ { block = 1; count++; next }
    /^```$/ { block = 0 }
    block { print }
    END { if (count != 4) exit 1 }
' README.md > "$work/first-run.sh"
export TMPDIR="$work"
source "$work/first-run.sh" > "$work/output.tsv"
grep -Fxq $'summary\tpending\t0\tgroups=0;flagged_groups=0' "$work/output.tsv"
grep -Fxq $'pending\ttitle_author_year_type\tidx:1\tinspect_metadata_only;keep=cat:1;assigned=' "$work/output.tsv"
grep -Fxq $'summary\tall\t3\tretained=2;suppressed=1' reviewed/mapping.tsv
grep -Fxq $'summary\tall\t3\tretained=3;suppressed=0' restored/mapping.tsv
grep -Fq $'map\tverified-duplicate-of=cat:1\tidx:1\t' reviewed/mapping.tsv
grep -Fxq 'TI  - Marsh transects after spring flooding: seasonal observations' reviewed/reviewed.ris
cat catalog.ris index.ris | cmp - restored/reviewed.ris
echo "PASS: packaged CLI verified one copy, retained the subtitled record and restored all three occurrences."

# Use the same three synthetic occurrences through the prompt-driven entry.
mkdir 'Search exports'
cp catalog.ris 'Search exports/Catalog export.ris'
cp index.ris 'Search exports/Index export.ris'
guided_inputs() {
    printf '%s\n' '"Review choices.tsv"' cat '"Search exports/Catalog export.ris"' \
        idx 'Search exports/Index export.ris' '' "$1"
}
guided_inputs :cancel | dedup start > cancelled.tsv 2> prompts.txt
test ! -e 'Review choices.tsv'
test ! -s cancelled.tsv
grep -Fq 'Review setup cancelled.' prompts.txt

guided_inputs yes | dedup start > guided.tsv 2> prompts.txt
review --init-metadata-decisions reference.tsv > reference-queue.tsv
cmp reference.tsv 'Review choices.tsv'
cmp reference-queue.tsv guided.tsv
grep -Fxq $'cat:1\tpending' 'Review choices.tsv'
grep -Fxq $'idx:1\tpending' 'Review choices.tsv'

# Reopen the partially reviewed sheet left by the walkthrough.
cp choices.tsv 'Review choices.tsv'
guided_inputs yes | dedup start > guided-pending.tsv 2> prompts.txt
review --pending-decisions choices.tsv > reference-pending.tsv
cmp reference-pending.tsv guided-pending.tsv
cmp choices.tsv 'Review choices.tsv'
cmp catalog.ris 'Search exports/Catalog export.ris'
cmp index.ris 'Search exports/Index export.ris'

# A changed source must be diagnosed before emitting any saved decisions.
printf '\n' >> 'Search exports/Index export.ris'
if guided_inputs yes | dedup start > rejected.tsv 2> prompts.txt; then exit 1; fi
test ! -s rejected.tsv
grep -Fq 'Decision sheet inputs differ.' prompts.txt
cmp choices.tsv 'Review choices.tsv'
echo "PASS: guided cancel, create, reopen and changed-source refusal with spaced paths."
