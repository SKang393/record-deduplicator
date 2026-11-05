#!/bin/bash
set -euo pipefail
umask 077
unset MACOS_READINESS_KEY GITHUB_ENV GITHUB_OUTPUT GITHUB_PATH GITHUB_STEP_SUMMARY ACTIONS_ID_TOKEN_REQUEST_URL ACTIONS_ID_TOKEN_REQUEST_TOKEN
work="$RUNNER_TEMP/native-private"
evidence="$work/evidence"
mkdir -p "$work/home" "$work/fixtures"
cat > "$work/fixtures/synthetic.ris" <<'RIS'
TY  - JOUR
TI  - A synthetic meadow study
AU  - Example, Alex
PY  - 2024
DO  - 10.1234/synthetic-meadow
ER  -
TY  - JOUR
TI  - A synthetic river study
AU  - Example, Morgan
PY  - 2022
DO  - 10.1234/synthetic-river
ER  -
RIS
shasum -a 256 -c "$evidence/sandbox-profile.sha256"
sandbox-exec -f "$work/offline.sb" python3 scripts/egress-check.py deny "$work/egress-target.json"
status=0
HOME="$work/home" python3 -B scripts/dialog-probe/probe.py --app "$work/corrected-source/Record Deduplicator.app" --fixture "$work/fixtures/synthetic.ris" --output "$evidence/native-dialog" --sandbox-profile "$work/offline.sb" --allow-unavailable-bundle-id --timeout-seconds 120 || status=$?
printf '%s\n' "$status" > "$evidence/native-dialog-exit.txt"
shasum -a 256 -c "$evidence/sandbox-profile.sha256"
exit "$status"
