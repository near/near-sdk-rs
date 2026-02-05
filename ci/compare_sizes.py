#!/usr/bin/env python3
"""
Contract size comparison tool for near-sdk-rs.

Compares two JSON files containing contract sizes and outputs a markdown report.

Usage:
    ./compare_sizes.py baseline.json current.json [--threshold 1] [--output report.md]
"""

import argparse
import json
import sys


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


def compare(baseline: dict, current: dict, threshold: float) -> tuple[str, bool, bool]:
    """
    Compare baseline and current sizes.

    Returns: (markdown_report, has_significant_changes, has_increases)
    """
    all_names = sorted(set(baseline.keys()) | set(current.keys()))

    rows = []
    has_significant_changes = False
    has_increases = False

    stats = {"increased": 0, "decreased": 0, "unchanged": 0, "new": 0, "removed": 0}

    for name in all_names:
        base_size = baseline.get(name)
        curr_size = current.get(name)

        if base_size is None:
            rows.append(
                {
                    "name": name,
                    "baseline": "—",
                    "current": format_size(curr_size),  # type: ignore[arg-type]
                    "change": "🆕 new",
                    "percent": "—",
                }
            )
            stats["new"] += 1
            has_significant_changes = True
        elif curr_size is None:
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

    lines.append("| Contract | Baseline | Current | Change | % |")
    lines.append("|----------|----------|---------|--------|---|")

    for row in rows:
        lines.append(
            f"| {row['name']} | {row['baseline']} | {row['current']} | {row['change']} | {row['percent']} |"
        )

    lines.append("")
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

    # Hidden markers for CI
    exit_code = 1 if has_increases else 0
    lines.append(f"<!-- CONTRACT_SIZES_EXIT_CODE:{exit_code} -->")
    lines.append(
        f"<!-- CONTRACT_SIZES_HAS_CHANGES:{str(has_significant_changes).lower()} -->"
    )

    return "\n".join(lines), has_significant_changes, has_increases


def main():
    parser = argparse.ArgumentParser(
        description="Compare contract sizes and generate a markdown report",
    )
    parser.add_argument("baseline", help="Baseline sizes JSON file")
    parser.add_argument("current", help="Current sizes JSON file")
    parser.add_argument(
        "--threshold",
        "-t",
        type=float,
        default=1.0,
        help="Minimum %% change to be considered significant (default: 1.0)",
    )
    parser.add_argument("--output", "-o", help="Output file (default: stdout)")

    args = parser.parse_args()

    with open(args.baseline) as f:
        baseline = json.load(f)
    with open(args.current) as f:
        current = json.load(f)

    report, _, _ = compare(baseline, current, args.threshold)

    if args.output:
        with open(args.output, "w") as f:
            f.write(report)
        print(f"Report written to {args.output}", file=sys.stderr)
    else:
        print(report)


if __name__ == "__main__":
    main()
