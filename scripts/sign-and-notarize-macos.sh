#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || "$1" == "-h" || "$1" == "--help" ]]; then
  echo "usage: $0 PATH_TO_REMTR_BINARY"
  exit "$( [[ $# -eq 1 ]] && echo 0 || echo 2 )"
fi

binary="$1"
[[ -f "$binary" ]] || { echo "binary not found: $binary" >&2; exit 1; }
[[ "$(uname -s)" == "Darwin" ]] || { echo "signing requires macOS" >&2; exit 1; }

required_variables=(
  MACOS_CERTIFICATE_P12_BASE64
  MACOS_CERTIFICATE_PASSWORD
  APPLE_API_KEY_P8_BASE64
  APPLE_API_KEY_ID
  APPLE_API_ISSUER_ID
  APPLE_TEAM_ID
)
for variable in "${required_variables[@]}"; do
  [[ -n "${!variable:-}" ]] || { echo "missing required environment variable: $variable" >&2; exit 1; }
done

work_parent="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
work_dir="$(mktemp -d "$work_parent/remtr-signing.XXXXXX")"
keychain_path="$work_dir/remtr-signing.keychain-db"
certificate_path="$work_dir/developer-id.p12"
api_key_path="$work_dir/AuthKey_${APPLE_API_KEY_ID}.p8"
notary_archive="$work_dir/remtr-notary.zip"
keychain_password="$(openssl rand -hex 24)"

cleanup() {
  security delete-keychain "$keychain_path" >/dev/null 2>&1 || true
  rm -rf "$work_dir"
}
trap cleanup EXIT

printf '%s' "$MACOS_CERTIFICATE_P12_BASE64" | /usr/bin/base64 -D > "$certificate_path"
printf '%s' "$APPLE_API_KEY_P8_BASE64" | /usr/bin/base64 -D > "$api_key_path"
chmod 600 "$certificate_path" "$api_key_path"

security create-keychain -p "$keychain_password" "$keychain_path"
security set-keychain-settings -lut 21600 "$keychain_path"
security unlock-keychain -p "$keychain_password" "$keychain_path"
security import "$certificate_path" -k "$keychain_path" -P "$MACOS_CERTIFICATE_PASSWORD" -f pkcs12 -A
security set-key-partition-list -S apple-tool:,apple: -s -k "$keychain_password" "$keychain_path" >/dev/null
security list-keychains -d user -s "$keychain_path"

signing_identity="$(security find-identity -v -p codesigning "$keychain_path" | awk -F '"' '/Developer ID Application/ { print $2; exit }')"
[[ -n "$signing_identity" ]] || { echo "Developer ID Application identity not found" >&2; exit 1; }

codesign --force --options runtime --timestamp --sign "$signing_identity" "$binary"
codesign --verify --strict --verbose=2 "$binary"

team_identifier="$(codesign -d --verbose=4 "$binary" 2>&1 | awk -F '=' '/^TeamIdentifier=/ { print $2; exit }')"
[[ "$team_identifier" == "$APPLE_TEAM_ID" ]] || { echo "signed binary Team ID does not match APPLE_TEAM_ID" >&2; exit 1; }

ditto -c -k --keepParent "$binary" "$notary_archive"
xcrun notarytool submit "$notary_archive" \
  --key "$api_key_path" \
  --key-id "$APPLE_API_KEY_ID" \
  --issuer "$APPLE_API_ISSUER_ID" \
  --wait

# `spctl --assess --type execute` only assesses app bundles. For a standalone
# CLI, strict codesign verification plus notarytool's Accepted result are the
# applicable release gates.
codesign --verify --strict --verbose=2 "$binary"
echo "signed and notarized $binary"
