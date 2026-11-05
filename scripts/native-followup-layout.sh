#!/bin/bash
set -euo pipefail
umask 077
unset MACOS_READINESS_KEY
work="$RUNNER_TEMP/native-private"
evidence="$work/evidence"
original_mount="$work/original-mount"
corrected_mount="$work/corrected-mount"
app_parent="$work/corrected-source"
app="$app_parent/Record Deduplicator.app"
mkdir -p "$original_mount" "$corrected_mount" "$app_parent"
original_mounted=0
corrected_mounted=0
cleanup() {
  if [ "$corrected_mounted" -eq 1 ]; then python3 -B scripts/native-followup-detach.py "$corrected_mount" >>"$evidence/detach.log" 2>&1 || true; fi
  if [ "$original_mounted" -eq 1 ]; then python3 -B scripts/native-followup-detach.py "$original_mount" >>"$evidence/detach.log" 2>&1 || true; fi
}
trap cleanup EXIT
hdiutil verify "$work/original.dmg"
shasum -a 256 "$work/original.dmg" > "$evidence/original-dmg.sha256"
hdiutil attach -nobrowse -readonly -mountpoint "$original_mount" "$work/original.dmg"
original_mounted=1
ls -la "$original_mount" > "$evidence/original-mount-root.txt"
find "$original_mount" -print > "$evidence/original-mount-files.txt"
shopt -s nullglob
original_apps=("$original_mount"/*.app)
if [ "${#original_apps[@]}" -eq 1 ] && [ -d "${original_apps[0]}/Contents" ]; then
  printf '%s\n' 'Original image contains an outer app bundle.' > "$evidence/original-layout.txt"
  ditto "${original_apps[0]}" "$app"
elif [ "${#original_apps[@]}" -eq 0 ] && [ -d "$original_mount/Contents" ]; then
  printf '%s\n' 'Original image contains Contents at the root; app bundle reconstructed for testing.' > "$evidence/original-layout.txt"
  mkdir -p "$app"
  ditto "$original_mount/Contents" "$app/Contents"
else
  echo 'Original mounted layout does not have exactly one recognized app.'
  exit 1
fi
exe_name=$(/usr/libexec/PlistBuddy -c 'Print CFBundleExecutable' "$app/Contents/Info.plist")
exe="$app/Contents/MacOS/$exe_name"
expected=75178ce01f85330caf9224abc6ca7de3d75cc2b5554670abf423022c76737121
[ "$(shasum -a 256 "$exe" | awk '{print $1}')" = "$expected" ]
shasum -a 256 "$exe" > "$evidence/staged-executable.sha256"
lipo -archs "$exe" > "$evidence/staged-architectures.txt"
lipo "$exe" -verify_arch arm64 x86_64
codesign -dv --verbose=2 "$app" > "$evidence/staged-signature.txt" 2>&1 || true
hdiutil create -volname 'Record Deduplicator' -srcfolder "$app_parent" -format UDZO "$evidence/unsigned-review-corrected.dmg"
hdiutil verify "$evidence/unsigned-review-corrected.dmg"
shasum -a 256 "$evidence/unsigned-review-corrected.dmg" > "$evidence/corrected-dmg.sha256"
hdiutil attach -nobrowse -readonly -mountpoint "$corrected_mount" "$evidence/unsigned-review-corrected.dmg"
corrected_mounted=1
ls -la "$corrected_mount" > "$evidence/corrected-mount-root.txt"
find "$corrected_mount" -print > "$evidence/corrected-mount-files.txt"
corrected_apps=("$corrected_mount"/*.app)
[ "${#corrected_apps[@]}" -eq 1 ] && [ -d "${corrected_apps[0]}/Contents" ]
corrected_exe="${corrected_apps[0]}/Contents/MacOS/$exe_name"
[ "$(shasum -a 256 "$corrected_exe" | awk '{print $1}')" = "$expected" ]
lipo "$corrected_exe" -verify_arch arm64 x86_64
printf '%s\n' 'Corrected mounted image contains one app with the expected universal executable.' > "$evidence/corrected-layout.txt"
