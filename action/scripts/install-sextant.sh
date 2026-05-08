#!/usr/bin/env bash
# Download a prebuilt `sextant` binary from GitHub Releases, verify its
# checksum, and put it on PATH.
#
# Inputs (env):
#   SEXTANT_VERSION  Tag name (e.g. v0.1.0) or "latest". Required.
#   SEXTANT_REPO     owner/repo to pull from. Default: kylebastien/sextant-mcp.
#   GH_TOKEN         Optional; raises rate limits when running on PRs from
#                    forks where the default token is read-only.
#
# The action sets these. Locally:
#   SEXTANT_VERSION=v0.1.0 ./install-sextant.sh

set -euo pipefail

repo="${SEXTANT_REPO:-kylebastien/sextant-mcp}"
version="${SEXTANT_VERSION:?SEXTANT_VERSION is required}"

case "$(uname -s)" in
  Linux)   os=linux ;;
  Darwin)  os=macos ;;
  MINGW*|MSYS*|CYGWIN*) os=windows ;;
  *) echo "::error::unsupported OS: $(uname -s)" >&2; exit 2 ;;
esac

case "$(uname -m)" in
  x86_64|amd64) arch=x86_64 ;;
  arm64|aarch64) arch=aarch64 ;;
  *) echo "::error::unsupported arch: $(uname -m)" >&2; exit 2 ;;
esac

ext=tar.gz
[ "$os" = "windows" ] && ext=zip

# Resolve "latest" to a concrete tag once so checksum + asset URLs match.
api="https://api.github.com/repos/$repo/releases"
auth=()
if [ -n "${GH_TOKEN:-}" ]; then
  auth=(-H "Authorization: Bearer $GH_TOKEN")
fi

if [ "$version" = "latest" ]; then
  version=$(curl -fsSL "${auth[@]}" "$api/latest" | jq -r .tag_name)
fi

asset="sextant-${version#v}-${os}-${arch}.${ext}"
url="https://github.com/$repo/releases/download/$version/$asset"
sums_url="https://github.com/$repo/releases/download/$version/SHA256SUMS"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "::group::install sextant $version ($os-$arch)"
echo "downloading $url"
curl -fsSL -o "$work/$asset" "$url"
echo "downloading $sums_url"
curl -fsSL -o "$work/SHA256SUMS" "$sums_url"

(
  cd "$work"
  expected=$(grep "  $asset\$" SHA256SUMS | awk '{print $1}')
  if [ -z "$expected" ]; then
    echo "::error::no checksum entry for $asset in SHA256SUMS" >&2
    exit 2
  fi
  actual=$(sha256sum "$asset" | awk '{print $1}')
  if [ "$expected" != "$actual" ]; then
    echo "::error::checksum mismatch for $asset (expected $expected, got $actual)" >&2
    exit 2
  fi
)

dest="${HOME}/.local/bin"
mkdir -p "$dest"
case "$ext" in
  tar.gz) tar -xzf "$work/$asset" -C "$work" ;;
  zip)    unzip -q "$work/$asset" -d "$work" ;;
esac

bin_name=sextant
[ "$os" = "windows" ] && bin_name=sextant.exe
# Some archives wrap the binary in a directory; some don't. Find it.
src=$(find "$work" -maxdepth 3 -name "$bin_name" -type f -print -quit)
if [ -z "$src" ]; then
  echo "::error::could not locate $bin_name inside $asset" >&2
  exit 2
fi
install -m 0755 "$src" "$dest/$bin_name"

echo "$dest" >> "$GITHUB_PATH"
echo "installed: $dest/$bin_name"
"$dest/$bin_name" --version || true
echo "::endgroup::"
