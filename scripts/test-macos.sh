#!/bin/bash
set -euo pipefail
umask 077
unset GITHUB_ENV GITHUB_OUTPUT GITHUB_PATH GITHUB_STEP_SUMMARY ACTIONS_ID_TOKEN_REQUEST_URL ACTIONS_ID_TOKEN_REQUEST_TOKEN
export CARGO_NET_OFFLINE=true CARGO_TERM_COLOR=never npm_config_offline=true RUSTUP_TOOLCHAIN=1.99.0 PYTHONDONTWRITEBYTECODE=1
work="$RUNNER_TEMP/dedup-private"
evidence="$work/evidence"
source_dir="$work/source"
python3 scripts/egress-check.py deny "$work/egress-target.json"
cd "$source_dir"
sw_vers > "$evidence/host.txt"
uname -a >> "$evidence/host.txt"
xcodebuild -version >> "$evidence/host.txt"
rustc +1.99.0 --version >> "$evidence/host.txt"
rustup show active-toolchain >> "$evidence/host.txt"
rustup target list --installed > "$evidence/rust-targets-active.txt"
grep -Fx aarch64-apple-darwin "$evidence/rust-targets-active.txt"
grep -Fx x86_64-apple-darwin "$evidence/rust-targets-active.txt"
node --version >> "$evidence/host.txt"
cargo +1.99.0 fmt --all --check
cargo +1.99.0 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.99.0 test --workspace --locked
cargo +1.99.0 build --locked -p dedup-cli
DEDUP_CLI="$source_dir/target/debug/dedup" python3 -B -m unittest discover -s validation/tests -v
python3 -B -m unittest discover -s scripts/tests -v
mkdir -p "$work/fixtures"
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
shasum -a 256 "$work/fixtures/synthetic.ris" > "$evidence/synthetic-before.sha256"
target/debug/dedup run --output "$work/output" --source synthetic Synthetic "$work/fixtures/synthetic.ris"
target/debug/dedup verify-determinism "$work/output" > "$evidence/determinism.json"
test -s "$work/output/project.json"
test -s "$work/output/retained.ris"
test -s "$work/output/retained.csv"
shasum -a 256 "$work/fixtures/synthetic.ris" > "$evidence/synthetic-after.sha256"
cmp "$evidence/synthetic-before.sha256" "$evidence/synthetic-after.sha256"
(cd app && npm test && npm run build)
cargo +1.99.0 fmt --manifest-path app/src-tauri/Cargo.toml --all --check
cargo +1.99.0 clippy --manifest-path app/src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo +1.99.0 test --manifest-path app/src-tauri/Cargo.toml --locked --lib -- --list > "$evidence/desktop-test-list.txt"
grep -F 'recovery::tests::unix_directory_sync_opens_and_flushes_created_entries: test' "$evidence/desktop-test-list.txt"
grep -F 'recovery::tests::active_journal_and_commit_paths_cannot_be_replaced_silently: test' "$evidence/desktop-test-list.txt"
cargo +1.99.0 test --manifest-path app/src-tauri/Cargo.toml --locked --lib
cargo +1.99.0 test --manifest-path app/src-tauri/Cargo.toml --locked --test permissions
(cd app && npm run tauri -- build --no-sign --ci --target universal-apple-darwin --bundles app -- --locked)
bundle_root="$source_dir/app/src-tauri/target/universal-apple-darwin/release/bundle/macos"
apps=("$bundle_root"/*.app)
[ "${#apps[@]}" -eq 1 ] && [ -d "${apps[0]}" ]
app="${apps[0]}"
exe_name=$(/usr/libexec/PlistBuddy -c 'Print CFBundleExecutable' "$app/Contents/Info.plist")
exe="$app/Contents/MacOS/$exe_name"
lipo -archs "$exe" > "$evidence/architectures.txt"
lipo -verify_arch arm64 x86_64 "$exe"
codesign -dv --verbose=2 "$app" > "$evidence/signature.txt" 2>&1 || true
find "$app" -type f -print > "$evidence/app-files.txt"
dmg_status=0
if hdiutil create -volname 'Record Deduplicator' -srcfolder "$app" -format UDZO "$evidence/unsigned-review.dmg"; then
  hdiutil verify "$evidence/unsigned-review.dmg" || dmg_status=$?
else
  dmg_status=$?
fi
if [ -f "$evidence/unsigned-review.dmg" ]; then
  shasum -a 256 "$evidence/unsigned-review.dmg" > "$evidence/dmg.sha256"
fi
printf '%s\n' "$dmg_status" > "$evidence/dmg-exit.txt"
mkdir -p "$work/probe-home"
HOME="$work/probe-home" "$exe" > "$evidence/native-launch.log" 2>&1 &
pid=$!
sleep 10
if kill -0 "$pid" 2>/dev/null; then
  printf '%s\n' 'Process remained alive for 10 seconds; UI, dialogs, and interactions were not verified.' > "$evidence/native-launch-status.txt"
  python3 -B "$GITHUB_WORKSPACE/scripts/native-probe.py" "$pid" "$evidence"
  kill -TERM "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
else
  wait "$pid" 2>/dev/null || true
  printf '%s\n' 'INCONCLUSIVE: process exited before 10 seconds; inspect native-launch.log and runner display availability.' > "$evidence/native-launch-status.txt"
fi
mkdir -p "$work/dialog-home"
dialog_status=0
HOME="$work/dialog-home" python3 -B "$GITHUB_WORKSPACE/scripts/dialog-probe/probe.py" --app "$app" --fixture "$work/fixtures/synthetic.ris" --output "$evidence/native-dialog" --timeout-seconds 120 || dialog_status=$?
printf '%s\n' "$dialog_status" > "$evidence/native-dialog-exit.txt"
if [ "$dialog_status" -ne 0 ]; then
  echo 'Native dialog probe did not pass; inspect its encrypted status and logs.'
  exit "$dialog_status"
fi
if [ "$dmg_status" -ne 0 ]; then
  echo 'DMG creation or verification failed; inspect encrypted evidence.'
  exit "$dmg_status"
fi
