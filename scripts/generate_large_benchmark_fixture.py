#!/usr/bin/env python3
"""Generate the large conformant repo used by the instruction benchmark."""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path


DEFAULT_FILE_COUNT = 10_000
DEFAULT_COMPONENT_COUNT = 100


def id_for(index: int) -> str:
    return f"FS-{index:05d}-feature-{index:05d}"


def fixture_config() -> str:
    return """grund_config_version = 1

[reference]
strict = true

[scan]
include = ["docs"]
exclude = ["target", "node_modules", ".git", "dist", "build", ".venv"]
extensions = ["md"]
respect_gitignore = false
"""


def declaration_body(index: int, file_count: int, component_count: int) -> str:
    ident = id_for(index)
    next_ident = id_for(index + 1 if index < file_count else 1)
    component = (index - 1) % component_count
    return (
        f"# {ident}: Feature {index:05d}\n\n"
        f"Feature {index:05d} belongs to synthetic component {component:03d} "
        f"and cites §{next_ident} so every declaration is used.\n"
    )


def generate_fixture(root: Path, file_count: int, component_count: int) -> None:
    if file_count < 1:
        raise ValueError("--files must be at least 1")
    if component_count < 1:
        raise ValueError("--components must be at least 1")

    if root.exists():
        shutil.rmtree(root)

    (root / ".agents").mkdir(parents=True)
    (root / ".agents" / "grund.toml").write_text(fixture_config(), encoding="utf-8")

    for index in range(1, file_count + 1):
        component = (index - 1) % component_count
        directory = root / "docs" / "functional-spec" / f"component-{component:03d}"
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / f"{id_for(index)}.md"
        path.write_text(
            declaration_body(index, file_count, component_count),
            encoding="utf-8",
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path("target/bench-fixtures/large-conformant-repo"),
        help="fixture root to replace and regenerate",
    )
    parser.add_argument(
        "--files",
        type=int,
        default=DEFAULT_FILE_COUNT,
        help="number of Markdown declaration files to generate",
    )
    parser.add_argument(
        "--components",
        type=int,
        default=DEFAULT_COMPONENT_COUNT,
        help="number of component directories to spread files across",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    generate_fixture(args.root, args.files, args.components)
    print(f"generated {args.files} files under {args.root}")


if __name__ == "__main__":
    main()
