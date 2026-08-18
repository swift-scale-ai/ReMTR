#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
prefix="${PREFIX:-/usr/local}"
destdir="${DESTDIR:-}"
destination="$destdir$prefix/bin"
binary="$script_dir/remtr"

[[ "$(uname -s)" == "Darwin" && "$(uname -m)" == "arm64" ]] || {
  echo "ReMTR supports native Apple Silicon macOS only" >&2
  exit 1
}
[[ -x "$binary" ]] || {
  echo "remtr binary not found next to install.sh" >&2
  echo "run this installer from an extracted ReMTR release archive" >&2
  exit 1
}

mkdir -p "$destination" || {
  echo "cannot create $destination; choose a writable PREFIX or rerun with appropriate privileges" >&2
  exit 1
}
install -m 0755 "$binary" "$destination/remtr"
echo "installed $destination/remtr"
