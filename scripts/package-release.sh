#!/usr/bin/env bash
set -euo pipefail

target="${1:?target is required}"
version="${2:?version is required}"
binary_name="hash-killer"
root_dir="$(pwd)"
dist_dir="${root_dir}/dist/release"
staging_dir="${root_dir}/target/package/${target}"

rm -rf "${staging_dir}"
mkdir -p "${staging_dir}" "${dist_dir}"

case "${target}" in
  aarch64-apple-darwin)
    app_dir="${staging_dir}/Hash Killer.app"
    contents_dir="${app_dir}/Contents"
    macos_dir="${contents_dir}/MacOS"
    resources_dir="${contents_dir}/Resources"
    mkdir -p "${macos_dir}" "${resources_dir}"
    cp "${root_dir}/target/${target}/release/${binary_name}" "${macos_dir}/Hash Killer"
    cp resources/hashkiller.icns "${resources_dir}/hashkiller.icns"
    cat > "${contents_dir}/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDisplayName</key>
  <string>Hash Killer</string>
  <key>CFBundleExecutable</key>
  <string>Hash Killer</string>
  <key>CFBundleIconFile</key>
  <string>hashkiller</string>
  <key>CFBundleIdentifier</key>
  <string>dev.yldst.hash-killer</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>Hash Killer</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${version}</string>
  <key>CFBundleVersion</key>
  <string>${version}</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST
    chmod +x "${macos_dir}/Hash Killer"
    if command -v codesign >/dev/null 2>&1; then
      codesign --force --deep --sign - "${app_dir}" >/dev/null 2>&1 || true
    fi
    (cd "${staging_dir}" && zip -qry "${dist_dir}/hash-killer-${version}-macos-arm64.zip" "Hash Killer.app")
    ;;
  x86_64-pc-windows-msvc)
    cp "${root_dir}/target/${target}/release/${binary_name}.exe" "${staging_dir}/Hash Killer.exe"
    (cd "${staging_dir}" && 7z a "${dist_dir}/hash-killer-${version}-windows-x64.zip" "Hash Killer.exe" >/dev/null)
    ;;
  x86_64-unknown-linux-gnu)
    install -Dm755 "${root_dir}/target/${target}/release/${binary_name}" "${staging_dir}/hash-killer"
    install -Dm644 resources/hashkiller.png "${staging_dir}/share/icons/hicolor/512x512/apps/hash-killer.png"
    cat > "${staging_dir}/hash-killer.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Hash Killer
Exec=hash-killer
Icon=hash-killer
Categories=Utility;
Terminal=false
DESKTOP
    tar -C "${staging_dir}" -czf "${dist_dir}/hash-killer-${version}-linux-x64.tar.gz" .
    ;;
  *)
    echo "unsupported target: ${target}" >&2
    exit 1
    ;;
esac

if command -v shasum >/dev/null 2>&1; then
  (cd "${dist_dir}" && shasum -a 256 hash-killer-${version}-* > "hash-killer-${version}-checksums.txt")
elif command -v sha256sum >/dev/null 2>&1; then
  (cd "${dist_dir}" && sha256sum hash-killer-${version}-* > "hash-killer-${version}-checksums.txt")
fi
