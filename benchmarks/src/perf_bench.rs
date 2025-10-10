//! Performance benchmark runner CLI.
#![allow(
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::tests_outside_test_module,
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::collapsible_if,
    clippy::items_after_statements,
    clippy::absolute_paths,
    clippy::min_ident_chars,
    clippy::excessive_nesting,
    reason = "Test allows"
)]

use anyhow::{Context as _, Result};
use chrono::Local;
use clap::Parser;
use serde_json::Value as JsonValue;
use std::fmt::Write as _;
use std::fs::{self, write};
use std::path::PathBuf;
use std::process::{Command, Output};

#[derive(Parser)]
#[command(name = "perf-bench")]
#[command(about = "Run performance benchmarks locally", long_about = None)]
struct Args {
    /// Output file for results (markdown format)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Package to benchmark (default: merlin-routing)
    #[arg(short, long, default_value = "merlin-routing")]
    package: String,

    /// Specific benchmark to run
    #[arg(short = 'n', long)]
    name: Option<String>,

    /// Run IAI benchmarks instead of Criterion
    #[arg(long)]
    iai: bool,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let bench_type = if args.iai {
        "IAI-Callgrind"
    } else {
        "Criterion"
    };
    println!("Running {bench_type} performance benchmarks...");
    println!("Package: {}", args.package);
    println!();

    let mut cmd = Command::new("cargo");
    cmd.arg("bench")
        .arg("-p")
        .arg(&args.package)
        .arg("--no-fail-fast");

    if let Some(name) = &args.name {
        cmd.arg("--bench").arg(name);
    } else if args.iai {
        // Run only IAI benchmarks
        cmd.arg("iai_");
    } else {
        // Run all non-gungraun benchmarks individually
        // gungraun benchmarks are Linux/Valgrind only, run in CI
        cmd.arg("--bench")
            .arg("cache_benchmarks")
            .arg("--bench")
            .arg("integration_benchmarks")
            .arg("--bench")
            .arg("metrics_benchmarks")
            .arg("--bench")
            .arg("routing_benchmarks");
    }

    if args.verbose {
        println!("Running command: {cmd:?}");
    }

    let output = cmd.output().context("Failed to run cargo bench")?;

    if !output.status.success() {
        eprintln!("Benchmark failed:");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        anyhow::bail!("Benchmark execution failed");
    }

    if args.verbose {
        println!("Benchmark output:");
        println!("{}", String::from_utf8_lossy(&output.stdout));
    }

    let report = if args.iai {
        generate_iai_report(&output)?
    } else {
        let criterion_dir = PathBuf::from("target/criterion");
        if !criterion_dir.exists() {
            anyhow::bail!(
                "Criterion results directory not found at {}",
                criterion_dir.display()
            );
        }
        generate_criterion_report(&criterion_dir)?
    };

    if let Some(output_path) = &args.output {
        let display = output_path.display();
        write(output_path, &report)
            .with_context(|| format!("Failed to write report to {display}"))?;
        println!("Report written to: {display}");
        println!();
        println!("To upload results:");
        println!("  git add -f {display}");
        println!("  git commit -m \"Update performance benchmark results\"");
        println!("  git push");
    } else {
        println!("{report}");
    }

    Ok(())
}

