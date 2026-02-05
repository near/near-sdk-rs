#!/usr/bin/env python3
"""
Contract size comparison tool for near-sdk-rs examples.

Usage:
    ./compare_sizes.py build [--output FILE]    Build all examples and output sizes as JSON
    ./compare_sizes.py compare BASE CURRENT     Compare two size JSON files and output markdown report

Examples:
    # Build and collect sizes
    ./compare_sizes.py build > sizes.json

    # Compare against baseline
    ./compare_sizes.py compare baseline.json sizes.json --threshold 1
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

# All example contracts to build and track
EXAMPLES = [
    "examples/adder",
    "examples/callback-results",
    "examples/cross-contract-calls/high-level",
    "examples/cross-contract-calls/low-level",
    "examples/factory-contract/high-level",
    "examples/factory-contract/low-level",
    "examples/fungible-token/ft",
    "examples/fungible-token/test-contract-defi",
    "examples/lockable-fungible-token",
    "examples/mission-control",
    "examples/mpc-contract",
    "examples/non-fungible-token/nft",
    "examples/non-fungible-token/test-approval-receiver",
    "examples/non-fungible-token/test-token-receiver",
    "examples/status-message",
    "examples/test-contract",
    "examples/versioned",
]


def get_project_root() -> Path:
    """Get the project root directory (where this script lives is ci/, so go up one level)."""
    return Path(__file__).parent.parent.resolve()


def build_example(example_path: Path) -> bool:
    """Build a single example contract. Returns True on success."""
    print(f"Building {example_path}...", file=sys.stderr)
    try:
        result = subprocess.run(
            ["cargo", "near", "build", "reproducible-wasm", "--no-locked"],
            cwd=example_path,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"  Warning: {example_path} failed to build", file=sys.stderr)
            print(result.stderr, file=sys.stderr)
            return False
        print(f"  Done: {example_path}", file=sys.stderr)
        return True
    except Exception as e:
        print(f"  Error building {example_path}: {e}", file=sys.stderr)
        return False


def collect_sizes(project_root: Path) -> dict[str, int]:
    """Collect sizes of all built wasm files in examples/**/target/near/**/*.wasm."""
    sizes = {}
    examples_dir = project_root / "examples"

    # Match wasms at any depth under target/near/
    for wasm_file in examples_dir.rglob("target/near/**/*.wasm"):
        name = wasm_file.name
        size = wasm_file.stat().st_size
        sizes[name] = size

    return sizes


def cmd_build(args) -> int:
    """Build all examples and output sizes as JSON."""
    project_root = get_project_root()

    # Build all examples
    failed = []
    for example in EXAMPLES:
        example_path = project_root / example
        if not example_path.exists():
            print(f"Warning: {example} does not exist, skipping", file=sys.stderr)
            continue
        if not build_example(example_path):
            failed.append(example)

    if failed:
        print(f"\nFailed to build: {', '.join(failed)}", file=sys.stderr)

    # Collect and output sizes
    sizes = collect_sizes(project_root)

    if args.output:
        with open(args.output, "w") as f:
            json.dump(sizes, f, indent=2, sort_keys=True)
        print(f"Sizes written to {args.output}", file=sys.stderr)
    else:
        print(json.dumps(sizes, indent=2, sort_keys=True))

    return 0


def format_size(size_bytes: int) -> str:
    """Format bytes as human-readable size."""
    if size_bytes >= 1_048_576:
        return f"{size_bytes / 1_048_576:.2f}MB"
    elif size_bytes >= 1024:
        return f"{size_bytes / 1024:.1f}KB"
    else:
        return f"{size_bytes}B"


def format_diff_percent(percent: float) -> str:
    """Format percentage difference."""
    if percent > 0:
        return f"+{percent:.2f}%"
    elif percent < 0:
        return f"{percent:.2f}%"
    else:
        return "0%"


def format_diff_bytes(diff: int) -> str:
    """Format byte difference."""
    if diff > 0:
        return f"+{diff}"
    elif diff < 0:
        return str(diff)
    else:
        return "±0"


def cmd_compare(args) -> int:
    """Compare two size JSON files and output a markdown report."""
    # Load baseline and current sizes
    with open(args.baseline) as f:
        baseline = json.load(f)
    with open(args.current) as f:
        current = json.load(f)

    threshold = args.threshold

    # Get all contract names from both
    all_names = sorted(set(baseline.keys()) | set(current.keys()))

    # Build comparison data
    rows = []
    has_significant_changes = False
    has_increases = False

    stats = {"increased": 0, "decreased": 0, "unchanged": 0, "new": 0, "removed": 0}

    for name in all_names:
        base_size = baseline.get(name)
        curr_size = current.get(name)

        if base_size is None:
            # New contract
            rows.append(
                {
                    "name": name,
                    "baseline": "—",
                    "current": format_size(curr_size),
                    "change": "🆕 new",
                    "percent": "—",
                }
            )
            stats["new"] += 1
            has_significant_changes = True
        elif curr_size is None:
            # Removed contract
            rows.append(
                {
                    "name": name,
                    "baseline": format_size(base_size),
                    "current": "—",
                    "change": "🗑️ removed",
                    "percent": "—",
                }
            )
            stats["removed"] += 1
            has_significant_changes = True
        else:
            # Both exist - compare
            diff_bytes = curr_size - base_size
            diff_percent = (diff_bytes / base_size * 100) if base_size > 0 else 0

            rows.append(
                {
                    "name": name,
                    "baseline": format_size(base_size),
                    "current": format_size(curr_size),
                    "change": f"{format_diff_bytes(diff_bytes)}B",
                    "percent": format_diff_percent(diff_percent),
                }
            )

            if abs(diff_percent) >= threshold:
                has_significant_changes = True
                if diff_percent > 0:
                    has_increases = True
                    stats["increased"] += 1
                else:
                    stats["decreased"] += 1
            else:
                stats["unchanged"] += 1

    # Generate markdown report
    lines = ["## 📊 Contract Size Report", ""]

    if not has_significant_changes:
        lines.append(
            f"✅ No significant size changes detected (threshold: {threshold}%)"
        )
        lines.append("")

    # Table header
    lines.append("| Contract | Baseline | Current | Change | % |")
    lines.append("|----------|----------|---------|--------|---|")

    # Table rows
    for row in rows:
        lines.append(
            f"| {row['name']} | {row['baseline']} | {row['current']} | {row['change']} | {row['percent']} |"
        )

    lines.append("")

    # Summary in details
    lines.append("<details>")
    lines.append("<summary>Summary</summary>")
    lines.append("")
    lines.append(f"- **Total contracts:** {len(all_names)}")
    lines.append(f"- **Increased:** {stats['increased']}")
    lines.append(f"- **Decreased:** {stats['decreased']}")
    lines.append(f"- **Unchanged:** {stats['unchanged']}")
    if stats["new"] > 0:
        lines.append(f"- **New:** {stats['new']}")
    if stats["removed"] > 0:
        lines.append(f"- **Removed:** {stats['removed']}")
    lines.append("</details>")
    lines.append("")

    # Hidden marker for CI to parse
    exit_code = 1 if has_increases else 0
    lines.append(f"<!-- CONTRACT_SIZES_EXIT_CODE:{exit_code} -->")
    lines.append(
        f"<!-- CONTRACT_SIZES_HAS_CHANGES:{str(has_significant_changes).lower()} -->"
    )

    report = "\n".join(lines)

    if args.output:
        with open(args.output, "w") as f:
            f.write(report)
        print(f"Report written to {args.output}", file=sys.stderr)
    else:
        print(report)

    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Contract size comparison tool for near-sdk-rs examples",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Build command
    build_parser = subparsers.add_parser(
        "build", help="Build all examples and output sizes as JSON"
    )
    build_parser.add_argument("--output", "-o", help="Output file (default: stdout)")
    build_parser.set_defaults(func=cmd_build)

    # Compare command
    compare_parser = subparsers.add_parser(
        "compare", help="Compare two size JSON files"
    )
    compare_parser.add_argument("baseline", help="Baseline sizes JSON file")
    compare_parser.add_argument("current", help="Current sizes JSON file")
    compare_parser.add_argument(
        "--threshold",
        "-t",
        type=float,
        default=1.0,
        help="Minimum %% change to be considered significant (default: 1.0)",
    )
    compare_parser.add_argument("--output", "-o", help="Output file (default: stdout)")
    compare_parser.set_defaults(func=cmd_compare)

    args = parser.parse_args()
    sys.exit(args.func(args))


if __name__ == "__main__":
    main()
