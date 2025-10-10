#!/usr/bin/env python3
"""
Parse benchmark results from Criterion and Gungraun and convert to JSON for the dashboard.
"""

import json
import re
import sys
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any, Optional

def parse_criterion_results(criterion_dir: Path) -> Dict[str, Any]:
    """Parse Criterion benchmark results from target/criterion directory."""
    results = {
        "timestamp": datetime.now().isoformat(),
        "type": "criterion",
        "benchmarks": [],
        "metrics": {}
    }

    # Look for benchmark groups
    if not criterion_dir.exists():
        raise FileNotFoundError(f"Criterion directory not found: {criterion_dir}")

    # Recursively find all estimates.json files
    def find_benchmarks(directory: Path, prefix: str = ""):
        """Recursively find benchmark results."""
        for item in directory.iterdir():
            if not item.is_dir() or item.name == "report":
                continue

            # Check if this directory has a base/estimates.json
            estimates_file = item / "base" / "estimates.json"
            if estimates_file.exists():
                try:
                    with open(estimates_file) as f:
                        estimates = json.load(f)

                    # Extract mean time
                    mean_time_ns = estimates.get("mean", {}).get("point_estimate", 0)
                    mean_time_ms = mean_time_ns / 1_000_000  # Convert ns to ms

                    # Build hierarchical name
                    bench_name = f"{prefix}/{item.name}" if prefix else item.name

                    results["benchmarks"].append({
                        "name": bench_name,
                        "mean_time_ms": round(mean_time_ms, 3),
                        "mean_time_ns": mean_time_ns,
                        "std_dev": estimates.get("std_dev", {}).get("point_estimate", 0),
                    })
                except Exception as e:
                    print(f"Error parsing {estimates_file}: {e}", file=sys.stderr)
            else:
                # Recurse into subdirectories
                new_prefix = f"{prefix}/{item.name}" if prefix else item.name
                find_benchmarks(item, new_prefix)

    find_benchmarks(criterion_dir)
    
    # Calculate aggregate metrics
    if not results["benchmarks"]:
        raise ValueError("No Criterion benchmarks found - check that benchmarks have been run")

    total_time = sum(b["mean_time_ms"] for b in results["benchmarks"])
    results["metrics"] = {
        "total_benchmarks": len(results["benchmarks"]),
        "avg_time_ms": round(total_time / len(results["benchmarks"]), 3),
        "total_time_ms": round(total_time, 3),
    }

    return results

def parse_gungraun_output(output_file: Path) -> Dict[str, Any]:
    """Parse Gungraun benchmark output."""
    results = {
        "timestamp": datetime.now().isoformat(),
        "type": "gungraun",
        "metrics": {}
    }

    if not output_file.exists():
        raise FileNotFoundError(f"Gungraun output file not found: {output_file}")
    
    with open(output_file) as f:
        content = f.read()

    # Parse total instructions
    instr_match = re.search(r'I\s+refs:\s+([\d,]+)', content)
    if not instr_match:
        raise ValueError("Could not find instruction references (I refs) in Gungraun output")
    instructions = int(instr_match.group(1).replace(',', ''))
    results["metrics"]["total_instructions"] = instructions

    # Parse memory allocations
    alloc_match = re.search(r'total heap usage:\s+([\d,]+)\s+allocs', content)
    if not alloc_match:
        raise ValueError("Could not find total allocations in Gungraun output")
    allocs = int(alloc_match.group(1).replace(',', ''))
    results["metrics"]["total_allocations"] = allocs

    # Parse bytes allocated
    bytes_match = re.search(r'total heap usage:.*?([\d,]+)\s+bytes allocated', content)
    if not bytes_match:
        raise ValueError("Could not find bytes allocated in Gungraun output")
    bytes_allocated = int(bytes_match.group(1).replace(',', ''))
    results["metrics"]["bytes_allocated"] = bytes_allocated
    results["metrics"]["peak_memory_mb"] = round(bytes_allocated / (1024 * 1024), 2)

    # Parse cache references and misses
    cache_refs_match = re.search(r'D\s+refs:\s+([\d,]+)', content)
    if not cache_refs_match:
        raise ValueError("Could not find data references (D refs) in Gungraun output")
    cache_refs = int(cache_refs_match.group(1).replace(',', ''))
    results["metrics"]["cache_references"] = cache_refs

    cache_miss_match = re.search(r'D1\s+misses:\s+([\d,]+)', content)
    if not cache_miss_match:
        raise ValueError("Could not find D1 cache misses in Gungraun output")
    cache_misses = int(cache_miss_match.group(1).replace(',', ''))
    results["metrics"]["cache_misses"] = cache_misses

    # Calculate cache miss rate
    miss_rate = (cache_misses / cache_refs) * 100
    results["metrics"]["cache_miss_rate"] = round(miss_rate, 2)

    return results

