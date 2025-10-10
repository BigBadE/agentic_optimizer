#!/usr/bin/env python3
"""
Backfill historical data into latest.json files from gh-pages timestamped data.
"""

import json
import sys
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any

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

def load_timestamped_files(data_dir: Path, bench_type: str, max_history: int = 500) -> List[Dict[str, Any]]:
    """Load timestamped JSON files and extract metrics as history entries."""
    history = []
    bench_dir = data_dir / bench_type

    if not bench_dir.exists():
        print(f"Warning: Directory not found: {bench_dir}", file=sys.stderr)
        return history

    # Find all timestamped JSON files (format: YYYY-MM-DD-HH-MM-SS.json)
    json_files = sorted([f for f in bench_dir.glob("*.json") if f.name != "latest.json"])

    # Load all files first
    for json_file in json_files:
        try:
            with open(json_file) as f:
                data = json.load(f)

            # Create history entry from metrics
            if "metrics" in data and "timestamp" in data:
                entry = {"timestamp": data["timestamp"]}
                entry.update(data["metrics"])
                history.append(entry)
        except Exception as e:
            print(f"Warning: Failed to load {json_file}: {e}", file=sys.stderr)

    # Apply smart downsampling if needed
    if len(history) > max_history:
        history = smart_downsample_history(history, max_history)

    return history

def backfill_history(latest_file: Path, gh_pages_data: Path, bench_type: str):
    """Backfill history into a latest.json file."""
    if not latest_file.exists():
        print(f"Error: File not found: {latest_file}", file=sys.stderr)
        return False

    try:
        # Load current latest.json
        with open(latest_file) as f:
            data = json.load(f)

        # Load historical data from gh-pages
        history = load_timestamped_files(gh_pages_data, bench_type)

        # Create history entry for current data
        if "metrics" in data and "timestamp" in data:
            current_entry = {"timestamp": data["timestamp"]}
            current_entry.update(data["metrics"])

            # Only add if not already in history
            if not history or history[-1]["timestamp"] != current_entry["timestamp"]:
                history.append(current_entry)

        # Add history to data
        data["history"] = history

        # Write back
        with open(latest_file, 'w') as f:
            json.dump(data, f, indent=2)

        print(f"✓ Backfilled {len(history)} history entries into {latest_file}", file=sys.stderr)
        return True

    except Exception as e:
        print(f"Error backfilling {latest_file}: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        return False

def main():
    import argparse

    parser = argparse.ArgumentParser(description="Backfill history into latest.json files")
    parser.add_argument("--gh-pages-data", type=Path, required=True,
                       help="Path to gh-pages data directory")
    parser.add_argument("--benchmark-data", type=Path, default=Path("benchmarks/data"),
                       help="Path to benchmark data directory with latest.json files")
    parser.add_argument("--bench-type", choices=["criterion", "gungraun", "quality", "all"],
                       default="all", help="Which benchmark type to backfill")

    args = parser.parse_args()

    bench_types = ["criterion", "gungraun", "quality"] if args.bench_type == "all" else [args.bench_type]

    success = True
    for bench_type in bench_types:
        latest_file = args.benchmark_data / bench_type / "latest.json"
        if not backfill_history(latest_file, args.gh_pages_data, bench_type):
            success = False

    return 0 if success else 1

if __name__ == "__main__":
    sys.exit(main())
