# Code Plan (Distribution Product Feature)

This is a product/distribution feature, not a CLI command feature.

## Goal

Ship installable binaries through GitHub only, with no custom server.

Users install through scripts hosted in the repo:

- macOS/Linux: `install.sh`
- Windows: `install.ps1`

## Release Trigger

GitHub Actions workflow runs on pushed tags matching `v*.*.*`.

The workflow also validates strict SemVer tag format:

- required: `vX.Y.Z`
- examples: `v0.1.0`, `v2.4.12`

If the tag is not strict SemVer, the job fails early.

## Workflow Tooling Choices

Use official GitHub actions for artifact flow:

- `actions/checkout@v6`
- `actions/upload-artifact@v6`
- `actions/download-artifact@v7`

Release publishing:

- use GitHub CLI (`gh release create/upload`) with `GITHUB_TOKEN`
- avoid third-party release action wrappers in Stage 2

Rust toolchain setup:

- use `rustup` commands directly in workflow steps
- avoid `@master` action refs for Rust setup

## Build Matrix (Stage 2)

Publish these targets:

- `x86_64-unknown-linux-gnu` (Linux x86_64)
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS Apple Silicon)
- `x86_64-pc-windows-msvc` (Windows x86_64)

## Release Assets

Asset naming convention:

- non-Windows: `basecamp-cli-<target>.tar.gz`
- Windows: `basecamp-cli-<target>.zip`

Also publish:

- `SHA256SUMS` (compatibility fallback for installer verification)

## Installer Contract

`install.sh`:

- detects OS/arch
- resolves target triple
- downloads from GitHub Releases (`latest` or explicit version)
- verifies SHA-256 via GitHub release asset `digest` API field
- falls back to `SHA256SUMS` from the same release if digest lookup is unavailable
- extracts binary and installs to `${BASECAMP_CLI_INSTALL_DIR:-$HOME/.local/bin}`

`install.ps1`:

- targets Windows x86_64
- downloads from GitHub Releases (`latest` or explicit version)
- verifies SHA-256 via GitHub release asset `digest` API field
- falls back to `SHA256SUMS` from the same release if digest lookup is unavailable
- extracts binary and installs to `%LOCALAPPDATA%\Programs\basecamp-cli\bin` by default

Common env overrides:

- `BASECAMP_CLI_REPO` (optional override when auto-detection is not available)
- `BASECAMP_CLI_VERSION` (default `latest`)
- `BASECAMP_CLI_INSTALL_DIR` (custom install destination)

Repository source resolution:

- `BASECAMP_CLI_REPO` if provided
- otherwise `GITHUB_REPOSITORY` when present
- otherwise parse local `git remote origin`
- otherwise fallback to default repo: `mkv27/basecamp-cli`

## Why This Makes Sense

Yes, this approach is a standard and viable distribution model:

- binaries are produced and signed-off by CI
- assets are stored in GitHub Releases
- installers fetch directly from GitHub-hosted URLs
- no dedicated backend infrastructure is required

## Notes

- Linux `aarch64` is not included in Stage 2 matrix yet.
- If needed later, add `aarch64-unknown-linux-gnu` through an ARM runner or cross toolchain.
