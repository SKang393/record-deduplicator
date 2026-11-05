#!/bin/bash
set -euo pipefail
umask 077
unset MACOS_READINESS_KEY GITHUB_ENV GITHUB_OUTPUT GITHUB_PATH GITHUB_STEP_SUMMARY ACTIONS_ID_TOKEN_REQUEST_URL ACTIONS_ID_TOKEN_REQUEST_TOKEN
work="$RUNNER_TEMP/native-private"
command -v sandbox-exec >/dev/null
python3 scripts/egress-check.py probe "$work/egress-target.json"
node - "$GITHUB_WORKSPACE" "$work/offline.sb" <<'NODE'
const fs = require('node:fs');
const [workspace, output] = process.argv.slice(2);
fs.writeFileSync(output, `(version 1)\n(allow default)\n(deny network*)\n(deny file-write* (subpath ${JSON.stringify(workspace)}))\n(deny file-write* (literal ${JSON.stringify(output)}))\n`, { flag: 'wx' });
NODE
sandbox-exec -f "$work/offline.sb" python3 scripts/egress-check.py deny "$work/egress-target.json"
shasum -a 256 "$work/offline.sb" > "$work/evidence/sandbox-profile.sha256"
python3 -B scripts/native-followup-controls.py "$work/offline.sb" "$work/evidence/automation-controls.json"
sw_vers > "$work/evidence/host.txt"
uname -a >> "$work/evidence/host.txt"
python3 --version >> "$work/evidence/host.txt"
