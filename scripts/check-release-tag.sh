#!/usr/bin/env bash
set -euo pipefail

tag_name="${1:-${GITHUB_REF_NAME:-}}"

if [[ -z "$tag_name" ]]; then
  echo "Release tag is required." >&2
  exit 1
fi

if [[ ! "$tag_name" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]]; then
  echo "Release tag must look like vX.Y.Z, optionally with a prerelease/build suffix: $tag_name" >&2
  exit 1
fi

if ! git show-ref --verify --quiet "refs/tags/$tag_name"; then
  echo "Release tag does not exist locally: $tag_name" >&2
  exit 1
fi

if ! git rev-parse --verify --quiet "refs/tags/$tag_name^{commit}" >/dev/null; then
  echo "Release tag does not resolve to a commit: $tag_name" >&2
  exit 1
fi

command -v node >/dev/null 2>&1 || { echo "node is required to check package version" >&2; exit 1; }

package_version="$(
  node -e 'const fs = require("node:fs"); const pkg = JSON.parse(fs.readFileSync("package.json", "utf8")); process.stdout.write(pkg.version);'
)"
expected_tag="v${package_version}"

if [[ "$tag_name" != "$expected_tag" ]]; then
  echo "Release tag $tag_name does not match package.json version $package_version; expected $expected_tag." >&2
  exit 1
fi

echo "Release tag preflight passed: $tag_name"
