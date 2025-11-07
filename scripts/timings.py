#!/usr/bin/env python3
"""
Comprehensive timing analysis for fixture tests.

Runs fixture tests with timing instrumentation and generates detailed reports
including per-category breakdowns, hierarchical span timing, and performance metrics.
"""

import subprocess
import sys
import re
import json
import os
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, field
from collections import defaultdict
import argparse

# Fix Windows console encoding issues
if sys.platform == 'win32':
    # Try to enable UTF-8 mode for Windows console
    try:
        sys.stdout.reconfigure(encoding='utf-8')
    except AttributeError:
        # Python < 3.7 doesn't have reconfigure
        import codecs
        sys.stdout = codecs.getwriter('utf-8')(sys.stdout.buffer, 'strict')
    # Set console to UTF-8
    os.system('chcp 65001 >nul 2>&1')


@dataclass
class CategoryStats:
    """Statistics for a single fixture category."""
    name: str
    count: int = 0
    total_duration: float = 0.0
    fixtures: List[Tuple[str, float]] = field(default_factory=list)

    @property
    def avg_duration(self) -> float:
        """Average duration per fixture in this category."""
        return self.total_duration / self.count if self.count > 0 else 0.0


@dataclass
class SpanTiming:
    """Timing data for a traced span."""
    id: int
    name: str
    duration: float
    parent: Optional[int]
    children: List['SpanTiming'] = field(default_factory=list)


