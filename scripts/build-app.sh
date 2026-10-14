#!/bin/zsh
set -euo pipefail

project_dir="${0:A:h:h}"
output_dir="${QUADCLICKER_OUTPUT_DIR:-$project_dir/dist}"
destination="$output_dir/QuadClicker.app"
signing_identity="${QUADCLICKER_SIGNING_IDENTITY:-}"

if [[ "$(uname -s)" != Darwin ]]; then
    print -u2 'QuadClicker requires macOS.'
    exit 1
fi

# Signing is explicit: silently falling back to ad-hoc can invalidate an existing grant.
if [[ -z "$signing_identity" ]]; then
    print -u2 'Set QUADCLICKER_SIGNING_IDENTITY to your signing certificate, or use scripts/package-release.sh for an ad-hoc release.'
    exit 1
fi
if [[ "$signing_identity" != - ]] && ! /usr/bin/security find-identity -v -p codesigning | /usr/bin/grep -F -q -- "$signing_identity"; then
    print -u2 'The requested signing identity is unavailable in this Mac’s keychain.'
    exit 1
fi

cd "$project_dir"
cargo build --release --locked --target-dir "$project_dir/target"
/usr/bin/plutil -lint "$project_dir/packaging/Info.plist"
binary_dir="$project_dir/target"
if [[ -n "${CARGO_BUILD_TARGET:-}" ]]; then
    binary_dir="$binary_dir/$CARGO_BUILD_TARGET"
fi

# Sign a complete new bundle before replacing the previously built app.
mkdir -p "$output_dir"
staging_dir="$(mktemp -d "$output_dir/.quadclicker.XXXXXX")"
trap 'rm -rf "$staging_dir"' EXIT
bundle="$staging_dir/QuadClicker.app"
mkdir -p "$bundle/Contents/MacOS"
mkdir -p "$bundle/Contents/Resources"
cp "$project_dir/packaging/Info.plist" "$bundle/Contents/Info.plist"
cp "$binary_dir/release/quadclicker" "$bundle/Contents/MacOS/quadclicker"
cp "$project_dir/assets/QuadClicker.icns" "$bundle/Contents/Resources/QuadClicker.icns"
chmod 755 "$bundle/Contents/MacOS/quadclicker"
/usr/bin/codesign --sign "$signing_identity" "$bundle"
/usr/bin/codesign --verify --strict "$bundle"

if [[ -e "$destination" ]]; then
    mv "$destination" "$staging_dir/previous.app"
fi
if ! mv "$bundle" "$destination"; then
    if [[ -e "$staging_dir/previous.app" ]]; then
        mv "$staging_dir/previous.app" "$destination"
    fi
    exit 1
fi

print "Built $destination"
if [[ "$output_dir" == "$project_dir/dist" ]]; then
    print "Install after quitting QuadClicker: ditto '$destination' /Applications/QuadClicker.app"
    print 'Launch: open /Applications/QuadClicker.app'
fi
