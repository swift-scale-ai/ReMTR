# Release and Packaging

ReMTR publishes one target: `aarch64-apple-darwin`.

## Local package

```bash
./scripts/package-macos.sh
./scripts/verify-package.sh dist/remtr-0.1.0-aarch64-apple-darwin.tar.gz
```

The archive contains the arm64 executable, installer, checksum-compatible metadata, English documentation, and Chinese user guide. Packaging a prebuilt signed binary avoids rebuilding after code signing:

```bash
./scripts/package-macos.sh --binary target/release/remtr
```

## CI artifacts

Every successful CI run uploads the archive and SHA-256 file for 14 days. Pushes to trusted `main` and manual runs perform a privileged probe only against `127.0.0.1`; pull requests never execute contributed code with `sudo`, and CI never probes an internet target.

## GitHub release

1. Ensure the version in `Cargo.toml` is correct.
2. Ensure CI passes on the release commit.
3. Push a matching annotated tag such as `v0.1.0`.

The Release workflow builds on native arm64 macOS, tests a loopback probe, signs with a Developer ID Application certificate, waits for Apple notarization to return `Accepted`, verifies the signed archive, and publishes both files to GitHub Releases.

Required GitHub Actions repository secrets:

- `MACOS_CERTIFICATE_P12_BASE64`
- `MACOS_CERTIFICATE_PASSWORD`
- `APPLE_API_KEY_P8_BASE64`
- `APPLE_API_KEY_ID`
- `APPLE_API_ISSUER_ID`
- `APPLE_TEAM_ID`

Never commit certificates, private keys, passwords, or API credentials.
