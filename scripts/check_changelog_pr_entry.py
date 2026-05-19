#!/usr/bin/env python3
"""Require pull requests to be represented in the Unreleased changelog. §FS-distribution.4"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Sequence


UNRELEASED_RE = re.compile(r"^## Unreleased\s*$")
TOP_LEVEL_RE = re.compile(r"^##(?!#)\s+")


class ChangelogPrError(Exception):
    pass


def check_changelog_pr_entry(changelog: Path, pr_number: int) -> None:
    body = _unreleased_body(changelog)
    if not _mentions_pr(body, pr_number):
        raise ChangelogPrError(
            f"docs/changelog.md ## Unreleased must mention PR #{pr_number}; "
            f"add `PR #{pr_number}` to the relevant changelog bullet"
        )


def pr_number_from_event(event_path: Path) -> int | None:
    try:
        event = json.loads(event_path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ChangelogPrError(f"missing GitHub event file: {event_path}") from exc
    except json.JSONDecodeError as exc:
        raise ChangelogPrError(f"invalid GitHub event JSON: {event_path}: {exc}") from exc

    pull_request = event.get("pull_request")
    if isinstance(pull_request, dict):
        number = pull_request.get("number")
    else:
        number = event.get("number") if event.get("pull_request") is not None else None

    if number is None:
        return None
    if not isinstance(number, int) or number <= 0:
        raise ChangelogPrError(f"invalid pull request number in event: {number!r}")
    return number


def pr_number_from_current_branch() -> int | None:
    try:
        result = subprocess.run(
            ["gh", "pr", "view", "--json", "number", "--jq", ".number"],
            check=False,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError:
        return None

    if result.returncode != 0:
        return None
    output = result.stdout.strip()
    if not output:
        return None
    try:
        number = int(output)
    except ValueError as exc:
        raise ChangelogPrError(f"invalid pull request number from gh: {output!r}") from exc
    if number <= 0:
        raise ChangelogPrError(f"invalid pull request number from gh: {number!r}")
    return number


def _unreleased_body(changelog: Path) -> str:
    try:
        lines = changelog.read_text(encoding="utf-8").splitlines()
    except FileNotFoundError as exc:
        raise ChangelogPrError(f"missing changelog: {changelog}") from exc

    start = None
    for index, line in enumerate(lines):
        if UNRELEASED_RE.match(line):
            start = index + 1
            break
    if start is None:
        raise ChangelogPrError("missing ## Unreleased section in docs/changelog.md")

    end = len(lines)
    for index in range(start, len(lines)):
        if TOP_LEVEL_RE.match(lines[index]):
            end = index
            break
    return "\n".join(lines[start:end])


def _mentions_pr(body: str, pr_number: int) -> bool:
    pr = re.escape(str(pr_number))
    patterns = [
        rf"(?i)\bPR\s*#\s*{pr}\b",
        rf"(?i)\bpull request\s*#\s*{pr}\b",
        rf"/pull/{pr}(?:\b|[/#?)])",
    ]
    return any(re.search(pattern, body) for pattern in patterns)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check that the current PR has an Unreleased changelog entry.")
    parser.add_argument("--changelog", type=Path, default=Path("docs/changelog.md"))
    parser.add_argument("--pr-number", type=int)
    parser.add_argument("--event-path", type=Path, default=None)
    parser.add_argument(
        "--local-pr",
        action="store_true",
        help="resolve the current branch PR with gh; skip if no PR is available",
    )
    args = parser.parse_args(argv)

    try:
        pr_number = args.pr_number
        if pr_number is None:
            event_path = args.event_path
            if event_path is None:
                raw_event_path = os.environ.get("GITHUB_EVENT_PATH")
                event_path = Path(raw_event_path) if raw_event_path else None
            pr_number = pr_number_from_event(event_path) if event_path is not None else None

        if pr_number is None and args.local_pr:
            pr_number = pr_number_from_current_branch()

        if pr_number is None:
            print("not a pull_request event; skipping changelog PR-entry check")
            return 0
        if pr_number <= 0:
            raise ChangelogPrError(f"invalid pull request number: {pr_number}")
        check_changelog_pr_entry(args.changelog, pr_number)
    except ChangelogPrError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
