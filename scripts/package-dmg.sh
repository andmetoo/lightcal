#!/bin/sh
# Build an unsigned macOS review disk image.
# This script must not sign, notarize, staple, or publish the image.

set -eu

root=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
skip_build=0

for arg in "$@"; do
  case "$arg" in
    --skip-build) skip_build=1 ;;
    --sign|--codesign|--notarize)
      echo "scripts/package-dmg.sh only builds an unsigned review image." >&2
      exit 2
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  echo "scripts/package-dmg.sh needs macOS hdiutil and iconutil." >&2
  exit 1
fi

version=$(awk '
  /^\[workspace\.package\]/ { in_package = 1; next }
  /^\[/ { in_package = 0 }
  in_package && /^version = "[0-9]+\.[0-9]+\.[0-9]+"/ {
    gsub(/"/, "", $3)
    print $3
    exit
  }
' "$root/Cargo.toml")
if [ -z "$version" ]; then
  echo "Expected a stable workspace version in Cargo.toml." >&2
  exit 1
fi

arch=$(uname -m)
case "$arch" in
  arm64|x86_64) ;;
  *)
    echo "Unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

if [ "$skip_build" -eq 0 ]; then
  (cd "$root" && cargo build --release -p lightcal --locked)
fi

binary="$root/target/release/lightcal"
if [ ! -f "$binary" ] || [ ! -x "$binary" ]; then
  echo "Build the release executable before packaging." >&2
  exit 1
fi

stage=$(mktemp -d "${TMPDIR:-/tmp}/lightcal-dmg.XXXXXX")
trap 'rm -rf "$stage"' EXIT

app="$stage/Lightcal.app"
iconset="$stage/AppIcon.iconset"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$iconset"

cp "$binary" "$app/Contents/MacOS/lightcal"
chmod 755 "$app/Contents/MacOS/lightcal"

copy_icon() {
  src="$root/assets/icons/better/png/icon-$1.png"
  if [ ! -f "$src" ]; then
    echo "Missing icon: $src" >&2
    exit 1
  fi
  cp "$src" "$iconset/$2"
}

copy_icon 16 icon_16x16.png
copy_icon 32 icon_16x16@2x.png
copy_icon 32 icon_32x32.png
copy_icon 64 icon_32x32@2x.png
copy_icon 128 icon_128x128.png
copy_icon 256 icon_128x128@2x.png
copy_icon 256 icon_256x256.png
copy_icon 512 icon_256x256@2x.png
copy_icon 512 icon_512x512.png
copy_icon 1024 icon_512x512@2x.png
iconutil -c icns "$iconset" -o "$app/Contents/Resources/AppIcon.icns"

# Provisional review identity. It is not the frozen release bundle id.
cat > "$app/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>Lightcal</string>
  <key>CFBundleExecutable</key>
  <string>lightcal</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>app.koona.lightcal</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Lightcal</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${version}</string>
  <key>CFBundleVersion</key>
  <string>${version}</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.productivity</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF
plutil -lint "$app/Contents/Info.plist" >/dev/null

for name in LICENSE THIRD_PARTY_NOTICES.md README.md; do
  cp "$root/$name" "$app/Contents/Resources/$name"
  cp "$root/$name" "$stage/$name"
done

cat > "$stage/UNSIGNED.txt" << EOF
Lightcal ${version} unsigned review disk image (${arch})

This disk image and the Lightcal application inside it are not code-signed
and not notarized. macOS Gatekeeper blocks the application until it is
opened explicitly. This file is a CI review artifact. It is not a release
download, and it does not show that the application launches, joins the
menu bar, or reads calendars.

The bundle identifier app.koona.lightcal is provisional. It is not a frozen
release identity. The Info.plist minimum of macOS 11 is not a tested
support matrix.
EOF

mkdir -p "$root/target"
dmg="$root/target/lightcal-${version}-macos-${arch}-unsigned.dmg"
rm -f "$dmg" "${dmg}.sha256"

hdiutil create \
  -volname "Lightcal" \
  -srcfolder "$stage" \
  -ov \
  -format UDZO \
  "$dmg"
hdiutil verify "$dmg" >/dev/null

# /tmp on macOS is /private/tmp, so hdiutil's reported path is not a reliable key.
mnt_parent=$(mktemp -d "${TMPDIR:-/tmp}/lightcal-mnt.XXXXXX")
mount=
cleanup() {
  if [ -n "${mount:-}" ]; then
    hdiutil detach "$mount" >/dev/null 2>&1 || hdiutil detach -force "$mount" >/dev/null 2>&1 || true
    mount=
  fi
  rm -rf "$stage" "$mnt_parent"
}
trap cleanup EXIT
if ! attach_output=$(hdiutil attach -nobrowse -readonly -mountrandom "$mnt_parent" "$dmg" 2>&1); then
  printf '%s\n' "$attach_output" >&2
  echo "Could not attach the disk image." >&2
  exit 1
fi

attempt=0
while [ "$attempt" -lt 10 ]; do
  for candidate in "$mnt_parent"/*; do
    if [ -d "$candidate" ]; then
      mount=$candidate
      break
    fi
  done
  if [ -n "$mount" ]; then
    break
  fi
  attempt=$((attempt + 1))
  sleep 0.2
done
if [ -z "$mount" ]; then
  printf '%s\n' "$attach_output" >&2
  printf '%s\n' "$attach_output" | awk '/^\/dev\// { print $1 }' | while read -r device; do
    hdiutil detach "$device" >/dev/null 2>&1 || true
  done
  echo "Could not mount the disk image." >&2
  exit 1
fi

if [ ! -x "$mount/Lightcal.app/Contents/MacOS/lightcal" ] \
  || [ ! -f "$mount/Lightcal.app/Contents/Resources/AppIcon.icns" ] \
  || [ ! -f "$mount/UNSIGNED.txt" ]; then
  echo "Disk image is missing the unsigned application payload." >&2
  exit 1
fi
if ! grep -q "app.koona.lightcal" "$mount/Lightcal.app/Contents/Info.plist"; then
  echo "Disk image bundle identifier does not match the review plist." >&2
  exit 1
fi
if ! grep -q "<string>${version}</string>" "$mount/Lightcal.app/Contents/Info.plist"; then
  echo "Disk image version does not match Cargo.toml." >&2
  exit 1
fi

cleanup

# A signed image would contradict this review artifact. Do not sign it.
if codesign --verify "$dmg" >/dev/null 2>&1; then
  rm -f "$dmg"
  echo "Refusing to keep a signed disk image." >&2
  exit 1
fi

checksum=$(shasum -a 256 "$dmg" | awk '{ print $1 }')
printf '%s  %s\n' "$checksum" "$(basename "$dmg")" > "${dmg}.sha256"
echo "Created unsigned disk image: $dmg"
echo "No signature, notarization, or publication was performed."
