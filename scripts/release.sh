#!/usr/bin/env sh
set -eu

usage() {
  cat <<'EOF'
Usage:
  scripts/release.sh <version>

Example:
  scripts/release.sh 0.2.0

What it does:
  1) Updates Cargo.toml package version
  2) Updates root package version in Cargo.lock
  3) Runs cargo check --locked --offline
  4) Creates commit "release: v<version>"
  5) Creates tag "v<version>"
  6) Pushes commit and tag to origin
EOF
}

VERSION="${1:-}"
if [ -z "${VERSION}" ]; then
  usage
  exit 2
fi

case "${VERSION}" in
  *[!0-9.]* | *.*.*.* | .* | *.)
    echo "Error: version must be strict semver core format X.Y.Z (example: 0.2.0)." >&2
    exit 2
    ;;
esac

if ! printf '%s\n' "${VERSION}" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: version must be strict semver core format X.Y.Z (example: 0.2.0)." >&2
  exit 2
fi

TAG="v${VERSION}"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "Error: this command must run inside a git repository." >&2
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "Error: working tree is not clean. Commit or stash changes first." >&2
  exit 1
fi

if git rev-parse -q --verify "refs/tags/${TAG}" >/dev/null 2>&1; then
  echo "Error: tag ${TAG} already exists." >&2
  exit 1
fi

VERSION="${VERSION}" perl -0777 -i -pe 's/(\[package\]\n(?:.*?\n)*?version = )"[0-9]+\.[0-9]+\.[0-9]+"/$1 . q{"} . $ENV{VERSION} . q{"}/se' Cargo.toml
VERSION="${VERSION}" perl -0777 -i -pe 's/(\[\[package\]\]\nname = "basecamp-cli"\nversion = )"[^"]+"/$1 . q{"} . $ENV{VERSION} . q{"}/se' Cargo.lock

TOML_VERSION="$(perl -0777 -ne 'if(/\[package\]\n(?:.*?\n)*?version = "([^"]+)"/s){print $1}' Cargo.toml)"
LOCK_VERSION="$(perl -0777 -ne 'if(/\[\[package\]\]\nname = "basecamp-cli"\nversion = "([^"]+)"/s){print $1}' Cargo.lock)"

if [ "${TOML_VERSION}" != "${VERSION}" ] || [ "${LOCK_VERSION}" != "${VERSION}" ]; then
  echo "Error: failed to set version consistently (Cargo.toml=${TOML_VERSION}, Cargo.lock=${LOCK_VERSION})." >&2
  exit 1
fi

cargo check --locked --offline

git add Cargo.toml Cargo.lock
git commit -m "release: ${TAG}"
git tag "${TAG}"
git push origin HEAD
git push origin "${TAG}"

echo "Release prepared and pushed: ${TAG}"