class TimingAnalyzer:
    """Analyzes and reports fixture timing data."""

    def __init__(self):
        self.category_stats: Dict[str, CategoryStats] = {}
        self.span_timings: Dict[int, SpanTiming] = {}
        self.root_spans: List[int] = []
        self.raw_output: List[str] = []

    def run_tests(self, with_timing_layer: bool = True) -> bool:
        """
        Run fixture tests with timing instrumentation.

        Returns True if tests passed, False otherwise.
        """
        print("🔧 Running fixture tests with timing instrumentation...")
        print("=" * 80)

        # Build command - use nextest with success-output=immediate to show test output
        cmd = ["cargo", "nextest", "run", "--success-output", "immediate"]

        if with_timing_layer:
            cmd.extend(["--features", "timing-layer"])

        cmd.extend([
            "-p", "integration-tests",
            "test_all_fixtures"
        ])

        # Set environment to enable tracing output
        env = os.environ.copy()
        env["RUST_LOG"] = "info"

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False,
                env=env,
                encoding='utf-8',
                errors='replace'  # Replace invalid UTF-8 characters
            )

            # Combine stdout and stderr since tracing output goes to stderr
            stdout = result.stdout if result.stdout else ""
            stderr = result.stderr if result.stderr else ""
            combined_output = stdout + "\n" + stderr
            self.raw_output = combined_output.split('\n')


            # Check if tests passed
            if result.returncode != 0:
                print("❌ Tests failed!")
                print("\nTest output:")
                print(result.stdout)
                if result.stderr:
                    print("\nErrors:")
                    print(result.stderr)
                return False

            print("✅ Tests passed!")
            return True

        except Exception as e:
            print(f"❌ Error running tests: {e}")
            return False

    def parse_category_breakdown(self):
        """Parse per-category timing breakdown from test output."""
        in_category_section = False

        for line in self.raw_output:
            # Strip tracing prefix (timestamp and level)
            # Format: "2025-11-06T21:58:55.776174Z  INFO fixture_tests: ..."
            if "INFO fixture_tests:" in line:
                line = line.split("INFO fixture_tests:", 1)[1].strip()
            elif "INFO" in line and ":" in line:
                # Generic INFO line stripping
                parts = line.split(":", 2)
                if len(parts) >= 3:
                    line = parts[2].strip()

            # Detect start of category section
            if "Per-Category Timing Breakdown" in line:
                in_category_section = True
                continue

            # Stop at end of category section (=== line)
            if in_category_section and "====" in line:
                break

            # Skip header line
            if in_category_section and line.startswith("Category"):
                continue

            # Skip separator line
            if in_category_section and line.startswith("---"):
                continue

            # Parse category line
            if in_category_section and line.strip():
                # Format: "category_name      count    total_time    avg_time"
                parts = line.split()
                if len(parts) >= 4:
                    try:
                        category = parts[0]
                        count = int(parts[1])
                        total = float(parts[2].rstrip('s'))
                        avg = float(parts[3].rstrip('s'))

                        self.category_stats[category] = CategoryStats(
                            name=category,
                            count=count,
                            total_duration=total
                        )
                    except (ValueError, IndexError):
                        continue

    def parse_hierarchical_timing(self):
        """Parse hierarchical span timing from test output."""
        in_timing_section = False
        span_timings = {}  # name -> list of durations

        for line in self.raw_output:
            # Strip DEBUG prefix if present
            if "DEBUG integration_tests::timing:" in line:
                line = line.split("DEBUG integration_tests::timing:", 1)[1].strip()
            elif "DEBUG" in line and "timing:" in line:
                parts = line.split(":", 2)
                if len(parts) >= 3:
                    line = parts[2].strip()

            # Detect timing report section
            if "=== Timing Report ===" in line:
                in_timing_section = True
                continue

            if in_timing_section and "=====" in line:
                break

            # Parse span timing lines
            if in_timing_section and line.strip():
                # Format: "  span_name: duration" or "span_name: duration"
                match = re.match(r'^(\s*)([^:]+):\s+([\d.]+)s', line)
                if match:
                    name = match.group(2).strip()
                    duration = float(match.group(3))

                    if name not in span_timings:
                        span_timings[name] = []
                    span_timings[name].append(duration)

        return span_timings

    def parse_individual_fixtures(self):
        """Parse individual fixture timing from SLOW markers."""
        for line in self.raw_output:
            # Format: "[SLOW] fixture_name took 1.234s"
            match = re.search(r'\[SLOW\]\s+(\S+)\s+took\s+([\d.]+)s', line)
            if match:
                fixture_name = match.group(1)
                duration = float(match.group(2))

                # Try to extract category from fixture name
                # Fixtures are named like "category_name.json"
                parts = fixture_name.split('_')
                if len(parts) > 1:
                    category = parts[0]
                    if category in self.category_stats:
                        self.category_stats[category].fixtures.append(
                            (fixture_name, duration)
                        )

    def print_summary_report(self, span_timings=None):
        """Print comprehensive timing summary."""
        print("\n" + "=" * 80)
        print("📊 COMPREHENSIVE TIMING REPORT")
        print("=" * 80)

        self._print_category_summary()
        self._print_slow_fixtures()
        self._print_performance_metrics()

        if span_timings:
            self._print_function_timing(span_timings)

    def _print_category_summary(self):
        """Print per-category timing summary."""
        print("\n📁 PER-CATEGORY BREAKDOWN")
        print("-" * 80)
        print(f"{'Category':<20} {'Count':>6} {'Total':>10} {'Average':>10} {'% of Total':>12}")
        print("-" * 80)

        total_time = sum(cat.total_duration for cat in self.category_stats.values())

        # Sort by total duration (descending)
        categories = sorted(
            self.category_stats.values(),
            key=lambda c: c.total_duration,
            reverse=True
        )

        for cat in categories:
            pct = (cat.total_duration / total_time * 100) if total_time > 0 else 0
            print(
                f"{cat.name:<20} {cat.count:>6} "
                f"{cat.total_duration:>9.2f}s {cat.avg_duration:>9.3f}s "
                f"{pct:>11.1f}%"
            )

        print("-" * 80)
        print(f"{'TOTAL':<20} {sum(c.count for c in categories):>6} "
              f"{total_time:>9.2f}s")

    def _print_slow_fixtures(self):
        """Print slowest individual fixtures."""
        print("\n🐌 SLOWEST FIXTURES (>= 1.0s)")
        print("-" * 80)

        # Collect all slow fixtures across categories
        slow_fixtures = []
        for category in self.category_stats.values():
            for fixture, duration in category.fixtures:
                if duration >= 1.0:
                    slow_fixtures.append((category.name, fixture, duration))

        if not slow_fixtures:
            print("No slow fixtures found (all < 1.0s)")
            return

        # Sort by duration (descending)
        slow_fixtures.sort(key=lambda x: x[2], reverse=True)

        print(f"{'Category':<20} {'Fixture':<40} {'Duration':>10}")
        print("-" * 80)

        for category, fixture, duration in slow_fixtures:
            print(f"{category:<20} {fixture:<40} {duration:>9.2f}s")

    def _print_performance_metrics(self):
        """Print overall performance metrics."""
        print("\n⚡ PERFORMANCE METRICS")
        print("-" * 80)

        if not self.category_stats:
            print("No timing data available")
            return

        total_sequential = sum(cat.total_duration for cat in self.category_stats.values())
        total_fixtures = sum(cat.count for cat in self.category_stats.values())

        # Try to extract wall clock time from test output
        wall_clock = None
        for line in self.raw_output:
            # Nextest format: "Summary [   1.865s] 1 test run: 1 passed, 3 skipped"
            match = re.search(r'Summary\s+\[\s*([\d.]+)s\]', line)
            if match:
                wall_clock = float(match.group(1))
                break
            # Fallback: standard test format
            match = re.search(r'finished in ([\d.]+)s', line)
            if match:
                wall_clock = float(match.group(1))
                break

        print(f"Total fixtures:        {total_fixtures}")
        print(f"Sequential time:       {total_sequential:.2f}s")

        if wall_clock:
            print(f"Wall clock time:       {wall_clock:.2f}s")
            speedup = total_sequential / wall_clock if wall_clock > 0 else 0
            print(f"Parallelization:       {speedup:.1f}x speedup")
            print(f"Avg per fixture:       {total_sequential / total_fixtures:.3f}s")

        # Category distribution
        print("\nCategory Distribution:")
        categories = sorted(
            self.category_stats.values(),
            key=lambda c: c.total_duration,
            reverse=True
        )

        for cat in categories[:5]:  # Top 5 categories
            pct = (cat.total_duration / total_sequential * 100)
            print(f"  • {cat.name:<18} {pct:>5.1f}% ({cat.avg_duration:.3f}s avg)")

    def _print_function_timing(self, span_timings: Dict[str, List[float]]):
        """Print function-level timing breakdown."""
        print("\n🔍 FUNCTION-LEVEL TIMING")
        print("-" * 80)

        # Aggregate spans by function
        aggregated = {}
        for name, durations in span_timings.items():
            total = sum(durations)
            count = len(durations)
            avg = total / count if count > 0 else 0
            aggregated[name] = {
                'total': total,
                'count': count,
                'avg': avg
            }

        # Sort by total time descending
        sorted_funcs = sorted(
            aggregated.items(),
            key=lambda x: x[1]['total'],
            reverse=True
        )

        print(f"{'Function':<40} {'Calls':>8} {'Total':>10} {'Average':>10}")
        print("-" * 80)

        for func_name, stats in sorted_funcs[:15]:  # Top 15 functions
            print(
                f"{func_name:<40} {stats['count']:>8} "
                f"{stats['total']:>9.3f}s {stats['avg']:>9.3f}s"
            )

    def export_json(self, output_path: Path):
        """Export timing data to JSON for further analysis."""
        data = {
            "categories": {
                name: {
                    "count": cat.count,
                    "total_duration": cat.total_duration,
                    "avg_duration": cat.avg_duration,
                    "fixtures": [
                        {"name": fname, "duration": dur}
                        for fname, dur in cat.fixtures
                    ]
                }
                for name, cat in self.category_stats.items()
            }
        }

        with open(output_path, 'w') as f:
            json.dump(data, f, indent=2)

        print(f"\n💾 Timing data exported to: {output_path}")


