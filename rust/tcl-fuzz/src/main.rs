//! `tcl-fuzz` — differential fuzzer for the native Tcl VM.
//!
//! Generates pure, bounded Tcl over the VM's supported surface, runs each
//! script through both `tclvm` (subject) and `tclsh` (reference), and records
//! any divergence in stdout or error status. See the module docs for the
//! generator scope and the comparison rules.

#![forbid(unsafe_code)]

mod campaign;
mod findings;
mod generator;
mod harness;
mod rng;

use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Parser, Subcommand};

use campaign::{Campaign, Stats};
use findings::Registry;
use generator::{generate, GenConfig};
use harness::{compare, run_backend, write_script, Outcome};

/// Differential fuzzer for the native Tcl bytecode VM.
#[derive(Parser)]
#[command(name = "tcl-fuzz", version, about)]
struct Cli {
    /// Path to the `tclvm` (subject) binary. Defaults to one beside this
    /// executable, else `tclvm` on `PATH`.
    #[arg(long, global = true)]
    tclvm: Option<PathBuf>,
    /// Path to the reference `tclsh`. Defaults to `tclsh9.0`, else `tclsh`.
    #[arg(long, global = true)]
    tclsh: Option<PathBuf>,
    /// Findings directory.
    #[arg(long, global = true, default_value = "fuzz-findings")]
    findings: PathBuf,
    /// Per-script timeout, in milliseconds.
    #[arg(long, global = true, default_value_t = 5000)]
    timeout_ms: u64,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a fuzzing campaign.
    Run {
        /// Number of scripts to generate and test.
        #[arg(long, default_value_t = 1000)]
        iterations: u64,
        /// Starting seed (each iteration uses seed + i). Defaults to a
        /// time-based seed.
        #[arg(long)]
        seed: Option<u64>,
        /// Print each finding's seed as it is discovered.
        #[arg(long)]
        verbose: bool,
    },
    /// Replay a single seed and print both engines' output side by side.
    Replay {
        /// The seed to reproduce.
        seed: u64,
    },
    /// Print a summary of the findings registry.
    Summary,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let timeout = Duration::from_millis(cli.timeout_ms);
    let config = GenConfig::default();

    match &cli.command {
        Cmd::Run { iterations, seed, verbose } => {
            let Some(tclvm) = resolve_tclvm(cli.tclvm.as_deref()) else {
                eprintln!("error: could not find `tclvm` (pass --tclvm <path>)");
                return std::process::ExitCode::from(2);
            };
            let tclsh = resolve_tclsh(cli.tclsh.as_deref());
            let registry = match Registry::open(&cli.findings) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: opening findings dir: {e}");
                    return std::process::ExitCode::from(2);
                }
            };
            let scratch = std::env::temp_dir().join(format!("tcl-fuzz-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&scratch);
            let base_seed = seed.unwrap_or_else(time_seed);

            eprintln!(
                "campaign: {iterations} iterations from seed {base_seed}\n  subject: {}\n  reference: {}",
                tclvm.display(),
                tclsh.display(),
            );
            let campaign = Campaign {
                tclsh: &tclsh,
                tclvm: &tclvm,
                timeout,
                config,
                registry: &registry,
                scratch: scratch.clone(),
            };
            let mut last_findings = 0u64;
            let stats = campaign.run(base_seed, *iterations, |i, stats| {
                if *verbose && stats.findings() > last_findings {
                    eprintln!("  finding @ iteration {i}");
                    last_findings = stats.findings();
                } else if i % 100 == 0 {
                    eprintln!("  {i}/{iterations} — {} findings", stats.findings());
                }
            });
            let _ = std::fs::remove_dir_all(&scratch);
            print_stats(&stats);
            // Exit non-zero when a divergence was found, so CI can gate on it.
            if stats.findings() > 0 {
                std::process::ExitCode::from(1)
            } else {
                std::process::ExitCode::SUCCESS
            }
        }
        Cmd::Replay { seed } => {
            let Some(tclvm) = resolve_tclvm(cli.tclvm.as_deref()) else {
                eprintln!("error: could not find `tclvm` (pass --tclvm <path>)");
                return std::process::ExitCode::from(2);
            };
            let tclsh = resolve_tclsh(cli.tclsh.as_deref());
            if let Ok(registry) = Registry::open(&cli.findings)
                && let Some(prior) = registry.load(*seed)
            {
                eprintln!("note: seed {seed} is a recorded finding ({:?})", prior.category);
            }
            replay(*seed, &config, &tclvm, &tclsh, timeout);
            std::process::ExitCode::SUCCESS
        }
        Cmd::Summary => {
            match Registry::open(&cli.findings) {
                Ok(registry) => {
                    let summary = registry.summary();
                    if summary.is_empty() {
                        println!("no findings in {}", cli.findings.display());
                    } else {
                        println!("findings in {}:", cli.findings.display());
                        for (cat, n) in summary {
                            println!("  {cat:?}: {n}");
                        }
                    }
                    std::process::ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::ExitCode::from(2)
                }
            }
        }
    }
}

/// Regenerate `seed`, run both engines, and print a side-by-side comparison.
fn replay(seed: u64, config: &GenConfig, tclvm: &Path, tclsh: &Path, timeout: Duration) {
    let script = generate(seed, config);
    println!("=== script (seed {seed}) ===\n{script}");
    let scratch = std::env::temp_dir();
    let Ok(path) = write_script(&scratch, seed, &script) else {
        eprintln!("error: could not write scratch script");
        return;
    };
    let reference = run_backend(tclsh, &path, timeout);
    let subject = run_backend(tclvm, &path, timeout);
    let _ = std::fs::remove_file(&path);
    println!("=== tclsh (reference) ===\n{}", render(&reference));
    println!("=== tclvm (subject) ===\n{}", render(&subject));
    println!("=== verdict: {:?} ===", compare(&reference, &subject));
}

fn render(o: &Outcome) -> String {
    match o {
        Outcome::Ran { stdout, errored } => {
            format!("[errored={errored}]\n{stdout}")
        }
        Outcome::Timeout => "<timeout>".to_owned(),
        Outcome::Unavailable(m) => format!("<unavailable: {m}>"),
    }
}

fn print_stats(stats: &Stats) {
    eprintln!(
        "\ncampaign complete:\n  total {} | matched {} | skipped {}\n  findings: stdout {} | status {} | timeout {} (new this run: {})",
        stats.total,
        stats.matched,
        stats.skipped,
        stats.stdout_mismatch,
        stats.status_mismatch,
        stats.timeout,
        stats.new_findings,
    );
}

/// Locate the `tclvm` binary: explicit flag, then beside this executable, then
/// `PATH`.
fn resolve_tclvm(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return p.is_file().then(|| p.to_owned());
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let beside = dir.join("tclvm");
        if beside.is_file() {
            return Some(beside);
        }
    }
    which("tclvm")
}

/// Locate the reference `tclsh`: explicit flag, then `tclsh9.0`, then `tclsh`.
fn resolve_tclsh(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        return p.to_owned();
    }
    which("tclsh9.0")
        .or_else(|| which("tclsh"))
        .unwrap_or_else(|| PathBuf::from("tclsh"))
}

/// Find an executable on `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

/// A coarse time-based seed for ad-hoc campaigns. The nanosecond count is
/// folded into 64 bits — any value works as a seed.
fn time_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_nanos() & u128::from(u64::MAX)).ok())
        .unwrap_or(0x1234_5678)
}
