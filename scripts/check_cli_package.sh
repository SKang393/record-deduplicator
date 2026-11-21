#!/usr/bin/env bash
# Exercise the documented walkthrough and guided setup using the extracted executable.
set -euo pipefail
work=$(mktemp -d)
trap 'cd / && rm -rf "$work"' EXIT
mkdir "$work/verify-pairs"
cp -R "$(dirname "$0")/../testdata/fixtures/reviewed-export-v1" "$work/verify-pairs/legacy"
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
    END { if (count != 5) exit 1 }
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

# Compare ordinary review output with the mixed-source control from 0d61ee7441e3.
(
    mkdir input-check
    cd input-check
    bom=$'\xef\xbb\xbf'
    printf '%s\r\n' "${bom}TY  - JOUR" 'TI  - Tidal survey protocol' \
        "N1  - Été ${bom}à marée basse, ${bom}TY  -texte" 'DO  - 10.0000/tide' 'ER  -' > catalog.ris
    printf '%s\n' 'TY  - JOUR' 'TI  - Tidal survey follow-up' 'SP  - ahead of print' \
        'DO  - 10.0000/tide' 'ER  -' > index.ris
    review_input() { dedup review-ids "$@" --ris-source cat catalog.ris --ris-source idx index.ris; }
    review_input --init-decisions choices.tsv > queue.tsv
    review_input --inspect-pair choices.tsv cat:1 idx:1 > inspection.txt
    grep -Fxq 'cat:1 has no SP or EP field for a final page range or article number.' inspection.txt
    grep -Fxq 'idx:1 supplies pagination fields without a usable final page range or article number.' inspection.txt
    review_input --export-reviewed choices.tsv clean
    cat > "$work/baseline.sha256" <<'SUMS'
1d82abac0229ac0219fbac70bee42653b1e944bfe23a075b75693c97a2c95d4e *catalog.ris
390bb4a3fbc69f21eb4dc04f448c3fc8624c641dd8fca22b726ed0dc815d88b1 *index.ris
8106ad9551dfe5a8fa74ac6c12292a837affdc72495b1bbc7e6961d1d027a6b0 *choices.tsv
0a505359c056098596e7ead809b1c6224e333ae648969b3009c2435e004de3fa *queue.tsv
a3cb119441cbfee42cb2d85e5561e94eccf2990e740c968114f25ba13a2ed472 *inspection.txt
0e84d084db3d065bc3572211880e45d2929060243246acb79b4000571ea2a2dc *clean/reviewed.ris
48314f0dce442cd7c83355e561c2e6f0977e8c91c66cb9c917ccd22134d28afe *clean/mapping.tsv
59def875d9cd1a68023e3e9301e260a21965fc7a04345a9c4d4915d0a7b2c0c5 *clean/reconciliation.tsv
SUMS
    cp catalog.ris renamed.ris
    cat catalog.ris index.ris > combined.ris
    # Capture coverage output outside this project so every project file can be checked.
    find . -print | LC_ALL=C sort > "$work/project-before.txt"
    report="$work/coverage.tsv"
    coverage_case() {
        local expected=$1 status=0
        shift
        dedup review-ids --check-coverage "$@" > "$report" 2> "$work/coverage.err" || status=$?
        test "$status" -eq "$expected"
    }
    catalog=(--ris-source cat catalog.ris)
    both=("${catalog[@]}" --ris-source idx index.ris)
    tide=(tide --expected-total 2 --batch-range cat 1-1)
    plan=("${tide[@]}" --batch-range idx 2-2)
    coverage_case 0 "${plan[@]}" "${both[@]}"
    grep -Fxq $'snapshot\ttide\tparsed=2\texpected_total=2' "$report"
    grep -Fxq $'result\ttide\tmatched_expectations' "$report"
    coverage_case 2 "${plan[@]}" "${catalog[@]}"
    grep -Fxq $'issue\tmissing_batch\tidx\tpositions=2-2' "$report"
    coverage_case 2 "${tide[@]}" --batch-range idx 1-1 "${both[@]}"
    grep -Fxq $'snapshot\ttide\tparsed=2\texpected_total=2' "$report"
    grep -Fxq $'issue\trange_overlap\tidx\tpositions=1-1' "$report"
    grep -Fxq $'issue\trange_gap\tpositions=2-2' "$report"
    coverage_case 2 "${plan[@]}" "${catalog[@]}" --ris-source idx renamed.ris
    grep -Fxq $'counts_and_ranges\ttide\tmatched_expectations' "$report"
    grep -Fxq $'source_flag\trepeated_source_bytes\tcat,idx' "$report"
    grep -Fxq $'result\ttide\tdiscrepancies' "$report"
    coverage_case 0 tide --expected-total 3 --batch-range cat 1-1 --batch-range idx 2-3 \
        "${catalog[@]}" --ris-source idx combined.ris
    grep -Fxq $'snapshot\ttide\tparsed=3\texpected_total=3' "$report"
    grep -Fxq $'result\ttide\tmatched_expectations' "$report"
    if grep -q $'^source_flag\t' "$report"; then exit 1; fi
    coverage_case 0 tide-update --expected-total 1 --ris-source cat renamed.ris
    grep -Fxq $'snapshot\ttide-update\tparsed=1\texpected_total=1' "$report"
    grep -Fxq $'result\ttide-update\tmatched_expectations' "$report"
    if grep -q $'^source_flag\t' "$report"; then exit 1; fi
    coverage_case 2 "${plan[@]}" --check-coverage tide-update --expected-total 1 "${both[@]}"
    test ! -s "$report"
    test -s "$work/coverage.err"
    find . -print | LC_ALL=C sort | cmp - "$work/project-before.txt"
    "${checksum[@]}" -c "$work/baseline.sha256"
    cmp catalog.ris renamed.ris
    cat catalog.ris index.ris | cmp - combined.ris
    review_input | cmp - queue.tsv
    echo "PASS: renamed download flagged with 2 of 2 positions matched. Distinct batches sharing a record and a separate snapshot passed."
    echo "PASS: coverage matched 2, flagged a missing batch and equal-total overlap/gap, and checked the update separately."
    echo "PASS: project entries and byte hashes match the ordinary review baseline after all coverage checks."

    printf 'TY  - JOUR\r\nTI  - Enquêt' >> index.ris
    printf '%s\n' "${bom}TY  - JOUR" 'TI  - Next batch' 'ER  -' >> index.ris
    "${checksum[@]}" -b catalog.ris index.ris choices.tsv clean/* > before.sha256
    reject_join() {
        if review_input "$@" > queue.tsv 2> error.txt; then exit 1; else test "$?" -eq 2; fi
        test ! -s queue.tsv
        grep -Fq 'idx:2 (bytes 86..148, malformed). A UTF-8 BOM precedes a TY header inside the file at byte 111.' error.txt
    }
    reject_join --init-decisions new.tsv
    reject_join --export-reviewed choices.tsv incomplete
    test ! -e new.tsv
    test ! -e incomplete
    "${checksum[@]}" -c before.sha256
    echo "PASS: interrupted idx:2 rejected before sheet or export at embedded header byte 111."
)

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
grep -Fq 'Pending rows in your new decision sheet: 3.' prompts.txt
reference_review --init-metadata-decisions reference.tsv > reference-queue.tsv
cmp reference.tsv "$sheet"
cmp reference-queue.tsv guided.tsv
grep -Fxq $'cat:1\tpending' "$sheet"
grep -Fxq $'idx:1\tpending' "$sheet"

# Inspect two pending members with no keeper, then export in that same guide session.
reference_review --pending-decisions reference.tsv > initial-pending.tsv
grep -Fxq $'pending\ttitle_author_year_type\tcat:1,idx:1,idx:2\tinspect_metadata_only;keep=;assigned=' initial-pending.tsv
reference_review --inspect-pair reference.tsv cat:1 idx:2 > pair.txt
grep -Fxq '  SP  - 41-46' pair.txt
pending_dest="$handoff/Pending comparison"
guided_start yes ':inspect cat:1,idx:2' "$(guide_path "$pending_dest")" write > inspected.txt 2> prompts.txt
cat initial-pending.tsv pair.txt | cmp - inspected.txt
grep -Fq 'Records retained: 3 (including 3 pending).' prompts.txt
grep -Fxq $'summary\tall\t3\tretained=3;suppressed=0' "$pending_dest/mapping.tsv"
reference_review --export-reviewed reference.tsv pending-export
for file in reviewed.ris mapping.tsv reconciliation.tsv; do
    cmp "pending-export/$file" "$pending_dest/$file"
done
cmp reference.tsv "$sheet"
echo "PASS: $start_shell inspected two pending records without a keeper and exported all three in the same session."

# Wait for the actual pending view before changing a disposable copy of the index.
cp guide-index.ris stale-index.ris
stale_answers() {
    printf '%s\n' reference.tsv cat catalog.ris idx stale-index.ris '' yes
    for ((attempt=0; attempt<200; attempt++)); do
        if cmp -s initial-pending.tsv stale-output.tsv; then
            printf '\n' >> stale-index.ris
            printf '%s\n' ':inspect cat:1,idx:2'
            return 0
        fi
        sleep 0.05
    done
    echo 'Timed out waiting for the pending view.' >&2
    return 1
}
if stale_answers | dedup start > stale-output.tsv 2> stale-prompts.txt; then
    exit 1
else
    statuses=("${PIPESTATUS[@]}")
    [[ ${statuses[0]} == 0 && ${statuses[1]} == 2 ]]
fi
cmp initial-pending.tsv stale-output.tsv
grep -Fq 'idx: RIS source changed while being read' stale-prompts.txt
if grep -Fq 'Marsh transects after spring flooding' initial-pending.tsv prompts.txt stale-prompts.txt; then exit 1; fi
cmp reference.tsv "$sheet"
echo 'PASS: changing the index after the pending view refused inspection with exit 2 and displayed no source record.'

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

# Trace the existing three-occurrence reference export after archiving its downloads.
mkdir -p downloads/keepers
mv guide-index.ris "downloads/Index batch.ris"
mv catalog.ris "downloads/keepers/Main batch.ris"
sed 's/JOUR/BOOK/' "downloads/keepers/Main batch.ris" > "downloads/Main batch.ris"
mapping=reference-export/mapping.tsv
dedup trace "$mapping" idx:1 downloads downloads/keepers > "$work/traces.tsv"
dedup trace "$mapping" idx:2 downloads >> "$work/traces.tsv"
# Compare the entire display, including filenames, with the synthetic baseline.
cat > "$work/expected-traces.tsv" <<'ROWS'
occurrence	decision	output	original_bytes	source_sha256	matching_path
idx:1	verified-duplicate-of=cat:1	1	bytes=0..127	90c395d4d5099f5925c108d2a699e009b703bcbffa12e279499d2d33e269b429	"downloads/Index batch.ris"
cat:1	keep	1	bytes=0..149	b18a5f6080b1d99e7d7800d10967368e957bea2cd14a8b71b859e903599d24ad	"downloads/keepers/Main batch.ris"
occurrence	decision	output	original_bytes	source_sha256	matching_path
idx:2	pending	2	bytes=127..254	90c395d4d5099f5925c108d2a699e009b703bcbffa12e279499d2d33e269b429	"downloads/Index batch.ris"
ROWS
# The synthetic names contain no literal backslashes. Rust quotes Windows separators.
sed 's|\\\\|/|g' "$work/traces.tsv" | cmp - "$work/expected-traces.tsv"

reject_trace() {
    local problem=$1
    shift
    if dedup trace "$@" > "$work/trace.tsv" 2> "$work/trace.err"; then exit 1; else test "$?" -eq 2; fi
    test ! -s "$work/trace.tsv"
    grep -Fq "$problem" "$work/trace.err"
}
# With only the parent selected the real keeper is out of scope. The substitute has equal size.
reject_trace 'No original file matches cat:1' "$mapping" idx:1 downloads
sed 's/bytes=0\.\.127;output=1/bytes=0..126;output=1/' "$mapping" > changed-span.tsv
reject_trace 'Original occurrence idx:1 disagrees' changed-span.tsv idx:1 downloads downloads/keepers

# Byte hashes from the same guided fixture exported at e92ade288ad7, before trace existed.
cat > "$work/trace-export.sha256" <<'SUMS'
b0cedb50c703d63d28832635724ebd4e8d5e0db177a509af305487a263e0d68a *reference-export/reviewed.ris
4364562f7dd989c5e5c7c64df1d5b2d890e4bb7da4643f8f7f28d20e8e160e6e *reference-export/mapping.tsv
c12cb656ce49f76d67b58a056fed25ecc225a46f18e31284c5b7abf4abdca2c4 *reference-export/reconciliation.tsv
SUMS
"${checksum[@]}" -c "$work/trace-export.sha256"
echo "PASS: trace found the verified copy and keeper at output 1 and the pending report at output 2 in renamed files."
echo "PASS: same-size substitute with keeper outside the selected folders and altered span both exited 2 with empty stdout; all three export hashes matched."

# Only the two handoff folders are available to the verifier in its working directory.
cp -R reference-export "$work/verify-pairs/current"
cd "$work/verify-pairs"
touch -t 200101010000 "$work/pairs.stamp"
find . -depth -exec touch -r "$work/pairs.stamp" {} +
find . -print | LC_ALL=C sort > "$work/pairs-before.txt"
"${checksum[@]}" -b current/* legacy/* > "$work/pairs-before.sha256"
verify_pair() (
    cd "$1"
    test "$(grep -c '^TY  -' reviewed.ris)" -eq 2
    dedup verify-export mapping.tsv reviewed.ris > "$work/pair.tsv"
    grep -Fxq $'matched_output_fingerprint\t"mapping.tsv"\t'"$2"$'\t"reviewed.ris"\t276\t'"$3" "$work/pair.tsv"
)
verify_pair current 4364562f7dd989c5e5c7c64df1d5b2d890e4bb7da4643f8f7f28d20e8e160e6e b0cedb50c703d63d28832635724ebd4e8d5e0db177a509af305487a263e0d68a
verify_pair legacy 81d4818ca49610bd162e98e98107c64da8fefcc9fb07d81045c7d61e933977f2 cb204a3646da21d35ffb49bf904b40bd4b1f882b363d528ba9a8578ee349055d
if dedup verify-export current/mapping.tsv legacy/reviewed.ris > "$work/pair.tsv" 2> "$work/pair.err"; then
    exit 1
else
    test "$?" -eq 2
fi
test ! -s "$work/pair.tsv"
grep -Fxq 'Reviewed RIS SHA-256 differs from the mapping output row.' "$work/pair.err"
find . -print | LC_ALL=C sort | cmp - "$work/pairs-before.txt"
test -z "$(find . -newer "$work/pairs.stamp" -print)"
"${checksum[@]}" -c "$work/pairs-before.sha256"
echo "PASS: current and frozen v1 pairs matched; swapped two-record, 276-byte RIS exited 2 with empty stdout."
echo "PASS: handoff paths, timestamps and file hashes were unchanged."
