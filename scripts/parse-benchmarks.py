#!/usr/bin/env python3
"""
Parse benchmark results from Criterion and Gungraun and convert to JSON for the dashboard.
"""

import json
import re
import sys
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any, Optional, Tuple

def parse_criterion_results(criterion_dir: Path) -> Tuple[Dict[str, Any], bool]:
    """Parse Criterion benchmark results from target/criterion directory.

    Note: This requires that criterion benchmarks have been run locally and
    the target/criterion directory exists. The benchmark.yml workflow does NOT
    run benchmarks - it expects perf-results.md to be generated locally and
    committed, which triggers the workflow.
    """
    results = {
        "timestamp": datetime.now().isoformat(),
        "type": "criterion",
        "benchmarks": [],
        "metrics": {
            "total_benchmarks": 0,
            "avg_time_ms": 0,
            "total_time_ms": 0
        }
    }

    # Look for benchmark groups
    if not criterion_dir.exists():
        print(f"Warning: Criterion directory not found: {criterion_dir}", file=sys.stderr)
        return results, False

    for group_dir in criterion_dir.iterdir():
        if not group_dir.is_dir() or group_dir.name == "report":
            continue

        group_name = group_dir.name

        # First try direct path (single benchmark)
        direct_estimates = group_dir / "base" / "estimates.json"
        if direct_estimates.exists():
            try:
                with open(direct_estimates) as f:
                    estimates = json.load(f)

                mean_ns = estimates.get("mean", {}).get("point_estimate", 0)
                std_dev_ns = estimates.get("std_dev", {}).get("point_estimate", 0)
                median_ns = estimates.get("median", {}).get("point_estimate", mean_ns)

                results["benchmarks"].append({
                    "name": group_name,
                    "mean_ns": mean_ns,
                    "mean_ms": round(mean_ns / 1_000_000, 6),
                    "median_ns": median_ns,
                    "median_ms": round(median_ns / 1_000_000, 6),
                    "std_dev_ns": std_dev_ns,
                    "std_dev_ms": round(std_dev_ns / 1_000_000, 6),
                })
            except Exception as e:
                print(f"Error parsing {direct_estimates}: {e}", file=sys.stderr)
            continue

        # Try nested structure (sub-benchmarks)
        try:
            for sub_dir in group_dir.iterdir():
                if not sub_dir.is_dir():
                    continue

                sub_name = sub_dir.name
                estimates_file = sub_dir / "base" / "estimates.json"

                if not estimates_file.exists():
                    continue

                try:
                    with open(estimates_file) as f:
                        estimates = json.load(f)

                    mean_ns = estimates.get("mean", {}).get("point_estimate", 0)
                    std_dev_ns = estimates.get("std_dev", {}).get("point_estimate", 0)
                    median_ns = estimates.get("median", {}).get("point_estimate", mean_ns)

                    results["benchmarks"].append({
                        "name": f"{group_name}/{sub_name}",
                        "mean_ns": mean_ns,
                        "mean_ms": round(mean_ns / 1_000_000, 6),
                        "median_ns": median_ns,
                        "median_ms": round(median_ns / 1_000_000, 6),
                        "std_dev_ns": std_dev_ns,
                        "std_dev_ms": round(std_dev_ns / 1_000_000, 6),
                    })
                except Exception as e:
                    print(f"Error parsing {estimates_file}: {e}", file=sys.stderr)
        except Exception as e:
            print(f"Error reading sub-benchmarks in {group_dir}: {e}", file=sys.stderr)

    # Calculate aggregate metrics
    if results["benchmarks"]:
        total_time_ms = sum(b["mean_ms"] for b in results["benchmarks"])
        results["metrics"] = {
            "total_benchmarks": len(results["benchmarks"]),
            "avg_time_ms": round(total_time_ms / len(results["benchmarks"]), 6),
            "total_time_ms": round(total_time_ms, 6),
        }

    return results, bool(results["benchmarks"])  # valid only if at least one benchmark parsed

def strip_ansi_codes(text: str) -> str:
    """Remove ANSI color codes from text."""
    ansi_escape = re.compile(r'\x1b\[[0-9;]*m')
    return ansi_escape.sub('', text)

def parse_gungraun_output(output_file: Path) -> Tuple[Dict[str, Any], bool]:
    """Parse Gungraun benchmark output."""
    results = {
        "timestamp": datetime.now().isoformat(),
        "type": "gungraun",
        "benchmarks": [],
        "metrics": {
            "total_benchmarks": 0,
            "avg_instructions": 0,
            "total_instructions": 0,
            "avg_cycles": 0,
            "total_cycles": 0
        }
    }

    if not output_file.exists():
        print(f"Warning: Gungraun output file not found: {output_file}", file=sys.stderr)
        print("Using default metrics", file=sys.stderr)
        return results, False

    try:
        with open(output_file) as file:
            content = file.read()

        # Strip ANSI color codes before parsing
        content = strip_ansi_codes(content)

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

            results["benchmarks"].append({
                "name": bench_name,
                "instructions": instructions,
                "l1_accesses": l1_accesses,
                "l2_accesses": l2_accesses,
                "ram_accesses": ram_accesses,
                "estimated_cycles": estimated_cycles
            })

        # Calculate aggregate metrics
        if results["benchmarks"]:
            total_instructions = sum(bench["instructions"] for bench in results["benchmarks"])
            total_cycles = sum(bench["estimated_cycles"] for bench in results["benchmarks"])
            num_benchmarks = len(results["benchmarks"])

            results["metrics"] = {
                "total_benchmarks": num_benchmarks,
                "avg_instructions": round(total_instructions / num_benchmarks),
                "total_instructions": total_instructions,
                "avg_cycles": round(total_cycles / num_benchmarks),
                "total_cycles": total_cycles,
            }

    except Exception as exception:
        print(f"Error parsing Gungraun output: {exception}", file=sys.stderr)
        import traceback
        traceback.print_exc()

    return results, bool(results["benchmarks"])  # valid only if at least one benchmark parsed

