#!/bin/zsh
set -euo pipefail

project_dir="${0:A:h:h}"
if [[ "$(uname -s)" != Darwin ]]; then
    print -u2 'QuadClicker release packaging requires macOS.'
    exit 1
fi

version="$(/usr/bin/plutil -extract CFBundleShortVersionString raw "$project_dir/packaging/Info.plist")"
output_dir="$project_dir/target/releases/v$version"
archive_name="QuadClicker-$version-macos-arm64.zip"
mkdir -p "$output_dir"
staging_dir="$(mktemp -d "$output_dir/.package.XXXXXX")"
trap 'rm -rf "$staging_dir"' EXIT

# Public artifacts use ad-hoc signing without replacing the locally signed app.
QUADCLICKER_SIGNING_IDENTITY=- \
QUADCLICKER_OUTPUT_DIR="$staging_dir" \
CARGO_BUILD_TARGET=aarch64-apple-darwin \
MACOSX_DEPLOYMENT_TARGET=13.0 \
    "$project_dir/scripts/build-app.sh"

bundle="$staging_dir/QuadClicker.app"
[[ "$(/usr/bin/lipo -archs "$bundle/Contents/MacOS/quadclicker")" == arm64 ]]
/usr/bin/codesign --verify --strict "$bundle"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$bundle" "$staging_dir/$archive_name"

# Check what a downloader extracts, including executable permissions and signature.
/usr/bin/ditto -x -k "$staging_dir/$archive_name" "$staging_dir/extracted"
extracted_bundle="$staging_dir/extracted/QuadClicker.app"
[[ -x "$extracted_bundle/Contents/MacOS/quadclicker" ]]
/usr/bin/codesign --verify --strict "$extracted_bundle"
/usr/bin/cmp "$bundle/Contents/Info.plist" "$extracted_bundle/Contents/Info.plist"
/usr/bin/cmp "$bundle/Contents/MacOS/quadclicker" "$extracted_bundle/Contents/MacOS/quadclicker"
/usr/bin/cmp "$bundle/Contents/Resources/QuadClicker.icns" "$extracted_bundle/Contents/Resources/QuadClicker.icns"

mv "$staging_dir/$archive_name" "$output_dir/$archive_name"
(
    cd "$output_dir"
    /usr/bin/shasum -a 256 "$archive_name" > "$archive_name.sha256"
)
print "Release archive: $output_dir/$archive_name"
print "Checksum: $output_dir/$archive_name.sha256"
print 'This ad-hoc signed archive is not notarized by Apple.'
