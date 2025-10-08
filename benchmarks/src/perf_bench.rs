//! Performance benchmark runner CLI.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    reason = "CLI binary uses stdout/stderr for output"
)]

use anyhow::{Context as _, Result};
use chrono::Local;
use clap::Parser;
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
        .arg("--benches")
        .arg("--no-fail-fast");

    if let Some(name) = &args.name {
        cmd.arg("--").arg(name);
    } else if args.iai {
        // Run only IAI benchmarks
        cmd.arg("iai_");
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
        write(output_path, &report)
            .with_context(|| format!("Failed to write report to {}", output_path.display()))?;
        println!("Report written to: {}", output_path.display());
        println!();
        println!("To upload results:");
        println!("  git add -f {}", output_path.display());
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

    report.push_str("## Summary\n\n");
    report.push_str("Benchmarks completed successfully. Detailed results are available in the Criterion output.\n\n");

    report.push_str("## Benchmark Groups\n\n");

    let mut groups = Vec::new();
    for entry in fs::read_dir(criterion_dir).context("Failed to read criterion directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.is_dir()
            && path.file_name().is_some_and(|name| name != "report")
            && let Some(name) = path.file_name()
        {
            groups.push(name.to_string_lossy().to_string());
        }
    }

    groups.sort();

    for group in &groups {
        writeln!(&mut report, "- `{group}`")
            .map_err(|err| anyhow::anyhow!("Failed to write to string: {err}"))?;
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
        "Full benchmark data is stored in `target/criterion/` ({} groups)",
        groups.len()
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