def parse_quality_benchmarks(results_file: Path) -> Tuple[Dict[str, Any], bool]:
    """Parse quality benchmark results from markdown table format."""
    results = {
        "timestamp": datetime.now().isoformat(),
        "type": "quality",
        "metrics": {
            "test_cases": 0,
            "precision_at_3": 0.0,
            "precision_at_10": 0.0,
            "recall_at_10": 0.0,
            "mrr": 0.0,
            "ndcg_at_10": 0.0,
            "critical_in_top_3": 0.0
        }
    }

    if not results_file.exists():
        print(f"Warning: Quality results file not found: {results_file}", file=sys.stderr)
        print("Using default metrics", file=sys.stderr)
        return results, False

    try:
        with open(results_file) as f:
            content = f.read()

        # Parse test cases count
        test_cases_match = re.search(r'\*\*Test Cases\*\*:\s*(\d+)', content)
        if test_cases_match:
            results["metrics"]["test_cases"] = int(test_cases_match.group(1))

        # Parse metrics from table
        # Format: | Precision@3 | 45.2% | 60% |
        precision_3_match = re.search(r'\|\s*Precision@3\s*\|\s*([\d.]+|NaN)%', content)
        if precision_3_match and precision_3_match.group(1) != "NaN":
            results["metrics"]["precision_at_3"] = float(precision_3_match.group(1))

        precision_10_match = re.search(r'\|\s*Precision@10\s*\|\s*([\d.]+|NaN)%', content)
        if precision_10_match and precision_10_match.group(1) != "NaN":
            results["metrics"]["precision_at_10"] = float(precision_10_match.group(1))

        recall_10_match = re.search(r'\|\s*Recall@10\s*\|\s*([\d.]+|NaN)%', content)
        if recall_10_match and recall_10_match.group(1) != "NaN":
            results["metrics"]["recall_at_10"] = float(recall_10_match.group(1))

        mrr_match = re.search(r'\|\s*MRR\s*\|\s*([\d.]+|NaN)\s*\|', content)
        if mrr_match and mrr_match.group(1) != "NaN":
            results["metrics"]["mrr"] = float(mrr_match.group(1))

        ndcg_match = re.search(r'\|\s*NDCG@10\s*\|\s*(-?[\d.]+|NaN)\s*\|', content)
        if ndcg_match and ndcg_match.group(1) != "NaN":
            results["metrics"]["ndcg_at_10"] = float(ndcg_match.group(1))

        critical_match = re.search(r'\|\s*Critical in Top-3\s*\|\s*([\d.]+|NaN)%', content)
        if critical_match and critical_match.group(1) != "NaN":
            results["metrics"]["critical_in_top_3"] = float(critical_match.group(1))

    except Exception as e:
        print(f"Error parsing quality benchmarks: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()

    return results

def main():
    import argparse

    parser = argparse.ArgumentParser(description="Parse benchmark results and generate JSON")
    parser.add_argument("--criterion-dir", type=Path, default=Path("target/criterion"),
                       help="Path to Criterion output directory")
    parser.add_argument("--gungraun-output", type=Path, default=Path("gungraun-results.md"),
                       help="Path to Gungraun output file")
    parser.add_argument("--quality-results", type=Path, default=Path("quality-results.md"),
                       help="Path to quality benchmark results")
    parser.add_argument("--output-dir", type=Path, default=Path("benchmark-data"),
                       help="Output directory for JSON files")

    args = parser.parse_args()

    # Create output directories
    criterion_dir = args.output_dir / "criterion"
    gungraun_dir = args.output_dir / "gungraun"
    quality_dir = args.output_dir / "quality"

    criterion_dir.mkdir(parents=True, exist_ok=True)
    gungraun_dir.mkdir(parents=True, exist_ok=True)
    quality_dir.mkdir(parents=True, exist_ok=True)

    # Parse Criterion results
    criterion_results, criterion_valid = parse_criterion_results(args.criterion_dir)
    if criterion_valid:
        output_file = criterion_dir / "latest.json"
        with open(output_file, 'w') as f:
            json.dump(criterion_results, f, indent=2)
    else:
        print("Skipping criterion output (no benchmarks parsed)", file=sys.stderr)

    # Parse Gungraun results
    gungraun_results, gungraun_valid = parse_gungraun_output(args.gungraun_output)
    if gungraun_valid:
        output_file = gungraun_dir / "latest.json"
        with open(output_file, 'w') as f:
            json.dump(gungraun_results, f, indent=2)
    else:
        print("Skipping gungraun output (no benchmarks parsed)", file=sys.stderr)

    # Parse quality benchmarks
    quality_results, quality_valid = parse_quality_benchmarks(args.quality_results)
    if quality_valid:
        output_file = quality_dir / "latest.json"
        with open(output_file, 'w') as f:
            json.dump(quality_results, f, indent=2)
    else:
        print("Skipping quality output (only default metrics available)", file=sys.stderr)

if __name__ == "__main__":
    main()