def generate_flamegraph():
    """Generate flamegraph if available."""
    print("\n🔥 FLAMEGRAPH GENERATION")
    print("-" * 80)

    # Check if flamegraph script exists
    flamegraph_script = Path("scripts/profile_flamegraph.sh")
    if not flamegraph_script.exists():
        print("❌ Flamegraph script not found at scripts/profile_flamegraph.sh")
        return False

    print("Generating flamegraph (this may take a while)...")
    print("Note: On Windows, this requires WSL or may not work.")

    try:
        result = subprocess.run(
            ["bash", str(flamegraph_script)],
            capture_output=True,
            text=True,
            timeout=300000,  # 5 minutes
            encoding='utf-8',
            errors='replace'
        )

        if result.returncode == 0:
            print("✅ Flamegraph generated successfully!")
            print("   Check the output directory for flamegraph.svg")
            return True
        else:
            print(f"⚠️  Flamegraph generation failed (exit code {result.returncode})")
            if result.stderr:
                print(f"   Error: {result.stderr[:200]}")
            return False

    except subprocess.TimeoutExpired:
        print("⚠️  Flamegraph generation timed out after 5 minutes")
        return False
    except Exception as e:
        print(f"⚠️  Error generating flamegraph: {e}")
        return False


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Run fixture tests with comprehensive timing analysis"
    )
    parser.add_argument(
        "--export",
        type=Path,
        help="Export timing data to JSON file"
    )
    parser.add_argument(
        "--flamegraph",
        action="store_true",
        help="Generate flamegraph profile (requires Linux/WSL)"
    )

    args = parser.parse_args()

    analyzer = TimingAnalyzer()

    # Run tests - always enable timing layer to get tracing output
    if not analyzer.run_tests(with_timing_layer=True):
        sys.exit(1)

    # Parse timing data
    print("\n🔍 Analyzing timing data...")
    analyzer.parse_category_breakdown()
    analyzer.parse_individual_fixtures()
    span_timings = analyzer.parse_hierarchical_timing()

    # Generate report
    analyzer.print_summary_report(span_timings=span_timings)

    # Export if requested
    if args.export:
        analyzer.export_json(args.export)

    # Generate flamegraph if requested
    if args.flamegraph:
        generate_flamegraph()

    print("\n✅ Timing analysis complete!")


if __name__ == "__main__":
    main()
