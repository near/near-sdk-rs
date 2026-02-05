#!/usr/bin/env python3
"""
Compare contract sizes between PR and base branch.

This script reads size data from two directories (PR sizes and base sizes)
and generates a markdown report showing the differences.

Usage:
    python3 compare_sizes.py <pr_sizes_dir> <base_sizes_dir>

Size files are expected to be in the format:
    example-name:filename.wasm:size_in_bytes
"""

import sys
import glob
from pathlib import Path


# Threshold for reporting size changes (1% = 0.01)
SIZE_CHANGE_THRESHOLD = 0.01


def parse_size_file(filepath: Path) -> dict[str, tuple[str, int]]:
    """
    Parse a size file and return a dict mapping example name to (filename, size).

    Uses example name (directory-based) as key to avoid duplicate filename issues.
    """
    sizes = {}
    try:
        with open(filepath) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                parts = line.split(":")
                if len(parts) >= 3:
                    example_name = parts[0]
                    wasm_filename = parts[1]
                    size = int(parts[2])
                    if example_name in sizes:
                        print(
                            f"Warning: Duplicate example '{example_name}' found, "
                            f"keeping first occurrence",
                            file=sys.stderr,
                        )
                    else:
                        sizes[example_name] = (wasm_filename, size)
    except (OSError, ValueError) as e:
        print(f"Warning: Error reading {filepath}: {e}", file=sys.stderr)
    return sizes


def collect_sizes(directory: Path) -> dict[str, tuple[str, int]]:
    """
    Collect all sizes from text files in a directory.

    Returns a dict mapping example name to (filename, size).
    """
    all_sizes = {}
    for filepath in glob.glob(str(directory / "*.txt")):
        sizes = parse_size_file(Path(filepath))
        for example_name, (filename, size) in sizes.items():
            if example_name in all_sizes:
                print(
                    f"Warning: Duplicate example '{example_name}' across files, "
                    f"keeping first occurrence",
                    file=sys.stderr,
                )
            else:
                all_sizes[example_name] = (filename, size)
    return all_sizes


def format_size(size: int) -> str:
    """Format size in human-readable format."""
    if size >= 1024 * 1024:
        return f"{size / (1024 * 1024):.2f} MB"
    elif size >= 1024:
        return f"{size / 1024:.2f} KB"
    return f"{size} B"


def generate_report(
    pr_sizes: dict[str, tuple[str, int]], base_sizes: dict[str, tuple[str, int]]
) -> str:
    """Generate a markdown comparison report."""
    # Get all example names
    all_examples = sorted(set(pr_sizes.keys()) | set(base_sizes.keys()))

    if not all_examples:
        return ""

    rows = []
    has_significant_changes = False

    for example in all_examples:
        pr_data = pr_sizes.get(example)
        base_data = base_sizes.get(example)

        if pr_data and base_data:
            pr_filename, pr_size = pr_data
            base_filename, base_size = base_data

            if base_size > 0:
                diff_pct = (pr_size - base_size) / base_size
                diff_str = f"{diff_pct:+.2%}"

                if abs(diff_pct) >= SIZE_CHANGE_THRESHOLD:
                    has_significant_changes = True
            else:
                diff_str = "N/A"

            rows.append(
                f"| {example} | {base_size:,} | {pr_size:,} | {diff_str} |"
            )
        elif pr_data:
            _, pr_size = pr_data
            rows.append(f"| {example} | - | {pr_size:,} | *new* |")
            has_significant_changes = True
        elif base_data:
            _, base_size = base_data
            rows.append(f"| {example} | {base_size:,} | - | *removed* |")
            has_significant_changes = True

    if not has_significant_changes:
        return ""

    header = """# Contract size report

Sizes are given in bytes. Only showing changes >= 1%.

| Contract | Base | PR | Difference |
| --- | ---: | ---: | ---: |"""

    return "\n".join([header, *rows])


def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <pr_sizes_dir> <base_sizes_dir>", file=sys.stderr)
        sys.exit(1)

    pr_dir = Path(sys.argv[1])
    base_dir = Path(sys.argv[2])

    pr_sizes = collect_sizes(pr_dir)
    base_sizes = collect_sizes(base_dir)

    report = generate_report(pr_sizes, base_sizes)
    if report:
        print(report)


if __name__ == "__main__":
    main()
