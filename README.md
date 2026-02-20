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

Installers validate downloaded release archives against `SHA256SUMS` before extraction.

## Install path defaults

- macOS/Linux: `~/.local/bin/basecamp-cli`
- Windows: `%LOCALAPPDATA%\Programs\basecamp-cli\bin\basecamp-cli.exe`

If needed, add that directory to your `PATH`.
