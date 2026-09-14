#!/usr/bin/env python3
"""Scan publishable text for externally supplied protected terms."""

from __future__ import annotations

import argparse
from pathlib import Path


# Limit scanning to recognized text formats and exact special filenames.
# Binary assets and unknown suffixes are outside this name check.
TEXT_SUFFIXES = {
    ".c", ".cc", ".cpp", ".cs", ".h", ".hpp", ".json", ".jsonl",
    ".md", ".ps1", ".py", ".rs", ".sh", ".toml", ".txt", ".xml",
    ".yaml", ".yml", ".html", ".htm", ".js", ".css", ".svg",
}
TEXT_NAMES = {"cargo.lock", "license", "notice"}
EXCLUDED_DIRS = {
    ".git", ".kitaqfc_cache", ".pytest_cache", "bin", "node_modules",
    "obj", "out", "target",
}


# Read the private UTF-8 denylist, allowing a BOM, blank lines and hash-prefixed
# comments. Unicode case-folding makes comparisons case-insensitive.
def load_terms(path: Path) -> list[str]:
    terms = []
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        term = line.strip()
        if term and not term.startswith("#"):
            terms.append(term.casefold())
    return terms


# Yield recognized text files, excluding the resolved denylist and paths
# under named generated directories. rglob still traverses before this filter;
# this is a filename selection rule, not an archive or symlink security boundary.
def iter_text_files(root: Path, denylist: Path):
    for path in root.rglob("*"):
        if not path.is_file() or path.resolve() == denylist:
            continue
        relative = path.relative_to(root)
        if any(part.casefold() in EXCLUDED_DIRS for part in relative.parts[:-1]):
            continue
        if path.suffix.casefold() in TEXT_SUFFIXES or path.name.casefold() in TEXT_NAMES:
            yield path


# Record one path/line match when any deny term occurs as a substring.
# Decode damaged UTF-8 with replacement and keep matched text out of results.
# This heuristic detects names, not provenance, licensing or legal ownership.
def scan(root: Path, denylist: Path) -> list[tuple[Path, int]]:
    terms = load_terms(denylist)
    matches = []
    for path in iter_text_files(root, denylist.resolve()):
        for line_number, line in enumerate(
            path.read_text(encoding="utf-8", errors="replace").splitlines(), 1
        ):
            folded = line.casefold()
            if any(term in folded for term in terms):
                matches.append((path, line_number))
    return matches


# Resolve CLI paths, scan with the caller's private terms and return 1 when
# matches exist. Output gives locations without echoing terms or source lines.
def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--denylist", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    denylist = args.denylist.resolve()
    matches = scan(root, denylist)
    for path, line_number in matches:
        print(f"{path.relative_to(root)}:{line_number}: protected-term match")
    if matches:
        print(f"FAIL rights-name-guard matches={len(matches)}")
        return 1
    print("PASS rights-name-guard matches=0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
