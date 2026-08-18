#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || "$1" == "-h" || "$1" == "--help" ]]; then
  echo "usage: $0 PATH_TO_REMTR_TAR_GZ"
  exit "$( [[ $# -eq 1 ]] && echo 0 || echo 2 )"
fi

archive="$1"
[[ -f "$archive" ]] || { echo "archive not found: $archive" >&2; exit 1; }
archive_dir="$(cd "$(dirname "$archive")" && pwd)"
archive_name="$(basename "$archive")"
checksum="$archive_dir/$archive_name.sha256"

if [[ -f "$checksum" ]]; then
  (cd "$archive_dir" && shasum -a 256 -c "$(basename "$checksum")")
else
  echo "warning: checksum file not found: $checksum" >&2
fi

package_name="${archive_name%.tar.gz}"
case "$package_name" in
  remtr-*-aarch64-apple-darwin) ;;
  *) echo "unexpected archive name: $archive_name" >&2; exit 1 ;;
esac

verify_root="$(mktemp -d "${TMPDIR:-/tmp}/remtr-verify.XXXXXX")"
trap 'rm -rf "$verify_root"' EXIT
tar -xzf "$archive" -C "$verify_root"
package_root="$verify_root/$package_name"

for required in remtr install.sh README.md LICENSE CHANGELOG.md docs/USER_GUIDE.md docs/USER_GUIDE.zh-CN.md docs/ARCHITECTURE.md docs/DEVELOPMENT.md docs/RELEASE.md; do
  [[ -f "$package_root/$required" ]] || { echo "missing package file: $required" >&2; exit 1; }
done
[[ -x "$package_root/remtr" ]] || { echo "remtr is not executable" >&2; exit 1; }
file "$package_root/remtr" | grep -q 'arm64' || { echo "remtr is not an arm64 binary" >&2; exit 1; }
if [[ "${REMTR_REQUIRE_SIGNED:-0}" == "1" ]]; then
  codesign --verify --strict --verbose=2 "$package_root/remtr"
fi
"$package_root/remtr" --version | grep -q '^remtr '
"$package_root/remtr" --help | grep -q '^ReMTR '

echo "verified $archive"
