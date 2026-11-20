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
    checksum=(sha256sum)
else
    checksum=(shasum -a 256)
fi
"${checksum[@]}" -c SHA256SUMS
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
project='Étude de terrain'
handoff='Résultats partagés'
catalog="$project/Catalogue d'été.ris"
index="$project/Índice final.ris"
sheet="$project/Décisions de revue.tsv"
mkdir "$project" "$handoff"
cp catalog.ris "$catalog"
# The index has one true copy and one distinct report on different pages.
sed '/^DO  - /d' catalog.ris > guide-index.ris
sed '/^DO  - /d;s/^SP  - .*/SP  - 41-46/' catalog.ris >> guide-index.ris
cp guide-index.ris "$index"
reference_review() {
    dedup review-ids "$@" --ris-source cat catalog.ris --ris-source idx guide-index.ris
}
start_shell=direct
case "${RUNNER_OS:-$(uname -s)}" in
    Windows|MINGW*|MSYS*) start_shell=pwsh ;;
    macOS|Darwin) start_shell=zsh ;;
    Linux) if [[ $# == 1 && -x /bin/zsh ]]; then start_shell=zsh; fi ;;
esac

guide_path() {
    case "$start_shell" in
        pwsh) cygpath -aw "$1" ;;
        zsh) printf '%s/%s\n' "$PWD" "$1" ;;
        *) printf '%s\n' "$1" ;;
    esac
}
sheet_path=$(guide_path "$sheet")
catalog_path=$(guide_path "$catalog")
index_path=$(guide_path "$index")
sheet_answer="\"$sheet_path\""
index_answer=$index_path
case "$start_shell" in
    pwsh) index_answer="'$index_path'" ;; # PowerShell-style single-quoted argument.
    zsh)
        sheet_answer="'$sheet_path'"
        index_answer=$(/bin/zsh -fc 'print -r -- ${(q)1}' -- "$index_path")
        ;;
esac

guided_start() (
    printf '%s\n' "$sheet_answer" cat "\"$catalog_path\"" idx "$index_answer" '' "$@" > "$work/answers.txt"
    case "$start_shell" in
        pwsh)
            export REVIEW_ANSWERS
            REVIEW_ANSWERS=$(cygpath -aw "$work/answers.txt")
            cd "$package_dir"
            pwsh -NoProfile -NonInteractive -Command '
                $ErrorActionPreference = "Stop"
                [Console]::OutputEncoding = $OutputEncoding = [System.Text.UTF8Encoding]::new($false)
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

find "$project" "$handoff" -print | LC_ALL=C sort > "$work/before.txt"
if [[ $start_shell == pwsh ]]; then
    # Two quoted file paths pasted into one source prompt must not select either file.
    index_answer="\"$catalog_path\" \"$index_path\"" guided_start yes :cancel > cancelled.tsv 2> prompts.txt
    grep -Fq 'Cannot locate a regular file there.' prompts.txt
    test ! -s cancelled.tsv
    find "$project" "$handoff" -print | LC_ALL=C sort | cmp - "$work/before.txt"
    echo "PASS: PowerShell guide requested one source path and created no decision sheet."
fi
guided_start :cancel > cancelled.tsv 2> prompts.txt
find "$project" "$handoff" -print | LC_ALL=C sort | cmp - "$work/before.txt"
cmp catalog.ris "$catalog"
cmp guide-index.ris "$index"
test ! -s cancelled.tsv
grep -Fq 'Review setup cancelled.' prompts.txt
grep -Fq "cat = $catalog_path" prompts.txt
grep -Fq "idx = $index_path" prompts.txt

guided_start yes > guided.tsv 2> prompts.txt
reference_review --init-metadata-decisions reference.tsv > reference-queue.tsv
cmp reference.tsv "$sheet"
cmp reference-queue.tsv guided.tsv
grep -Fxq $'cat:1\tpending' "$sheet"
grep -Fxq $'idx:1\tpending' "$sheet"

# Record an explicit keeper and one verified copy. Keep the distinct report pending.
awk -F '\t' '
    $1 == "cat:1" {print "cat:1\tkeep"; next}
    $1 == "idx:1" {print "idx:1\tverified-duplicate-of=cat:1\tSame report on pages 21-26."; next}
    {print}
' "$sheet" > saved.tsv
cp saved.tsv "$sheet"
reference_review --pending-decisions saved.tsv > reference-pending.tsv
grep -Fxq $'pending\ttitle_author_year_type\tidx:2\tinspect_metadata_only;keep=cat:1;assigned=idx:1' reference-pending.tsv
guided_start yes '' > guided-pending.tsv 2> prompts.txt
cmp reference-pending.tsv guided-pending.tsv

