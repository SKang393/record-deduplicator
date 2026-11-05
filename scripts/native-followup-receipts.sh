#!/bin/bash
set -euo pipefail
umask 077
unset MACOS_READINESS_KEY GITHUB_ENV GITHUB_OUTPUT GITHUB_PATH GITHUB_STEP_SUMMARY ACTIONS_ID_TOKEN_REQUEST_URL ACTIONS_ID_TOKEN_REQUEST_TOKEN
export PYTHONDONTWRITEBYTECODE=1
work="$RUNNER_TEMP/native-private"
python3 -B scripts/safe-extract.py "$work/receipts.zip" "$work/receipt-source" receipts
cd "$work/receipt-source"
python3 -B -m unittest discover -s scripts/tests -v
