#!/usr/bin/env bash
# Exercise the documented walkthrough and guided setup using the extracted executable.
set -euo pipefail
work=$(mktemp -d)
trap 'cd / && rm -rf "$work"' EXIT
mkdir "$work/Extracted package"
tar -xzf "$1" -C "$work/Extracted package"
cd "$work/Extracted package"/recorddedu-*
package_dir=$PWD
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

# Mimic pasted file paths in the packaged guide, including Windows drive letters.
project='Search exports'
sheet="$project/Review choices.tsv"
mkdir "$project"
cp catalog.ris "$project/Catalog export.ris"
cp index.ris "$project/Index export.ris"
start_shell=direct
case "${RUNNER_OS:-$(uname -s)}" in
    Windows|MINGW*|MSYS*) start_shell=pwsh ;;
    macOS|Darwin) start_shell=zsh ;;
esac

guide_path() {
    case "$start_shell" in
        pwsh) cygpath -aw "$1" ;;
        zsh) printf '%s/%s\n' "$PWD" "$1" ;;
        *) printf '%s\n' "$1" ;;
    esac
}
sheet_path=$(guide_path "$sheet")
catalog_path=$(guide_path "$project/Catalog export.ris")
index_path=$(guide_path "$project/Index export.ris")

guided_start() (
    printf '%s\n' "\"$sheet_path\"" cat "\"$catalog_path\"" idx "$index_path" '' "$1" > "$work/answers.txt"
    case "$start_shell" in
        pwsh)
            export REVIEW_ANSWERS
            REVIEW_ANSWERS=$(cygpath -aw "$work/answers.txt")
            cd "$package_dir"
            pwsh -NoProfile -NonInteractive -Command '
                $ErrorActionPreference = "Stop"
                Get-Content -LiteralPath $env:REVIEW_ANSWERS -Encoding utf8 | .\dedup.exe start
                exit $LASTEXITCODE
            ' | sed $'s/\r$//'  # Compare displayed rows across shell line endings.
            ;;
        zsh)
            cd "$package_dir"
            /bin/zsh -fc './dedup start' < "$work/answers.txt"
            ;;
        *) dedup start < "$work/answers.txt" ;;
    esac
)

find "$project" -print | LC_ALL=C sort > "$work/before.txt"
guided_start :cancel > cancelled.tsv 2> prompts.txt
find "$project" -print | LC_ALL=C sort | cmp - "$work/before.txt"
cmp catalog.ris "$project/Catalog export.ris"
cmp index.ris "$project/Index export.ris"
test ! -s cancelled.tsv
grep -Fq 'Review setup cancelled.' prompts.txt

guided_start yes > guided.tsv 2> prompts.txt
review --init-metadata-decisions reference.tsv > reference-queue.tsv
cmp reference.tsv "$sheet"
cmp reference-queue.tsv guided.tsv
grep -Fxq $'cat:1\tpending' "$sheet"
grep -Fxq $'idx:1\tpending' "$sheet"

# Record one explicit keep on the new sheet. Leave the other row pending.
sed $'s/^cat:1\tpending$/cat:1\tkeep/' "$sheet" > edit.tmp
cat edit.tmp > "$sheet"
cmp choices.tsv "$sheet"
guided_start yes > guided-pending.tsv 2> prompts.txt
review --pending-decisions choices.tsv > reference-pending.tsv
cmp reference-pending.tsv guided-pending.tsv
cmp choices.tsv "$sheet"
cmp catalog.ris "$project/Catalog export.ris"
cmp index.ris "$project/Index export.ris"

# A changed source must be diagnosed before emitting any saved decisions.
printf '\n' >> "$project/Index export.ris"
if guided_start yes > rejected.tsv 2> prompts.txt; then exit 1; fi
test ! -s rejected.tsv
grep -Fq 'Decision sheet inputs differ.' prompts.txt
cmp choices.tsv "$sheet"
echo "PASS: $start_shell guided start with spaced paths; cancellation preserved project bytes and entries; created 2 pending; reopened 1 keep and 1 pending; changed source refused."
