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
        "benchmarks": [],
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

        # Parse metrics from aggregate table
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

        # Parse individual test results
        # Look for pattern like:
        # Test: test_name
        # Query: some query text
        # Metrics:
        #   P@3:  66.7%
        #   P@10: 40.0%
        #   R@10: 40.0%
        #   MRR:  0.611
        #   NDCG: 0.471
        #   Crit: 66.7%

        test_pattern = re.compile(
            r'Test:\s*([^\n]+)\s*\n'
            r'Query:\s*([^\n]+)\s*\n'
            r'[^\n]*\n'  # Results count line
            r'Metrics:\s*\n'
            r'\s*P@3:\s*([\d.]+)%\s*\n'
            r'\s*P@10:\s*([\d.]+)%\s*\n'
            r'\s*R@10:\s*([\d.]+)%\s*\n'
            r'\s*MRR:\s*([\d.]+)\s*\n'
            r'\s*NDCG:\s*([\d.]+)\s*\n'
            r'\s*Crit:\s*([\d.]+)%',
            re.MULTILINE
        )

        for match in test_pattern.finditer(content):
            test_name = match.group(1).strip()
            query = match.group(2).strip()
            p3 = float(match.group(3))
            p10 = float(match.group(4))
            r10 = float(match.group(5))
            mrr = float(match.group(6))
            ndcg = float(match.group(7))
            crit = float(match.group(8))

            results["benchmarks"].append({
                "test_case": test_name,
                "query": query,
                "precision_at_3": p3,
                "precision_at_10": p10,
                "recall_at_10": r10,
                "mrr": mrr,
                "ndcg_at_10": ndcg,
                "critical_in_top_3": crit
            })

    except Exception as e:
        print(f"Error parsing quality benchmarks: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()

    # valid if at least one parsed metric > 0 or test_cases > 0
    metrics = results["metrics"]
    valid = (
        metrics.get("test_cases", 0) > 0
        or any(
            (isinstance(value, (int, float)) and value > 0)
            for key, value in metrics.items()
            if key != "test_cases"
        )
    )
    return results, valid

def parse_timestamp(ts: str) -> datetime:
    """Parse ISO timestamp to datetime object."""
    return datetime.fromisoformat(ts.replace('Z', '+00:00'))

def smart_downsample_history(history: List[Dict[str, Any]], max_entries: int = 500) -> List[Dict[str, Any]]:
    """
    Intelligently downsample history to max_entries by removing entries that are:
    1. Older (prioritize keeping recent data)
    2. Close to other entries (remove redundant nearby points)

    Uses a weighted scoring system where older + closer entries get removed first.
    """
    if len(history) <= max_entries:
        return history

    # Calculate timestamps as seconds since epoch for easier math
    entries = []
    for i, entry in enumerate(history):
        try:
            ts = parse_timestamp(entry["timestamp"])
            entries.append({
                "index": i,
                "timestamp_dt": ts,
                "timestamp_sec": ts.timestamp(),
                "data": entry
            })
        except Exception as e:
            print(f"Warning: Could not parse timestamp {entry.get('timestamp')}: {e}", file=sys.stderr)
            # Keep entries with unparseable timestamps
            entries.append({
                "index": i,
                "timestamp_dt": None,
                "timestamp_sec": 0,
                "data": entry
            })

    # Always keep the first and last entries
    to_keep = {0, len(entries) - 1}

    # Calculate how many entries we need to remove
    to_remove_count = len(entries) - max_entries

    # Score each entry (except first and last) for removal
    # Higher score = more likely to remove
    scores = []

    for i in range(1, len(entries) - 1):
        if entries[i]["timestamp_sec"] == 0:
            # Don't remove entries with bad timestamps
            continue

        # Calculate time distance to nearest neighbors
        time_to_prev = entries[i]["timestamp_sec"] - entries[i-1]["timestamp_sec"]
        time_to_next = entries[i+1]["timestamp_sec"] - entries[i]["timestamp_sec"]
        min_neighbor_distance = min(time_to_prev, time_to_next)

        # Calculate age (older = higher score)
        # Normalize to 0-1 range where newest = 0, oldest = 1
        total_time_range = entries[-1]["timestamp_sec"] - entries[0]["timestamp_sec"]
        if total_time_range > 0:
            age_score = (entries[i]["timestamp_sec"] - entries[0]["timestamp_sec"]) / total_time_range
        else:
            age_score = 0

        # Calculate proximity score (closer to neighbors = higher score)
        # Normalize to 0-1 range where farthest = 0, closest = 1
        if total_time_range > 0:
            proximity_score = 1.0 - (min_neighbor_distance / (total_time_range / len(entries)))
            proximity_score = max(0.0, min(1.0, proximity_score))
        else:
            proximity_score = 0

        # Combined score: 60% proximity, 40% age
        # This prioritizes removing points that are close to neighbors,
        # but also biases toward removing older points
        combined_score = (0.6 * proximity_score) + (0.4 * age_score)

        scores.append({
            "index": i,
            "score": combined_score
        })

    # Sort by score (highest first) and mark top N for removal
    scores.sort(key=lambda x: x["score"], reverse=True)

    for i in range(min(to_remove_count, len(scores))):
        # Don't add to to_keep set (these get removed)
        pass

    # Add all entries we want to keep
    for i in range(len(entries)):
        if i in to_keep:
            continue
        # Check if this index is in the removal list
        should_remove = False
        for j in range(min(to_remove_count, len(scores))):
            if scores[j]["index"] == i:
                should_remove = True
                break
        if not should_remove:
            to_keep.add(i)

    # Return kept entries in original order
    result = [entries[i]["data"] for i in sorted(to_keep)]

    print(f"Downsampled history from {len(history)} to {len(result)} entries", file=sys.stderr)
    return result

def load_existing_history(output_file: Path, max_history: int = 500) -> List[Dict[str, Any]]:
    """Load existing history from latest.json if it exists."""
    if not output_file.exists():
        return []

    try:
        with open(output_file) as f:
            data = json.load(f)
            history = data.get("history", [])
            # Use smart downsampling instead of simple truncation
            return smart_downsample_history(history, max_history)
    except Exception as e:
        print(f"Warning: Could not load history from {output_file}: {e}", file=sys.stderr)
        return []

def add_history_entry(results: Dict[str, Any], existing_history: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Add current results as a history entry and append to existing history."""
    # Create a history entry from current metrics
    history_entry = {
        "timestamp": results["timestamp"],
    }

    # Add all metrics to history
    if "metrics" in results:
        history_entry.update(results["metrics"])

    # Append to history (most recent last)
    new_history = existing_history + [history_entry]

    # Add history to results
    results["history"] = new_history

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
    parser.add_argument("--history-source", type=Path, default=None,
                       help="Path to gh-pages data directory to load existing history from")

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

        # Load existing history from gh-pages if available
        history_file = output_file
        if args.history_source:
            history_file = args.history_source / "criterion" / "latest.json"

        existing_history = load_existing_history(history_file)
        criterion_results = add_history_entry(criterion_results, existing_history)

        with open(output_file, 'w') as f:
            json.dump(criterion_results, f, indent=2)
    else:
        print("Skipping criterion output (no benchmarks parsed)", file=sys.stderr)

    # Parse Gungraun results
    gungraun_results, gungraun_valid = parse_gungraun_output(args.gungraun_output)
    if gungraun_valid:
        output_file = gungraun_dir / "latest.json"

        # Load existing history from gh-pages if available
        history_file = output_file
        if args.history_source:
            history_file = args.history_source / "gungraun" / "latest.json"

        existing_history = load_existing_history(history_file)
        gungraun_results = add_history_entry(gungraun_results, existing_history)

        with open(output_file, 'w') as f:
            json.dump(gungraun_results, f, indent=2)
    else:
        print("Skipping gungraun output (no benchmarks parsed)", file=sys.stderr)

    # Parse quality benchmarks
    quality_results, quality_valid = parse_quality_benchmarks(args.quality_results)
    if quality_valid:
        output_file = quality_dir / "latest.json"

        # Load existing history from gh-pages if available
        history_file = output_file
        if args.history_source:
            history_file = args.history_source / "quality" / "latest.json"

        existing_history = load_existing_history(history_file)
        quality_results = add_history_entry(quality_results, existing_history)

        with open(output_file, 'w') as f:
            json.dump(quality_results, f, indent=2)
    else:
        print("Skipping quality output (only default metrics available)", file=sys.stderr)

if __name__ == "__main__":
    main()