dest="$handoff/À transmettre"
dest_path=$(guide_path "$dest")
dest_answer=$dest_path
if [[ $start_shell == zsh ]]; then
    dest_answer=$(/bin/zsh -fc 'print -r -- ${(q)1}' -- "$dest_path")
fi
mkdir "$handoff/Dossier occupé"
printf '%s\n' 'Previous output' > "$handoff/Dossier occupé/reviewed.ris"
find "$project" "$handoff" -print | LC_ALL=C sort > "$work/with-sheet.txt"
check_untouched() {
    find "$project" "$handoff" -print | LC_ALL=C sort | cmp - "$work/with-sheet.txt"
    printf '%s\n' 'Previous output' | cmp - "$handoff/Dossier occupé/reviewed.ris"
    cmp saved.tsv "$sheet"
    cmp catalog.ris "$catalog"
    cmp guide-index.ris "$index"
}
if [[ $start_shell == zsh ]]; then
    # Escaping just the new basename could also name a literal backslash.
    guided_start yes "${dest_path%/*}/À\ transmettre" :cancel > guided-pending.tsv 2> prompts.txt
    grep -Fq 'Paste the full path with spaces as written.' prompts.txt
    if grep -Fq 'Type write to create' prompts.txt; then exit 1; fi
    check_untouched
    echo "PASS: ambiguous zsh destination prompted for a path and wrote no files."
fi
guided_start yes "$dest_path" '' > guided-pending.tsv 2> prompts.txt
cmp reference-pending.tsv guided-pending.tsv
grep -Fq "Decision sheet: $sheet_path" prompts.txt
grep -Fq "New export folder: $dest_path" prompts.txt
grep -Fq 'Explicitly marked duplicates to remove: 1' prompts.txt
grep -Fq 'Records retained: 2 (including 1 pending).' prompts.txt
grep -Fq 'Export cancelled.' prompts.txt
check_untouched
if guided_start yes "$(guide_path "$handoff/Dossier occupé")" write > guided-pending.tsv 2> prompts.txt; then exit 1; fi
grep -Fq 'Destination already exists.' prompts.txt
check_untouched

guided_start yes "$dest_answer" write > guided-pending.tsv 2> prompts.txt
reference_review --export-reviewed saved.tsv reference-export
test "$(find "$dest" -print | wc -l)" -eq 4  # Directory and its three files.
for file in reviewed.ris mapping.tsv reconciliation.tsv; do
    cmp "reference-export/$file" "$dest/$file"
done
grep -Fxq $'summary\tall\t3\tretained=2;suppressed=1' "$dest/mapping.tsv"
grep -Fq $'map\tpending\tidx:2\t' "$dest/mapping.tsv"
grep -Fxq 'SP  - 41-46' "$dest/reviewed.ris"
for label in cat idx; do
    source_file=catalog.ris
    if [[ $label == idx ]]; then source_file=guide-index.ris; fi
    digest=$("${checksum[@]}" < "$source_file")
    grep -F $'input\tris\t'"$label"$'\t' "$dest/mapping.tsv" | grep -Fq "sha256=${digest%% *}"
done
cmp saved.tsv "$sheet"
cmp catalog.ris "$catalog"
cmp guide-index.ris "$index"
echo "PASS: $start_shell guided export outside the project matched all three reference files, removed one verified copy and kept two records including one pending."

# A changed source must be diagnosed before emitting any saved decisions.
if [[ $start_shell == zsh ]]; then
    # Both names exist. The literal spelling must select the different source.
    literal_index="$project/Índice\ final.ris"
    cp "$index" "$literal_index"
    printf '\n' >> "$literal_index"
    index_answer=$(guide_path "$literal_index")
else
    printf '\n' >> "$index"
fi
if guided_start yes > rejected.tsv 2> prompts.txt; then exit 1; fi
test ! -s rejected.tsv
grep -Fq 'Decision sheet inputs differ.' prompts.txt
if [[ $start_shell == zsh ]]; then
    grep -Fq "idx = $index_answer" prompts.txt
    echo "PASS: literal backslash filename selected while both spellings existed."
fi
cmp saved.tsv "$sheet"
echo "PASS: $start_shell guided start with spaced Unicode paths; cancellation preserved project bytes and entries; created 3 pending; reopened the partially reviewed sheet; changed source refused."
