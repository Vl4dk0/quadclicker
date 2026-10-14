#!/bin/zsh
set -euo pipefail

project_dir="${0:A:h:h}"
destination="$project_dir/dist/QuadClicker.app"

if [[ "$(uname -s)" != Darwin ]]; then
    print -u2 'QuadClicker requires macOS.'
    exit 1
fi

cd "$project_dir"
cargo build --release --locked --target-dir "$project_dir/target"
/usr/bin/plutil -lint "$project_dir/packaging/Info.plist"

# Sign a complete new bundle before replacing the previously built app.
mkdir -p "$project_dir/dist"
staging_dir="$(mktemp -d "$project_dir/dist/.quadclicker.XXXXXX")"
trap 'rm -rf "$staging_dir"' EXIT
bundle="$staging_dir/QuadClicker.app"
mkdir -p "$bundle/Contents/MacOS"
cp "$project_dir/packaging/Info.plist" "$bundle/Contents/Info.plist"
cp "$project_dir/target/release/quadclicker" "$bundle/Contents/MacOS/quadclicker"
chmod 755 "$bundle/Contents/MacOS/quadclicker"
/usr/bin/codesign --sign - "$bundle"
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
print "Install after quitting QuadClicker: ditto '$destination' /Applications/QuadClicker.app"
print 'Launch: open /Applications/QuadClicker.app'