def parse_quality_benchmarks(results_file: Path) -> Dict[str, Any]:
    """Parse quality benchmark results."""
    results = {
        "timestamp": datetime.now().isoformat(),
        "type": "quality",
        "metrics": {}
    }

    if not results_file.exists():
        raise FileNotFoundError(f"Quality results file not found: {results_file}")
    
    with open(results_file) as f:
        content = f.read()

    # Parse success rate
    success_match = re.search(r'Success Rate:\s*(\d+\.?\d*)%', content)
    if not success_match:
        raise ValueError("Could not find Success Rate in quality results")
    results["metrics"]["success_rate"] = float(success_match.group(1))

    # Parse average score
    score_match = re.search(r'Average Score:\s*(\d+\.?\d*)', content)
    if not score_match:
        raise ValueError("Could not find Average Score in quality results")
    results["metrics"]["avg_score"] = float(score_match.group(1))

    # Parse total tests
    tests_match = re.search(r'Total Tests:\s*(\d+)', content)
    if not tests_match:
        raise ValueError("Could not find Total Tests in quality results")
    results["metrics"]["total_tests"] = int(tests_match.group(1))

    # Parse passed tests
    passed_match = re.search(r'Passed:\s*(\d+)', content)
    if not passed_match:
        raise ValueError("Could not find Passed tests in quality results")
    results["metrics"]["passed_tests"] = int(passed_match.group(1))

    # Parse failed tests
    failed_match = re.search(r'Failed:\s*(\d+)', content)
    if not failed_match:
        raise ValueError("Could not find Failed tests in quality results")
    results["metrics"]["failed_tests"] = int(failed_match.group(1))

    return results

def merge_with_history(current: Dict[str, Any], history_file: Path, max_history: int = 30) -> Dict[str, Any]:
    """Merge current results with historical data."""
    history = []
    
    # Load existing history
    if history_file.exists():
        try:
            with open(history_file) as f:
                data = json.load(f)
                history = data.get("history", [])
        except Exception as e:
            print(f"Warning: Could not load history from {history_file}: {e}", file=sys.stderr)
    
    # Add current metrics to history
    if current.get("metrics"):
        history_entry = {
            "timestamp": current["timestamp"],
            **current["metrics"]
        }
        history.append(history_entry)
    
    # Keep only last N entries
    history = history[-max_history:]
    
    # Calculate changes from previous run
    if len(history) >= 2:
        prev = history[-2]
        curr = history[-1]
        
        for key in curr.keys():
            if key != "timestamp" and isinstance(curr[key], (int, float)):
                prev_val = prev.get(key, 0)
                if prev_val > 0:
                    change = ((curr[key] - prev_val) / prev_val) * 100
                    current["metrics"][f"{key}_change"] = round(change, 2)
                    current["metrics"][f"prev_{key}"] = prev_val
    
    current["history"] = history
    return current

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="Parse benchmark results and generate JSON")
    parser.add_argument("--criterion-dir", type=Path, default=Path("target/criterion"),
                       help="Path to Criterion output directory")
    parser.add_argument("--gungraun-output", type=Path, default=Path("gungraun-results.md"),
                       help="Path to Gungraun output file")
    parser.add_argument("--quality-results", type=Path, default=Path("quality-results.md"),
                       help="Path to quality benchmark results")
    parser.add_argument("--output-dir", type=Path, default=Path("gh-pages/data"),
                       help="Output directory for JSON files")
    parser.add_argument("--history-dir", type=Path, default=Path(".benchmark-history"),
                       help="Directory for historical data")
    
    args = parser.parse_args()
    
    # Create output directories
    args.output_dir.mkdir(parents=True, exist_ok=True)
    args.history_dir.mkdir(parents=True, exist_ok=True)
    
    print("Parsing benchmark results...")

    # Parse Criterion results
    print("  - Parsing Criterion benchmarks...")
    criterion_results = parse_criterion_results(args.criterion_dir)
    criterion_with_history = merge_with_history(
        criterion_results,
        args.history_dir / "perf-history.json"
    )

    output_file = args.output_dir / "perf-latest.json"
    with open(output_file, 'w') as f:
        json.dump(criterion_with_history, f, indent=2)
    print(f"    [OK] Saved to {output_file}")
    
    # Save history
    with open(args.history_dir / "perf-history.json", 'w') as f:
        json.dump(criterion_with_history, f, indent=2)
    
    # Parse Gungraun results
    print("  - Parsing Gungraun benchmarks...")
    gungraun_results = parse_gungraun_output(args.gungraun_output)
    gungraun_with_history = merge_with_history(
        gungraun_results,
        args.history_dir / "gungraun-history.json"
    )
    
    output_file = args.output_dir / "gungraun-latest.json"
    with open(output_file, 'w') as f:
        json.dump(gungraun_with_history, f, indent=2)
    print(f"    [OK] Saved to {output_file}")
    
    # Save history
    with open(args.history_dir / "gungraun-history.json", 'w') as f:
        json.dump(gungraun_with_history, f, indent=2)
    
    # Parse quality benchmarks
    print("  - Parsing quality benchmarks...")
    quality_results = parse_quality_benchmarks(args.quality_results)
    quality_with_history = merge_with_history(
        quality_results,
        args.history_dir / "quality-history.json"
    )
    
    output_file = args.output_dir / "quality-latest.json"
    with open(output_file, 'w') as f:
        json.dump(quality_with_history, f, indent=2)
    print(f"    [OK] Saved to {output_file}")
    
    # Save history
    with open(args.history_dir / "quality-history.json", 'w') as f:
        json.dump(quality_with_history, f, indent=2)
    
    print("\n[OK] All benchmark results parsed successfully!")
    print(f"Output directory: {args.output_dir}")
    print(f"History directory: {args.history_dir}")

if __name__ == "__main__":
    main()
