#!/bin/bash
set -euo pipefail
umask 077
unset MACOS_READINESS_KEY GITHUB_ENV GITHUB_OUTPUT GITHUB_PATH GITHUB_STEP_SUMMARY ACTIONS_ID_TOKEN_REQUEST_URL ACTIONS_ID_TOKEN_REQUEST_TOKEN
export CARGO_NET_OFFLINE=true CARGO_TERM_COLOR=never npm_config_offline=true RUSTUP_TOOLCHAIN=1.99.0-aarch64-apple-darwin PYTHONDONTWRITEBYTECODE=1
work="$RUNNER_TEMP/native-private"
evidence="$work/evidence"
source="$work/source"
test "$(uname -m)" = arm64
python3 scripts/egress-check.py deny "$work/egress-target.json"
python3 scripts/memory-verify-source.py "$work/candidate.zip" "$source" > "$evidence/source-before.txt"
cd "$source"
rustc +1.99.0-aarch64-apple-darwin -vV > "$evidence/rust-version.txt"
grep -Fx 'host: aarch64-apple-darwin' "$evidence/rust-version.txt"
cargo +1.99.0-aarch64-apple-darwin test --workspace --locked
cargo +1.99.0-aarch64-apple-darwin build -p dedup-cli --release --locked --offline
"$source/target/release/dedup" --version > "$evidence/cli-version.txt"
grep -Fx "dedup 0.1.0-validation.4+memory.1 (engine 0.1.0-validation.4)" "$evidence/cli-version.txt"
lipo -archs "$source/target/release/dedup" > "$evidence/cli-architectures.txt"
test "$(cat "$evidence/cli-architectures.txt")" = arm64
cargo +1.99.0-aarch64-apple-darwin test --manifest-path app/src-tauri/Cargo.toml --locked --lib
(cd app && npm test && npm run build)
(cd app && npm run tauri -- build --no-sign --ci --target aarch64-apple-darwin --bundles app -- --locked)
bundle="$source/app/src-tauri/target/aarch64-apple-darwin/release/bundle/macos"
apps=("$bundle"/*.app)
[ "${#apps[@]}" -eq 1 ] && [ -d "${apps[0]}" ]
name=$(/usr/libexec/PlistBuddy -c 'Print CFBundleExecutable' "${apps[0]}/Contents/Info.plist")
exe="${apps[0]}/Contents/MacOS/$name"
lipo -archs "$exe" > "$evidence/architectures.txt"
test "$(cat "$evidence/architectures.txt")" = arm64
shasum -a 256 "$exe" > "$evidence/app-executable.sha256"
printf '%s\n' "${apps[0]}" > "$work/app-path.txt"
python3 "$GITHUB_WORKSPACE/scripts/memory-verify-source.py" "$work/candidate.zip" "$source" > "$evidence/source-after.txt"
