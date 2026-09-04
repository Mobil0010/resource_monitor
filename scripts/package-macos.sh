#!/usr/bin/env bash
set -euo pipefail

version="${1:-0.1.0}"
root_dir="$(cd "$(dirname "$0")/.." && pwd)"
dist_dir="$root_dir/dist"
app_dir="$dist_dir/Resource Monitor.app"
dmg_root="$dist_dir/dmg-root"
dmg_path="$dist_dir/ResourceMonitor-${version}-macOS-Universal.dmg"

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --manifest-path "$root_dir/Cargo.toml" --release --target aarch64-apple-darwin
cargo build --manifest-path "$root_dir/Cargo.toml" --release --target x86_64-apple-darwin

rm -rf "$app_dir" "$dmg_root" "$dmg_path"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources" "$dmg_root"

lipo -create \
  "$root_dir/target/aarch64-apple-darwin/release/resource_monitor" \
  "$root_dir/target/x86_64-apple-darwin/release/resource_monitor" \
  -output "$app_dir/Contents/MacOS/ResourceMonitor"
chmod +x "$app_dir/Contents/MacOS/ResourceMonitor"
sed "s/__VERSION__/$version/g" "$root_dir/packaging/macos/Info.plist" > "$app_dir/Contents/Info.plist"

signing_identity="${APPLE_SIGNING_IDENTITY:--}"
if [[ "$signing_identity" == "-" ]]; then
  # 서명 인증서가 없는 공개 CI에서도 실행할 수 있도록 ad-hoc 서명을 적용합니다.
  codesign --force --deep --sign - "$app_dir"
else
  codesign --force --deep --options runtime --timestamp --sign "$signing_identity" "$app_dir"
fi

ditto "$app_dir" "$dmg_root/Resource Monitor.app"
ln -s /Applications "$dmg_root/Applications"
hdiutil create \
  -volname "Resource Monitor" \
  -srcfolder "$dmg_root" \
  -ov \
  -format UDZO \
  "$dmg_path"

if [[ -n "${APPLE_ID:-}" && -n "${APPLE_TEAM_ID:-}" && -n "${APPLE_APP_PASSWORD:-}" ]]; then
  xcrun notarytool submit "$dmg_path" \
    --apple-id "$APPLE_ID" \
    --team-id "$APPLE_TEAM_ID" \
    --password "$APPLE_APP_PASSWORD" \
    --wait
  xcrun stapler staple "$dmg_path"
fi

echo "$dmg_path"