fn generate_criterion_report(criterion_dir: &PathBuf) -> Result<String> {
    let mut report = String::from("# Performance Benchmark Results\n\n");
    writeln!(
        &mut report,
        "**Date**: {}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    )
    .map_err(|err| anyhow::anyhow!("Failed to write to string: {err}"))?;

    #[derive(Debug)]
    struct BenchResult {
        name: String,
        mean_ns: f64,
        std_dev_ns: f64,
        median_ns: f64,
    }

    // Collect benchmark results with detailed timing data
    let mut results = Vec::new();
    let mut total_time_ns: f64 = 0.0;

    for entry in fs::read_dir(criterion_dir).context("Failed to read criterion directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_dir()
            && path.file_name().is_some_and(|name| name != "report")
            && let Some(name) = path.file_name()
        {
            let group_name = name.to_string_lossy().to_string();

            // First try direct path (single benchmark)
            let direct_estimates = path.join("base").join("estimates.json");
            if direct_estimates.exists() {
                if let Ok(estimates_content) = fs::read_to_string(&direct_estimates) {
                    if let Ok(estimates) = serde_json::from_str::<JsonValue>(&estimates_content) {
                        let mean_ns = estimates
                            .get("mean")
                            .and_then(|mean_val| mean_val.get("point_estimate"))
                            .and_then(JsonValue::as_f64)
                            .unwrap_or(0.0);

                        let std_dev_ns = estimates
                            .get("std_dev")
                            .and_then(|std_val| std_val.get("point_estimate"))
                            .and_then(JsonValue::as_f64)
                            .unwrap_or(0.0);

                        let median_ns = estimates
                            .get("median")
                            .and_then(|median_val| median_val.get("point_estimate"))
                            .and_then(JsonValue::as_f64)
                            .unwrap_or(mean_ns);

                        total_time_ns += mean_ns;
                        results.push(BenchResult {
                            name: group_name,
                            mean_ns,
                            std_dev_ns,
                            median_ns,
                        });
                        continue;
                    }
                }
            }

            // Try nested structure (sub-benchmarks) - show each individually
            if let Ok(sub_entries) = fs::read_dir(&path) {
                let mut sub_results: Vec<_> = sub_entries
                    .filter_map(Result::ok)
                    .filter_map(|sub_entry| {
                        let sub_path = sub_entry.path();
                        if !sub_path.is_dir() {
                            return None;
                        }

                        let sub_name = sub_path.file_name()?.to_string_lossy().to_string();
                        let estimates_path = sub_path.join("base").join("estimates.json");

                        if !estimates_path.exists() {
                            return None;
                        }

                        let estimates_content = fs::read_to_string(&estimates_path).ok()?;
                        let estimates: JsonValue = serde_json::from_str(&estimates_content).ok()?;

                        let mean_ns = estimates.get("mean")?.get("point_estimate")?.as_f64()?;

                        let std_dev_ns = estimates
                            .get("std_dev")
                            .and_then(|std_val| std_val.get("point_estimate"))
                            .and_then(JsonValue::as_f64)
                            .unwrap_or(0.0);

                        let median_ns = estimates
                            .get("median")
                            .and_then(|median_val| median_val.get("point_estimate"))
                            .and_then(JsonValue::as_f64)
                            .unwrap_or(mean_ns);

                        Some(BenchResult {
                            name: format!("{group_name}/{sub_name}"),
                            mean_ns,
                            std_dev_ns,
                            median_ns,
                        })
                    })
                    .collect();

                if !sub_results.is_empty() {
                    for result in &sub_results {
                        total_time_ns += result.mean_ns;
                    }
                    results.append(&mut sub_results);
                }
            }
        }
    }

    results.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    report.push_str("## Summary\n\n");
    writeln!(&mut report, "**Total Benchmarks**: {}\n", results.len())
        .map_err(|err| anyhow::anyhow!("Failed to write to string: {err}"))?;

    if !results.is_empty() && total_time_ns > 0.0 {
        let avg_time_ms = (total_time_ns / results.len() as f64) / 1_000_000.0;
        writeln!(&mut report, "**Average Time**: {avg_time_ms:.3} ms\n")
            .map_err(|err| anyhow::anyhow!("Failed to write to string: {err}"))?;
    }

    report.push_str("## Benchmark Results\n\n");
    report.push_str("| Benchmark | Mean | Median | Std Dev |\n");
    report.push_str("|-----------|------|--------|--------|\n");

    for result in &results {
        if result.mean_ns > 0.0 {
            let format_time = |nanoseconds: f64| {
                let milliseconds = nanoseconds / 1_000_000.0;
                let microseconds = nanoseconds / 1_000.0;
                if milliseconds >= 1.0 {
                    format!("{milliseconds:.3} ms")
                } else if microseconds >= 1.0 {
                    format!("{microseconds:.3} μs")
                } else {
                    format!("{nanoseconds:.1} ns")
                }
            };

            let mean_str = format_time(result.mean_ns);
            let median_str = format_time(result.median_ns);
            let std_dev_str = format_time(result.std_dev_ns);

            writeln!(
                &mut report,
                "| `{}` | {} | {} | {} |",
                result.name, mean_str, median_str, std_dev_str
            )
            .map_err(|err| anyhow::anyhow!("Failed to write to string: {err}"))?;
        } else {
            writeln!(&mut report, "| `{}` | N/A | N/A | N/A |", result.name)
                .map_err(|err| anyhow::anyhow!("Failed to write to string: {err}"))?;
        }
    }

    report.push_str("\n## Viewing Results\n\n");
    report.push_str("To view detailed HTML reports:\n");
    report.push_str("```bash\n");
    report.push_str("# Open the main report\n");
    report.push_str("open target/criterion/report/index.html\n");
    report.push_str("```\n\n");

    report.push_str("## Raw Data\n\n");
    writeln!(
        &mut report,
        "Full benchmark data is stored in `target/criterion/` ({} benchmarks)",
        results.len()
    )
    .map_err(|err| anyhow::anyhow!("Failed to write to string: {err}"))?;

    Ok(report)
}

fn generate_iai_report(output: &Output) -> Result<String> {
    let mut report = String::from("# IAI-Callgrind Performance Benchmark Results\n\n");
    writeln!(
        &mut report,
        "**Date**: {}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    )
    .map_err(|err| anyhow::anyhow!("Failed to write to string: {err}"))?;

    report.push_str("## Summary\n\n");
    report.push_str(
        "IAI-Callgrind benchmarks use Valgrind's Callgrind to provide precise, deterministic measurements.\n\n",
    );

    report.push_str("## Benchmark Results\n\n");
    report.push_str("```\n");
    report.push_str(&String::from_utf8_lossy(&output.stdout));
    report.push_str("```\n\n");

    report.push_str("## Metrics Explained\n\n");
    report.push_str("- **Instructions**: Total CPU instructions executed\n");
    report.push_str("- **L1 Accesses**: Level 1 cache accesses\n");
    report.push_str("- **L2 Accesses**: Level 2 cache accesses\n");
    report.push_str("- **RAM Accesses**: Main memory accesses\n");
    report.push_str("- **Estimated Cycles**: Estimated CPU cycles (lower is better)\n\n");

    report.push_str("## Viewing Detailed Results\n\n");
    report.push_str("Callgrind output files are stored in `target/iai/`.\n");
    report
        .push_str("You can analyze them with tools like `callgrind_annotate` or `kcachegrind`.\n");

    Ok(report)
}
