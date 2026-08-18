#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
output_dir="$repo_root/dist"
binary_path=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output-dir)
      [[ $# -ge 2 ]] || { echo "missing value for --output-dir" >&2; exit 2; }
      output_dir="$2"
      shift 2
      ;;
    --binary)
      [[ $# -ge 2 ]] || { echo "missing value for --binary" >&2; exit 2; }
      binary_path="$2"
      shift 2
      ;;
    -h|--help)
      echo "usage: $0 [--output-dir DIR] [--binary PATH]"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

[[ "$(uname -s)" == "Darwin" ]] || { echo "packaging requires macOS" >&2; exit 1; }
[[ "$(uname -m)" == "arm64" ]] || { echo "packaging requires native Apple Silicon arm64" >&2; exit 1; }

version="$(awk -F '"' '/^version = "/ { print $2; exit }' "$repo_root/Cargo.toml")"
[[ -n "$version" ]] || { echo "unable to read package version" >&2; exit 1; }

package_name="remtr-${version}-aarch64-apple-darwin"
archive_name="${package_name}.tar.gz"
mkdir -p "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"

staging_root="$(mktemp -d "${TMPDIR:-/tmp}/remtr-package.XXXXXX")"
trap 'rm -rf "$staging_root"' EXIT
package_root="$staging_root/$package_name"
mkdir -p "$package_root/docs"

cd "$repo_root"
if [[ -z "$binary_path" ]]; then
  export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"
  cargo build --release --locked
  binary_path="$repo_root/target/release/remtr"
elif [[ "$binary_path" != /* ]]; then
  binary_path="$repo_root/$binary_path"
fi

[[ -f "$binary_path" ]] || { echo "binary not found: $binary_path" >&2; exit 1; }
file "$binary_path" | grep -q 'arm64' || { echo "release binary is not arm64" >&2; exit 1; }

install -m 0755 "$binary_path" "$package_root/remtr"
install -m 0755 scripts/install.sh "$package_root/install.sh"
install -m 0644 README.md LICENSE CHANGELOG.md "$package_root/"
install -m 0644 docs/*.md "$package_root/docs/"

COPYFILE_DISABLE=1 tar -C "$staging_root" -czf "$output_dir/$archive_name" "$package_name"
(
  cd "$output_dir"
  shasum -a 256 "$archive_name" > "$archive_name.sha256"
)

echo "created $output_dir/$archive_name"
echo "created $output_dir/$archive_name.sha256"
