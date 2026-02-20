#!/usr/bin/env sh
set -eu

BIN_NAME="basecamp-cli"
DEFAULT_REPO="mkv27/basecamp-cli"
VERSION="${BASECAMP_CLI_VERSION:-latest}"
INSTALL_DIR="${BASECAMP_CLI_INSTALL_DIR:-${HOME:-}/.local/bin}"

if [ -z "${INSTALL_DIR}" ]; then
  echo "Error: HOME is not set. Provide BASECAMP_CLI_INSTALL_DIR." >&2
  exit 1
fi

resolve_repo() {
  if [ -n "${BASECAMP_CLI_REPO:-}" ]; then
    printf '%s\n' "${BASECAMP_CLI_REPO}"
    return
  fi

  if [ -n "${GITHUB_REPOSITORY:-}" ]; then
    printf '%s\n' "${GITHUB_REPOSITORY}"
    return
  fi

  if command -v git >/dev/null 2>&1; then
    ORIGIN_URL="$(git config --get remote.origin.url 2>/dev/null || true)"
    if [ -n "${ORIGIN_URL}" ]; then
      REPO_CANDIDATE=""
      case "${ORIGIN_URL}" in
        git@github.com:*)
          REPO_CANDIDATE="${ORIGIN_URL#git@github.com:}"
          ;;
        https://github.com/*)
          REPO_CANDIDATE="${ORIGIN_URL#https://github.com/}"
          ;;
        ssh://git@github.com/*)
          REPO_CANDIDATE="${ORIGIN_URL#ssh://git@github.com/}"
          ;;
      esac

      REPO_CANDIDATE="${REPO_CANDIDATE%.git}"
      case "${REPO_CANDIDATE}" in
        */*)
          printf '%s\n' "${REPO_CANDIDATE}"
          return
          ;;
      esac
    fi
  fi

  printf '%s\n' "${DEFAULT_REPO}"
}

REPO="$(resolve_repo)"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}" in
  Darwin)
    OS_PART="apple-darwin"
    ;;
  Linux)
    OS_PART="unknown-linux-gnu"
    ;;
  *)
    echo "Error: unsupported OS '${OS}'. This installer supports macOS and Linux." >&2
    exit 1
    ;;
esac

case "${ARCH}" in
  x86_64|amd64)
    ARCH_PART="x86_64"
    ;;
  arm64|aarch64)
    ARCH_PART="aarch64"
    ;;
  *)
    echo "Error: unsupported architecture '${ARCH}'." >&2
    exit 1
    ;;
esac

if [ "${OS_PART}" = "unknown-linux-gnu" ] && [ "${ARCH_PART}" = "aarch64" ]; then
  echo "Error: Linux aarch64 binary is not published in Stage 2 yet." >&2
  exit 1
fi

TARGET="${ARCH_PART}-${OS_PART}"
ASSET="${BIN_NAME}-${TARGET}.tar.gz"
CHECKSUMS_ASSET="SHA256SUMS"

if [ "${VERSION}" = "latest" ]; then
  URL_BASE="https://github.com/${REPO}/releases/latest/download"
  RELEASE_API_URL="https://api.github.com/repos/${REPO}/releases/latest"
else
  URL_BASE="https://github.com/${REPO}/releases/download/${VERSION}"
  RELEASE_API_URL="https://api.github.com/repos/${REPO}/releases/tags/${VERSION}"
fi

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t basecamp-cli)"
ARCHIVE_PATH="${TMP_DIR}/${ASSET}"
CHECKSUMS_PATH="${TMP_DIR}/${CHECKSUMS_ASSET}"

cleanup() {
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT INT TERM

download_file() {
  url="$1"
  out="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "${url}" -o "${out}"
    return
  fi

  if command -v wget >/dev/null 2>&1; then
    wget -qO "${out}" "${url}"
    return
  fi

  echo "Error: curl or wget is required to download release assets." >&2
  exit 1
}

download_api_json() {
  url="$1"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL \
      -H "Accept: application/vnd.github+json" \
      -H "X-GitHub-Api-Version: 2022-11-28" \
      "${url}"
    return
  fi

  if command -v wget >/dev/null 2>&1; then
    wget -qO- \
      --header="Accept: application/vnd.github+json" \
      --header="X-GitHub-Api-Version: 2022-11-28" \
      "${url}"
    return
  fi

  return 1
}

sha256_of_file() {
  file="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${file}" | awk '{print $1}'
    return
  fi

  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${file}" | awk '{print $1}'
    return
  fi

  if command -v openssl >/dev/null 2>&1; then
    openssl dgst -sha256 "${file}" | awk '{print $NF}'
    return
  fi

  echo "Error: sha256 tool not found (need sha256sum, shasum, or openssl)." >&2
  exit 1
}

expected_sha256_for_asset() {
  asset="$1"
  checksums_file="$2"
  awk -v f="${asset}" '$2 == f || $2 == ("*" f) { print $1; exit }' "${checksums_file}"
}

expected_sha256_from_release_digest() {
  asset="$1"
  api_url="$2"

  if ! release_json="$(download_api_json "${api_url}" 2>/dev/null)"; then
    return 1
  fi

  digest="$(
    printf '%s' "${release_json}" \
      | tr -d '\n\r\t ' \
      | sed 's/},{/}\n{/g' \
      | awk -v a="${asset}" '
          index($0, "\"name\":\"" a "\"") > 0 {
            if (match($0, /"digest":"sha256:[A-Fa-f0-9]+"/)) {
              digest = substr($0, RSTART, RLENGTH)
              sub(/"digest":"sha256:/, "", digest)
              sub(/"$/, "", digest)
              print tolower(digest)
              exit
            }
          }
        '
  )"

  if [ -z "${digest}" ] || [ "${#digest}" -ne 64 ]; then
    return 1
  fi

  printf '%s\n' "${digest}"
}

ASSET_URL="${URL_BASE}/${ASSET}"
CHECKSUMS_URL="${URL_BASE}/${CHECKSUMS_ASSET}"

echo "Downloading ${ASSET} from ${REPO} (${VERSION})..."
download_file "${ASSET_URL}" "${ARCHIVE_PATH}"
if EXPECTED_SHA256="$(expected_sha256_from_release_digest "${ASSET}" "${RELEASE_API_URL}")"; then
  echo "Using GitHub release asset digest for verification."
else
  echo "GitHub release digest unavailable; downloading ${CHECKSUMS_ASSET}..."
  download_file "${CHECKSUMS_URL}" "${CHECKSUMS_PATH}"
  EXPECTED_SHA256="$(expected_sha256_for_asset "${ASSET}" "${CHECKSUMS_PATH}")"
  if [ -z "${EXPECTED_SHA256}" ]; then
    echo "Error: ${ASSET} checksum not found in ${CHECKSUMS_ASSET}." >&2
    exit 1
  fi
fi

ACTUAL_SHA256="$(sha256_of_file "${ARCHIVE_PATH}")"
if [ "${ACTUAL_SHA256}" != "${EXPECTED_SHA256}" ]; then
  echo "Error: checksum verification failed for ${ASSET}." >&2
  echo "Expected: ${EXPECTED_SHA256}" >&2
  echo "Actual:   ${ACTUAL_SHA256}" >&2
  exit 1
fi

echo "Checksum verified (${ASSET})."

tar -xzf "${ARCHIVE_PATH}" -C "${TMP_DIR}"
BIN_PATH="${TMP_DIR}/${BIN_NAME}-${TARGET}/${BIN_NAME}"

if [ ! -f "${BIN_PATH}" ]; then
  echo "Error: binary not found inside archive (${BIN_PATH})." >&2
  exit 1
fi

mkdir -p "${INSTALL_DIR}"
cp "${BIN_PATH}" "${INSTALL_DIR}/${BIN_NAME}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

echo "Installed ${BIN_NAME} to ${INSTALL_DIR}/${BIN_NAME}"
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo "Add ${INSTALL_DIR} to PATH to run '${BIN_NAME}' from anywhere."
    ;;
esac
