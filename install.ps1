param(
  [string]$Version = $(if ($env:BASECAMP_CLI_VERSION) { $env:BASECAMP_CLI_VERSION } else { "latest" }),
  [string]$InstallDir = $(if ($env:BASECAMP_CLI_INSTALL_DIR) { $env:BASECAMP_CLI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\basecamp-cli\bin" })
)

$ErrorActionPreference = "Stop"

$BinName = "basecamp-cli"
$DefaultRepo = "mkv27/basecamp-cli"
$Target = "x86_64-pc-windows-msvc"
$Asset = "$BinName-$Target.zip"
$ChecksumsAsset = "SHA256SUMS"

function Resolve-Repo {
  if ($env:BASECAMP_CLI_REPO) {
    return $env:BASECAMP_CLI_REPO
  }

  if ($env:GITHUB_REPOSITORY) {
    return $env:GITHUB_REPOSITORY
  }

  if (Get-Command git -ErrorAction SilentlyContinue) {
    $origin = (git config --get remote.origin.url 2>$null)
    if ($origin) {
      $origin = $origin.Trim()

      if ($origin -match '^git@github\.com:(?<repo>[^/]+/[^/]+?)(\.git)?$') {
        return $Matches.repo
      }
      if ($origin -match '^https://github\.com/(?<repo>[^/]+/[^/]+?)(\.git)?$') {
        return $Matches.repo
      }
      if ($origin -match '^ssh://git@github\.com/(?<repo>[^/]+/[^/]+?)(\.git)?$') {
        return $Matches.repo
      }
    }
  }

  return $DefaultRepo
}

function Get-ExpectedAssetHashFromReleaseDigest {
  param(
    [string]$RepoName,
    [string]$ReleaseVersion,
    [string]$AssetName
  )

  $releaseApiUrl = if ($ReleaseVersion -eq "latest") {
    "https://api.github.com/repos/$RepoName/releases/latest"
  } else {
    "https://api.github.com/repos/$RepoName/releases/tags/$ReleaseVersion"
  }

  try {
    $release = Invoke-RestMethod -Uri $releaseApiUrl -Headers @{
      Accept = "application/vnd.github+json"
      "X-GitHub-Api-Version" = "2022-11-28"
    }
  }
  catch {
    return $null
  }

  $asset = $release.assets | Where-Object { $_.name -eq $AssetName } | Select-Object -First 1
  if (-not $asset -or -not $asset.digest) {
    return $null
  }

  if ($asset.digest -match '^sha256:(?<hash>[A-Fa-f0-9]{64})$') {
    return $Matches.hash.ToLowerInvariant()
  }

  return $null
}

$Repo = Resolve-Repo

if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64" -and $env:PROCESSOR_ARCHITEW6432 -ne "AMD64") {
  throw "Unsupported Windows architecture. Stage 2 publishes only x86_64 Windows binaries."
}

if ($Version -eq "latest") {
  $UrlBase = "https://github.com/$Repo/releases/latest/download"
} else {
  $UrlBase = "https://github.com/$Repo/releases/download/$Version"
}

$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("basecamp-cli-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $TmpDir | Out-Null

try {
  $ArchivePath = Join-Path $TmpDir $Asset
  $ChecksumsPath = Join-Path $TmpDir $ChecksumsAsset
  $Url = "$UrlBase/$Asset"
  $ChecksumsUrl = "$UrlBase/$ChecksumsAsset"

  Write-Host "Downloading $Asset from $Repo ($Version)..."
  Invoke-WebRequest -Uri $Url -OutFile $ArchivePath
  $expectedHash = Get-ExpectedAssetHashFromReleaseDigest -RepoName $Repo -ReleaseVersion $Version -AssetName $Asset
  if ($expectedHash) {
    Write-Host "Using GitHub release asset digest for verification."
  } else {
    Write-Host "GitHub release digest unavailable; downloading $ChecksumsAsset..."
    Invoke-WebRequest -Uri $ChecksumsUrl -OutFile $ChecksumsPath

    Get-Content $ChecksumsPath | ForEach-Object {
      if ($_ -match '^(?<hash>[A-Fa-f0-9]{64})\s+\*?(?<name>.+)$') {
        $name = $Matches.name.Trim()
        if ($name -eq $Asset -and -not $expectedHash) {
          $expectedHash = $Matches.hash.ToLowerInvariant()
        }
      }
    }

    if (-not $expectedHash) {
      throw "Checksum for $Asset was not found in $ChecksumsAsset."
    }
  }

  $actualHash = (Get-FileHash -Algorithm SHA256 -Path $ArchivePath).Hash.ToLowerInvariant()
  if ($actualHash -ne $expectedHash) {
    throw "Checksum verification failed for $Asset. Expected $expectedHash, got $actualHash."
  }

  Write-Host "Checksum verified ($Asset)."

  Expand-Archive -Path $ArchivePath -DestinationPath $TmpDir -Force

  $BinaryPath = Join-Path $TmpDir "$BinName-$Target\$BinName.exe"
  if (-not (Test-Path $BinaryPath)) {
    throw "Binary not found inside archive ($BinaryPath)."
  }

  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  Copy-Item $BinaryPath (Join-Path $InstallDir "$BinName.exe") -Force

  Write-Host "Installed $BinName to $(Join-Path $InstallDir "$BinName.exe")"
  Write-Host "If needed, add '$InstallDir' to your PATH."
}
finally {
  Remove-Item -Recurse -Force $TmpDir -ErrorAction SilentlyContinue
}
