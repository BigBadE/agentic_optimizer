#!/usr/bin/env python3
"""
Format Gungraun benchmark output into markdown tables.
Reads gungraun terminal output and converts to readable markdown format.
"""

import re
import sys
from pathlib import Path
from typing import List, Dict, Any


def parse_gungraun_terminal_output(content: str) -> List[Dict[str, Any]]:
    """Parse gungraun terminal output format."""
    benchmarks = []

    # Parse gungraun/iai-callgrind format benchmark results
    # Format example:
    # bench_name  Instructions:               38331 (+0.046981%)
    #             L1 Accesses:                53765 (+0.048382%)
    #             L2 Accesses:                    6 (-14.28571%)
    #             RAM Accesses:                  45 (+4.651163%)
    #             Estimated Cycles:           55370 (+0.164619%)

    # More flexible pattern - allow various whitespace and formats
    # Handles both old format (name Instructions: value) and new format (name\n  Instructions: value)
    bench_pattern = re.compile(
        r'([a-zA-Z_][a-zA-Z0-9_/::\-]+)\s*\n?\s*Instructions:\s*([\d,]+)',
        re.MULTILINE
    )

    for match in bench_pattern.finditer(content):
        bench_name = match.group(1).strip()
        instructions = int(match.group(2).replace(',', ''))

        # Extract position after the benchmark name to find associated metrics
        start_pos = match.end()
        # Find the next benchmark or end of string
        next_match = bench_pattern.search(content, start_pos)
        end_pos = next_match.start() if next_match else len(content)
        bench_section = content[start_pos:end_pos]

        # Extract L1, L2, RAM accesses and estimated cycles
        l1_accesses = 0
        l2_accesses = 0
        ram_accesses = 0
        estimated_cycles = 0

        l1_match = re.search(r'L1 (?:Accesses|Hits):\s+([\d,]+)', bench_section)
        if l1_match:
            l1_accesses = int(l1_match.group(1).replace(',', ''))

        # Handle both "L2 Accesses/Hits" and "LL Hits" (last level cache)
        l2_match = re.search(r'(?:L2|LL) (?:Accesses|Hits):\s+([\d,]+)', bench_section)
        if l2_match:
            l2_accesses = int(l2_match.group(1).replace(',', ''))

        ram_match = re.search(r'RAM (?:Accesses|Hits):\s+([\d,]+)', bench_section)
        if ram_match:
            ram_accesses = int(ram_match.group(1).replace(',', ''))

        cycles_match = re.search(r'Estimated Cycles:\s+([\d,]+)', bench_section)
        if cycles_match:
            estimated_cycles = int(cycles_match.group(1).replace(',', ''))

        benchmarks.append({
            "name": bench_name,
            "instructions": instructions,
            "l1_accesses": l1_accesses,
            "l2_accesses": l2_accesses,
            "ram_accesses": ram_accesses,
            "estimated_cycles": estimated_cycles
        })

    return benchmarks


def format_number(num: int) -> str:
    """Format large numbers with commas for readability."""
    return f"{num:,}"


def generate_markdown_report(benchmarks: List[Dict[str, Any]]) -> str:
    """Generate markdown report from gungraun benchmark results."""
    if not benchmarks:
        return "# Gungraun Benchmark Results\n\nNo benchmarks found in output.\n"

    report = "# Gungraun Benchmark Results\n\n"
    report += "*High-precision instruction-level benchmarks using Valgrind Callgrind*\n\n"

    # Calculate totals
    total_instructions = sum(bench["instructions"] for bench in benchmarks)
    total_cycles = sum(bench["estimated_cycles"] for bench in benchmarks)
    total_l1 = sum(bench["l1_accesses"] for bench in benchmarks)
    total_l2 = sum(bench["l2_accesses"] for bench in benchmarks)
    total_ram = sum(bench["ram_accesses"] for bench in benchmarks)

    report += "## Summary\n\n"
    report += f"**Total Benchmarks**: {len(benchmarks)}\n\n"
    report += "| Metric | Total | Average |\n"
    report += "|--------|-------|--------|\n"
    report += f"| Instructions | {format_number(total_instructions)} | {format_number(total_instructions // len(benchmarks))} |\n"
    report += f"| Estimated Cycles | {format_number(total_cycles)} | {format_number(total_cycles // len(benchmarks))} |\n"
    report += f"| L1 Accesses | {format_number(total_l1)} | {format_number(total_l1 // len(benchmarks))} |\n"
    report += f"| L2 Accesses | {format_number(total_l2)} | {format_number(total_l2 // len(benchmarks))} |\n"
    report += f"| RAM Accesses | {format_number(total_ram)} | {format_number(total_ram // len(benchmarks))} |\n\n"

    # Individual benchmark results
    report += "## Individual Benchmarks\n\n"
    report += "| Benchmark | Instructions | Est. Cycles | L1 Access | L2 Access | RAM Access |\n"
    report += "|-----------|--------------|-------------|-----------|-----------|------------|\n"

    for bench in benchmarks:
        report += f"| `{bench['name']}` | "
        report += f"{format_number(bench['instructions'])} | "
        report += f"{format_number(bench['estimated_cycles'])} | "
        report += f"{format_number(bench['l1_accesses'])} | "
        report += f"{format_number(bench['l2_accesses'])} | "
        report += f"{format_number(bench['ram_accesses'])} |\n"

    report += "\n---\n"
    report += "*Generated by Gungraun formatter*\n"

    return report


def main():
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage: format-gungraun.py <input-file> [output-file]", file=sys.stderr)
        print("  If output-file is not specified, prints to stdout", file=sys.stderr)
        sys.exit(1)

    input_file = Path(sys.argv[1])
    output_file = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not input_file.exists():
        print(f"Error: Input file not found: {input_file}", file=sys.stderr)
        sys.exit(1)

    try:
        with open(input_file) as file:
            content = file.read()

        benchmarks = parse_gungraun_terminal_output(content)
        markdown_report = generate_markdown_report(benchmarks)

        if output_file:
            with open(output_file, 'w') as file:
                file.write(markdown_report)
            print(f"✅ Markdown report written to: {output_file}")
        else:
            print(markdown_report)

    except Exception as exception:
        print(f"Error: {exception}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
