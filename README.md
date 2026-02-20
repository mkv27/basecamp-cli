# basecamp-cli

Small Basecamp CLI written in Rust.

## Install

### macOS / Linux (latest)

```bash
sh -c "$(curl -fsSL https://raw.githubusercontent.com/mkv27/basecamp-cli/main/install.sh)"
```

### Windows PowerShell (latest)

```powershell
irm https://raw.githubusercontent.com/mkv27/basecamp-cli/main/install.ps1 | iex
```

## Advanced Install Options

### Install a specific version (macOS / Linux)

```bash
BASECAMP_CLI_VERSION=v0.1.0 \
sh -c "$(curl -fsSL https://raw.githubusercontent.com/mkv27/basecamp-cli/main/install.sh)"
```

### Install a specific version (Windows PowerShell)

```powershell
$env:BASECAMP_CLI_VERSION = "v0.1.0"
irm https://raw.githubusercontent.com/mkv27/basecamp-cli/main/install.ps1 | iex
```

### Install from a custom fork/repo

```bash
BASECAMP_CLI_REPO=your-org/your-repo \
sh -c "$(curl -fsSL https://raw.githubusercontent.com/mkv27/basecamp-cli/main/install.sh)"
```

```powershell
$env:BASECAMP_CLI_REPO = "your-org/your-repo"
irm https://raw.githubusercontent.com/mkv27/basecamp-cli/main/install.ps1 | iex
```

## Verify

```bash
basecamp-cli --help
```

Installers validate downloaded release archives against GitHub release asset digests (`sha256`) before extraction, with `SHA256SUMS` as a fallback.

## Install path defaults

- macOS/Linux: `~/.local/bin/basecamp-cli`
- Windows: `%LOCALAPPDATA%\Programs\basecamp-cli\bin\basecamp-cli.exe`

If needed, add that directory to your `PATH`.

## Release

Automated:

```bash
scripts/release.sh 0.2.0
```

Manual:

```bash
# 1) Update package version
# edit Cargo.toml -> [package].version

# 2) Keep lock file in sync
cargo check --locked

# 3) Commit + tag + push
git add Cargo.toml Cargo.lock
git commit -m "release: v0.2.0"
git tag v0.2.0
git push origin HEAD
git push origin v0.2.0
```
