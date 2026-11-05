#!/bin/bash
set -euo pipefail
umask 077
work="$RUNNER_TEMP/dedup-private"
source_dir="$work/source"
mkdir -p "$source_dir" "$work/evidence"
python3 scripts/safe-extract.py "$work/source.zip" "$source_dir" source
python3 scripts/safe-extract.py "$work/overlay.zip" "$source_dir" overlay
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
rustup target add --toolchain 1.99.0 aarch64-apple-darwin x86_64-apple-darwin
(cd "$source_dir" && cargo +1.99.0 fetch --locked)
(cd "$source_dir" && cargo +1.99.0 fetch --manifest-path app/src-tauri/Cargo.toml --locked)
(cd "$source_dir/app" && npm ci --ignore-scripts)
command -v sandbox-exec >/dev/null
python3 scripts/egress-check.py probe "$work/egress-target.json"
node - "$GITHUB_WORKSPACE" "$work/offline.sb" <<'NODE'
const fs = require('node:fs');
const [workspace, output] = process.argv.slice(2);
// Build processes cannot write the checked-out crypto helper used by the next step.
fs.writeFileSync(output, `(version 1)\n(allow default)\n(deny network*)\n(deny file-write* (subpath ${JSON.stringify(workspace)}))\n`, { flag: 'wx' });
NODE
