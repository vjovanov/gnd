#!/usr/bin/env bash
set -euo pipefail

# Pre-release package-name guard for §RM-distribution-naming.
# Claimed future package names must be either available or already owned by this
# repository. The historical collisions for bare "gnd" on npm/PyPI are reported
# as notices so the release manager can revisit the docs if those names change.

ua="gnd-release-name-check/0.1"
repo_pattern='github.com[/:]vjovanov/gnd'
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

http_get() {
  local url="$1"
  local out="$2"
  curl -sS -L -A "$ua" -o "$out" -w '%{http_code}' "$url"
}

metadata_mentions_repo() {
  local file="$1"
  python3 - "$file" "$repo_pattern" <<'PY'
import json
import re
import sys

path, pattern = sys.argv[1], sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

haystack = json.dumps(data, sort_keys=True).lower()
sys.exit(0 if re.search(pattern, haystack) else 1)
PY
}

check_claimed_json_name() {
  local registry="$1"
  local name="$2"
  local url="$3"
  local out="$tmpdir/${registry}-${name}.json"
  local code

  code="$(http_get "$url" "$out")"
  case "$code" in
    200)
      if metadata_mentions_repo "$out"; then
        echo "ok: $registry/$name is owned by this project"
      else
        echo "error: $registry/$name is already taken by another project" >&2
        echo "       $url" >&2
        return 1
      fi
      ;;
    404)
      echo "ok: $registry/$name is available"
      ;;
    *)
      echo "error: could not query $registry/$name (HTTP $code)" >&2
      echo "       $url" >&2
      return 1
      ;;
  esac
}

notice_external_json_name() {
  local registry="$1"
  local name="$2"
  local url="$3"
  local out="$tmpdir/${registry}-${name}-external.json"
  local code

  code="$(http_get "$url" "$out")"
  case "$code" in
    200)
      if metadata_mentions_repo "$out"; then
        echo "notice: $registry/$name is owned by this project; docs may no longer need the alternate-name rationale"
      else
        echo "notice: $registry/$name is occupied by an external package as documented"
      fi
      ;;
    404)
      echo "notice: $registry/$name appears available; revisit the alternate-name rationale before publishing"
      ;;
    *)
      echo "warning: could not query documented external collision $registry/$name (HTTP $code)" >&2
      ;;
  esac
}

check_claimed_json_name "crates.io" "gnd" "https://crates.io/api/v1/crates/gnd"
check_claimed_json_name "crates.io" "gnd-lsp" "https://crates.io/api/v1/crates/gnd-lsp"
check_claimed_json_name "npm" "gnd-cli" "https://registry.npmjs.org/gnd-cli"
check_claimed_json_name "npm" "gnd-lsp" "https://registry.npmjs.org/gnd-lsp"
check_claimed_json_name "pypi" "gnd-cli" "https://pypi.org/pypi/gnd-cli/json"
check_claimed_json_name "pypi" "gnd-lsp" "https://pypi.org/pypi/gnd-lsp/json"

notice_external_json_name "npm" "gnd" "https://registry.npmjs.org/gnd"
notice_external_json_name "pypi" "gnd" "https://pypi.org/pypi/gnd/json"
