#!/usr/bin/env bash
set -euo pipefail

# Block Claude / AI-tool attribution boilerplate from landing in committed
# files. Pre-commit passes the staged file paths as arguments; this script
# exits non-zero if any of them contains a recognized attribution line.
#
# Targeted patterns (narrow on purpose — normal prose mentions of "Claude"
# are fine; the boilerplate trailers and "generated with" markers are not):
#   - Co-Authored-By: Claude …
#   - 🤖 Generated with [Claude Code](…)
#   - Generated with [Claude Code]
#   - Claude <noreply@anthropic.com>

if [ "$#" -eq 0 ]; then
  exit 0
fi

pattern='Co-Authored-By: *Claude|Generated with \[?Claude Code\]?|🤖 Generated with|Claude <noreply@anthropic\.com>'

# `grep -I` skips binaries; `-n` prints line numbers; `-E` is the regex flavor
# the pattern above is written for. We invoke it once with every staged path so
# a single grep walks the set.
if matches="$(grep -InE "$pattern" -- "$@" 2>/dev/null)"; then
  printf 'Claude attribution boilerplate found in staged files:\n%s\n' "$matches" >&2
  printf '\nRemove the line(s) above before committing.\n' >&2
  exit 1
fi

exit 0
