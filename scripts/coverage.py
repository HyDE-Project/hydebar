#!/usr/bin/env python3
"""Report line coverage of production code only.

``cargo llvm-cov`` counts the bodies of ``#[cfg(test)] mod`` blocks as covered
lines, which flatters every number: a test module is executed by definition.
This script reads an LCOV report, drops every line that lives inside a test
module, and prints what is left — per crate, per file, worst first.

Usage:
    cargo llvm-cov --workspace --lcov --output-path target/coverage.lcov
    python3 scripts/coverage.py target/coverage.lcov [--top N] [--crate NAME]
"""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
import sys

TEST_MOD = re.compile(r"^\s*#\[cfg\(test\)\]\s*$")
MOD_START = re.compile(r"^\s*(pub\s+)?mod\s+\w+\s*\{\s*$")


def test_line_span(path: pathlib.Path) -> set[int]:
    """The 1-indexed lines of `path` that sit inside a `#[cfg(test)]` module."""
    try:
        source = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return set()

    inside: set[int] = set()
    index = 0

    while index < len(source):
        if not TEST_MOD.match(source[index]):
            index += 1
            continue

        start = index
        while index < len(source) and not MOD_START.match(source[index]):
            index += 1
            if index - start > 4:
                break

        if index >= len(source) or not MOD_START.match(source[index]):
            index = start + 1
            continue

        depth = 0
        for line_number in range(index, len(source)):
            depth += source[line_number].count("{") - source[line_number].count("}")
            inside.add(line_number + 1)
            if depth == 0:
                index = line_number + 1
                break
        else:
            index = len(source)

    return inside


def parse(lcov: pathlib.Path, root: pathlib.Path) -> dict[str, tuple[int, int]]:
    """Per-file (covered, total) production line counts."""
    counts: dict[str, tuple[int, int]] = {}
    current: str | None = None
    skip: set[int] = set()
    covered = total = 0

    for raw in lcov.read_text(encoding="utf-8").splitlines():
        if raw.startswith("SF:"):
            current = raw[3:]
            path = pathlib.Path(current)
            if not path.is_absolute():
                path = root / path
            skip = test_line_span(path)
            covered = total = 0
        elif raw.startswith("DA:") and current is not None:
            line_text, _, hits_text = raw[3:].partition(",")
            line = int(line_text)
            if line in skip:
                continue
            total += 1
            if int(hits_text) > 0:
                covered += 1
        elif raw == "end_of_record" and current is not None:
            try:
                relative = str(pathlib.Path(current).relative_to(root))
            except ValueError:
                relative = current
            previous = counts.get(relative, (0, 0))
            counts[relative] = (previous[0] + covered, previous[1] + total)
            current = None

    return counts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("lcov", type=pathlib.Path)
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument("--top", type=int, default=40)
    parser.add_argument("--crate", default=None)
    arguments = parser.parse_args()

    counts = parse(arguments.lcov, arguments.root.resolve())
    if arguments.crate:
        counts = {
            name: value
            for name, value in counts.items()
            if name.startswith(f"{arguments.crate}/")
        }

    per_crate: dict[str, list[int]] = collections.defaultdict(lambda: [0, 0])
    for name, (covered, total) in counts.items():
        crate = name.split("/")[0]
        per_crate[crate][0] += covered
        per_crate[crate][1] += total

    print("=== production lines, per crate ===")
    whole = [0, 0]
    for crate, (covered, total) in sorted(
        per_crate.items(), key=lambda item: item[1][0] - item[1][1]
    ):
        whole[0] += covered
        whole[1] += total
        share = 100 * covered / total if total else 100.0
        print(f"{crate:8} {covered:6}/{total:6} {share:6.2f}%  missing={total - covered}")

    share = 100 * whole[0] / whole[1] if whole[1] else 100.0
    print(f"{'TOTAL':8} {whole[0]:6}/{whole[1]:6} {share:6.2f}%  missing={whole[1] - whole[0]}")

    print(f"\n=== {arguments.top} files with the most uncovered lines ===")
    worst = sorted(counts.items(), key=lambda item: item[1][0] - item[1][1])
    for name, (covered, total) in worst[: arguments.top]:
        if covered == total:
            break
        share = 100 * covered / total if total else 100.0
        print(f"{total - covered:5} miss  {share:6.2f}%  {total:5}L  {name}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
