// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `tcl-fuzz` — differential fuzzer for the native Tcl VM and related
//! backends.
//!
//! Generates pure, bounded Tcl over the generator's supported surface, runs
//! each script through a pair of backend engines, and records any divergence
//! in stdout, error status, or (opt-in) error message text. The `run`
//! subcommand pairs any two [`engine::Engine`]s — `tclvm`/`tclsh` (the
//! original, and still the default), `runtime-rust`/`tclsh`, or
//! `tclvm`/`runtime-rust` (issue #1313) — over the same subprocess harness.
//! See the module docs for the generator scope and the comparison rules.

#![forbid(unsafe_code)]

mod bpf_diff;
mod campaign;
mod characterize;
mod engine;
mod findings;
mod generator;
mod harness;
mod linked_wasm;
mod rng;
mod version;
mod wasm;
mod wasm_diff;

use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Parser, Subcommand};

use campaign::{Backend, Campaign, Stats};
use characterize::Classification;
use engine::Engine;
use findings::{Registry, ReproducerStore};
use generator::{GenConfig, generate};
use harness::{Outcome, compare_outcomes, run_backend, write_script};

/// Differential fuzzer for the native Tcl bytecode VM and related backends.
#[derive(Parser)]
#[command(name = "tcl-fuzz", version, about)]
struct Cli {
    /// Path to the `tclvm` binary. Defaults to one beside this executable,
    /// else `tclvm` on `PATH`.
    #[arg(long, global = true)]
    tclvm: Option<PathBuf>,
    /// Path to a reference `tclsh`. Defaults to `tclsh9.0`, else `tclsh`.
    #[arg(long, global = true)]
    tclsh: Option<PathBuf>,
    /// Path to `runtime/rust`'s `run_script` dev-tool example binary (build
    /// with `cargo build --release --example run_script` under `runtime/rust`).
    /// Defaults to one beside this executable, else `run_script` on `PATH`.
    #[arg(long, global = true)]
    runtime_rust: Option<PathBuf>,
    /// Findings directory. A non-default `run --reference/--subject` pair is
    /// namespaced under `<findings>/<subject>-vs-<reference>/` so different
    /// backend pairs never collide on the same seed.
    #[arg(long, global = true, default_value = "fuzz-findings")]
    findings: PathBuf,
    /// Per-script timeout, in milliseconds.
    #[arg(long, global = true, default_value_t = 5000)]
    timeout_ms: u64,
    /// How often a generated top-level statement is a deliberately malformed
    /// *expression* (`1+`, `1 2`, `foo bar baz`, an unbalanced paren, …), in
    /// parts per thousand. `0` opts out entirely and restores the exact
    /// pre-existing generator stream, so a registry's historical findings
    /// still replay. Global, not `run`-only: `replay` has to generate a seed
    /// the same way the campaign that recorded it did.
    #[arg(long, global = true, default_value_t = GenConfig::default().malformed_expr_permille)]
    malformed_expr_permille: u32,
    #[command(subcommand)]
    command: Cmd,
}

/// Which two engines a command operates on. Shared by every pair-aware
/// subcommand so `run`, `replay`, and `summary` name a pair the same way and
/// land on the same (pair-namespaced) registry.
#[derive(clap::Args, Clone, Copy)]
struct PairArgs {
    /// The reference (presumed-correct) engine.
    #[arg(long, value_enum, default_value = "tclsh")]
    reference: Engine,
    /// The subject (implementation under test) engine.
    #[arg(long, value_enum, default_value = "tclvm")]
    subject: Engine,
}

/// Everything one `run` campaign is configured by, beyond the global flags.
#[derive(clap::Args)]
struct RunArgs {
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
    #[command(flatten)]
    pair: PairArgs,
    /// Additionally compare error message text when both engines error
    /// (default: only whether each errored is compared — see
    /// `harness::compare_outcomes`).
    #[arg(long)]
    compare_error_text: bool,
    /// Pin the Tcl release the **subject** engine emulates (`8.4`…`9.1`), so
    /// it can be matched to an older reference `tclsh`.
    ///
    /// `runtime-rust` and `tclvm` accept `--tcl-version`; pinning any other
    /// subject is refused rather than silently ignored. Without this, a
    /// campaign against e.g. `tclsh8.6` records
    /// every deliberate 8.6-vs-9.0 semantic change as a divergence — which is
    /// exactly how issue #1328's eight "findings" were produced.
    #[arg(long, value_name = "X.Y")]
    subject_tcl_version: Option<String>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a fuzzing campaign over one backend pair.
    Run(RunArgs),
    /// Replay a single seed and print both engines' output side by side.
    Replay {
        /// The seed to reproduce.
        seed: u64,
        /// The engine pair — must match the pair that recorded the finding,
        /// if replaying one from the registry.
        #[command(flatten)]
        pair: PairArgs,
    },
    /// Print a summary of a backend pair's findings registry.
    Summary {
        /// The engine pair whose registry to summarise.
        #[command(flatten)]
        pair: PairArgs,
    },
    /// Characterise `tcl-vm` and `runtime/rust` against C Tcl 9. This is a
    /// three-way campaign: agreement between the two Rust backends alone is
    /// never treated as correctness.
    Characterise {
        /// Number of scripts to generate and classify.
        #[arg(long, default_value_t = 1000)]
        iterations: u64,
        /// Starting seed (each iteration uses seed + i).
        #[arg(long)]
        seed: Option<u64>,
        /// Print each divergent seed as it is discovered.
        #[arg(long)]
        verbose: bool,
        /// Additionally compare error text when every backend errors.
        #[arg(long)]
        compare_error_text: bool,
    },
    /// WASM-runnability arm: compile each generated program to the
    /// eval-fallback WASM module and run it under `wasmtime`, flagging codegen
    /// panics/errors and modules that fail to instantiate or trap. (The value
    /// differential against `tclsh` is gated on the interpreter-backed host;
    /// this exercises the WASM codegen for crashes/traps.)
    WasmCheck {
        /// Number of scripts to generate and check.
        #[arg(long, default_value_t = 200)]
        iterations: u64,
        /// Starting seed (each iteration uses seed + i).
        #[arg(long)]
        seed: Option<u64>,
        /// Print each finding's seed and reason as it is discovered.
        #[arg(long)]
        verbose: bool,
    },
    /// WASM **value**-differential arm: drive each generated program's compiled
    /// WASM control flow with an embedded `tcl-vm` host and compare its output
    /// against running the program directly on `tcl-vm`. A divergence isolates a
    /// WASM control-flow miscompile (commands are `tcl-vm` on both sides).
    WasmDiff {
        /// Number of scripts to generate and check.
        #[arg(long, default_value_t = 200)]
        iterations: u64,
        /// Starting seed (each iteration uses seed + i).
        #[arg(long)]
        seed: Option<u64>,
        /// Print each finding's seed and reason as it is discovered.
        #[arg(long)]
        verbose: bool,
    },
    /// eBPF value-differential arm: generate a registry-described BPF-Tcl
    /// socket filter, compare its direct dialect contract with the real
    /// lowering, eBPF emitter, and userspace eBPF VM.
    BpfDiff {
        /// Number of bounded packet-read programs to generate and check.
        #[arg(long, default_value_t = 200)]
        iterations: u64,
        /// Starting seed (each iteration uses seed + i).
        #[arg(long)]
        seed: Option<u64>,
        /// Print every mismatch or backend error as it occurs.
        #[arg(long)]
        verbose: bool,
    },
    /// Real linked-WASM differential arm: compile a generated user module,
    /// link it against `runtime/rust`'s WASM module, and compare the result
    /// against C Tcl 9 running the identical program.
    LinkedWasmDiff {
        /// Number of pure linked-runtime programs to generate and check.
        #[arg(long, default_value_t = 50)]
        iterations: u64,
        /// Starting seed (each iteration uses seed + i).
        #[arg(long)]
        seed: Option<u64>,
        /// Print every mismatch or backend error as it occurs.
        #[arg(long)]
        verbose: bool,
    },
}

/// A [`PairArgs`] with both engines resolved to a concrete `(binary, leading
/// args)` invocation, so the campaign and replay paths never re-resolve.
struct ResolvedPair {
    reference: Engine,
    subject: Engine,
    reference_bin: PathBuf,
    reference_args: Vec<String>,
    subject_bin: PathBuf,
    subject_args: Vec<String>,
}

impl ResolvedPair {
    /// Resolve both engines' binaries, honouring the matching
    /// `--tclvm`/`--tclsh`/`--runtime-rust` override. `Err` carries a ready-to-print
    /// message naming the engine that could not be found and the flag to pass.
    fn resolve(pair: PairArgs, cli: &Cli) -> Result<Self, String> {
        if pair.reference == pair.subject {
            return Err(format!(
                "reference and subject are both `{}`; select two different backends",
                pair.reference.label()
            ));
        }
        let find = |engine: Engine| {
            resolve_engine(engine, cli).ok_or_else(|| {
                format!(
                    "could not find `{}` (pass --{})",
                    engine.label(),
                    cli_flag_for(engine),
                )
            })
        };
        let (reference_bin, reference_args) = find(pair.reference)?;
        let (subject_bin, subject_args) = find(pair.subject)?;
        Ok(Self {
            reference: pair.reference,
            subject: pair.subject,
            reference_bin,
            reference_args,
            subject_bin,
            subject_args,
        })
    }

    /// This pair's findings directory under `base` — see [`pair_findings_dir`].
    fn findings_dir(&self, base: &Path) -> PathBuf {
        pair_findings_dir(base, self.reference, self.subject)
    }
}

/// Probe both resolved backends before a run or replay.
///
/// A path that exists but cannot execute Tcl must not turn every iteration
/// into a skipped outcome and produce a misleading green campaign.
fn probe_pair_versions(
    pair: &ResolvedPair,
    timeout: Duration,
) -> Result<version::PairVersions, String> {
    let probe = |engine: Engine, binary: &Path, args: &[String]| {
        version::EngineVersion::probe(binary, args, timeout).ok_or_else(|| {
            format!(
                "could not validate `{}` at {}; the backend must execute `[info patchlevel]`",
                engine.label(),
                binary.display(),
            )
        })
    };
    Ok(version::PairVersions {
        reference: Some(probe(
            pair.reference,
            &pair.reference_bin,
            &pair.reference_args,
        )?),
        subject: Some(probe(pair.subject, &pair.subject_bin, &pair.subject_args)?),
    })
}

/// Run one `Cmd::Run` campaign: resolve both engines, open the (pair-namespaced)
/// findings registry, drive the campaign, and report the exit code.
fn run_campaign(
    cli: &Cli,
    config: &GenConfig,
    timeout: Duration,
    args: &RunArgs,
) -> std::process::ExitCode {
    let pair = match ResolvedPair::resolve(args.pair, cli) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    // Pin the subject's emulated release, when asked. Refused (not ignored)
    // for an engine that cannot be pinned, so a campaign never silently runs
    // version-skewed after being told not to.
    let mut pair = pair;
    if let Some(want) = &args.subject_tcl_version {
        match version_flag_for(pair.subject, want) {
            Ok(extra) => pair.subject_args.extend(extra),
            Err(e) => {
                eprintln!("error: {e}");
                return std::process::ExitCode::from(2);
            }
        }
    }
    let findings_dir = pair.findings_dir(&cli.findings);
    let registry = match Registry::open(&findings_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: opening findings dir: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    let scratch = std::env::temp_dir().join(format!("tcl-fuzz-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&scratch);
    let base_seed = args.seed.unwrap_or_else(time_seed);
    let iterations = args.iterations;

    eprintln!(
        "campaign: {iterations} iterations from seed {base_seed}\n  subject: {} ({})\n  reference: {} ({})\n  findings: {}",
        pair.subject.label(),
        pair.subject_bin.display(),
        pair.reference.label(),
        pair.reference_bin.display(),
        findings_dir.display(),
    );
    // Probe both engines once, before any script runs, so every finding this
    // campaign records carries the releases it was produced against, and a
    // skewed pair announces itself up front (issue #1328).
    let versions = match probe_pair_versions(&pair, timeout) {
        Ok(versions) => versions,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&scratch);
            eprintln!("error: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let describe = |version: &Option<version::EngineVersion>| {
        version.as_ref().map_or_else(
            || "unknown".to_owned(),
            |version| version.patchlevel.clone(),
        )
    };
    eprintln!(
        "  versions: reference Tcl {} | subject Tcl {}",
        describe(&versions.reference),
        describe(&versions.subject),
    );
    if let Some(warning) = versions.skew_warning() {
        eprintln!("{warning}");
    }
    let campaign = Campaign {
        reference: Backend {
            engine: pair.reference,
            binary: &pair.reference_bin,
            args: pair.reference_args.clone(),
        },
        subject: Backend {
            engine: pair.subject,
            binary: &pair.subject_bin,
            args: pair.subject_args.clone(),
        },
        timeout,
        config: *config,
        registry: &registry,
        scratch: scratch.clone(),
        compare_error_text: args.compare_error_text,
        versions,
    };
    let mut last_findings = 0u64;
    let stats = campaign.run(base_seed, iterations, |i, stats| {
        if args.verbose && stats.findings() > last_findings {
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

/// Whether replaying under `versions` differs from the versions persisted with
/// a finding. Kept separate so version-warning decisions stay testable without
/// invoking either backend.
fn replay_versions_changed(
    recorded_reference: Option<&str>,
    recorded_subject: Option<&str>,
    versions: &version::PairVersions,
) -> bool {
    let current_reference = versions
        .reference
        .as_ref()
        .map(|version| version.patchlevel.as_str());
    let current_subject = versions
        .subject
        .as_ref()
        .map(|version| version.patchlevel.as_str());
    recorded_reference != current_reference || recorded_subject != current_subject
}

/// Replay one generated seed against a resolved backend pair.
fn replay_command(
    cli: &Cli,
    config: &GenConfig,
    timeout: Duration,
    seed: u64,
    pair_args: PairArgs,
) -> std::process::ExitCode {
    let pair = match ResolvedPair::resolve(pair_args, cli) {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("error: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let versions = match probe_pair_versions(&pair, timeout) {
        Ok(versions) => versions,
        Err(error) => {
            eprintln!("error: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    if let Ok(registry) = Registry::open(pair.findings_dir(&cli.findings))
        && let Some(prior) = registry.load(seed)
    {
        eprintln!(
            "note: seed {seed} is a recorded finding ({:?})",
            prior.category
        );
        if replay_versions_changed(
            prior.reference_version.as_deref(),
            prior.subject_version.as_deref(),
            &versions,
        ) {
            eprintln!(
                "WARNING: replay backend versions differ from the finding: reference {:?} -> {:?}, subject {:?} -> {:?}",
                prior.reference_version.as_deref(),
                versions
                    .reference
                    .as_ref()
                    .map(|version| version.patchlevel.as_str()),
                prior.subject_version.as_deref(),
                versions
                    .subject
                    .as_ref()
                    .map(|version| version.patchlevel.as_str()),
            );
        }
    }
    replay(seed, config, &pair, timeout);
    std::process::ExitCode::SUCCESS
}

/// Print the findings summary for one backend pair.
fn summary_command(findings: &Path, pair: PairArgs) -> std::process::ExitCode {
    let dir = pair_findings_dir(findings, pair.reference, pair.subject);
    match Registry::open(&dir) {
        Ok(registry) => {
            let summary = registry.summary();
            if summary.is_empty() {
                println!("no findings in {}", dir.display());
            } else {
                println!("findings in {}:", dir.display());
                for (category, count) in summary {
                    println!("  {category:?}: {count}");
                }
            }
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::from(2)
        }
    }
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let timeout = Duration::from_millis(cli.timeout_ms);
    let config = GenConfig {
        malformed_expr_permille: cli.malformed_expr_permille,
        ..GenConfig::default()
    };

    match &cli.command {
        Cmd::Run(args) => run_campaign(&cli, &config, timeout, args),
        Cmd::Replay { seed, pair } => replay_command(&cli, &config, timeout, *seed, *pair),
        Cmd::Summary { pair } => summary_command(&cli.findings, *pair),
        Cmd::Characterise {
            iterations,
            seed,
            verbose,
            compare_error_text,
        } => characterise_campaign(
            &cli,
            &config,
            timeout,
            *iterations,
            *seed,
            *verbose,
            *compare_error_text,
        ),
        Cmd::WasmCheck {
            iterations,
            seed,
            verbose,
        } => wasm_check(*iterations, *seed, *verbose, &config, &cli.findings),
        Cmd::WasmDiff {
            iterations,
            seed,
            verbose,
        } => wasm_diff_campaign(*iterations, *seed, *verbose, &config, &cli.findings),
        Cmd::BpfDiff {
            iterations,
            seed,
            verbose,
        } => bpf_diff_campaign(*iterations, *seed, *verbose, &cli.findings),
        Cmd::LinkedWasmDiff {
            iterations,
            seed,
            verbose,
        } => linked_wasm_diff_campaign(&cli, timeout, *iterations, *seed, *verbose),
    }
}

/// Drive whole-program linked WASM executions against C Tcl 9. The runtime
/// build deliberately lives in this campaign's scratch directory so it cannot
/// collide with an ordinary Cargo worktree target directory.
fn linked_wasm_diff_campaign(
    cli: &Cli,
    timeout: Duration,
    iterations: u64,
    seed: Option<u64>,
    verbose: bool,
) -> std::process::ExitCode {
    if !linked_wasm::have_wasmtime() {
        eprintln!("error: `wasmtime` is required for the linked-WASM arm");
        return std::process::ExitCode::from(2);
    }
    let Some((oracle_bin, oracle_args)) = resolve_engine(Engine::Tclsh, cli) else {
        eprintln!("error: could not find C Tcl (pass --tclsh)");
        return std::process::ExitCode::from(2);
    };
    let scratch = std::env::temp_dir().join(format!("tcl-fuzz-linked-wasm-{}", std::process::id()));
    if let Err(error) = std::fs::create_dir_all(&scratch) {
        eprintln!("error: creating linked-WASM scratch directory: {error}");
        return std::process::ExitCode::from(2);
    }
    let linked_findings = match ReproducerStore::open(&cli.findings) {
        Ok(store) => store,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&scratch);
            eprintln!(
                "error: creating linked-WASM findings directory {}: {error}",
                cli.findings.display()
            );
            return std::process::ExitCode::from(2);
        }
    };
    if let Err(error) =
        verify_tcl90_backend("C Tcl oracle", &oracle_bin, &oracle_args, &scratch, timeout)
    {
        let _ = std::fs::remove_dir_all(&scratch);
        eprintln!("error: {error}");
        return std::process::ExitCode::from(2);
    }
    let runtime = match linked_wasm::build_runtime(&scratch) {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&scratch);
            eprintln!("error: {error}");
            return std::process::ExitCode::from(2);
        }
    };

    let base_seed = seed.unwrap_or_else(time_seed);
    let mut matched = 0u64;
    let mut failed = 0u64;
    eprintln!(
        "linked-wasm-diff: {iterations} programs from seed {base_seed}\n  oracle: C Tcl 9 ({})\n  runtime: {}",
        oracle_bin.display(),
        runtime.display(),
    );
    for offset in 0..iterations {
        let current_seed = base_seed.wrapping_add(offset);
        let case = linked_wasm::Case::from_seed(current_seed);
        match linked_wasm::check(
            &case,
            &runtime,
            &oracle_bin,
            &oracle_args,
            &scratch,
            current_seed,
            timeout,
        ) {
            linked_wasm::LinkedVerdict::Match => matched += 1,
            verdict => {
                failed += 1;
                if verbose {
                    eprintln!("  finding @ seed {current_seed}: {verdict:?}");
                }
                let file_name = format!("linked-wasm-{current_seed}.tcl");
                if let Err(error) = linked_findings.write(&file_name, case.program()) {
                    let _ = std::fs::remove_dir_all(&scratch);
                    eprintln!("error: writing linked-WASM reproducer {file_name}: {error}");
                    return std::process::ExitCode::from(2);
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
    eprintln!("\nlinked-wasm-diff complete:\n  matched {matched} | failed {failed}");
    if failed == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(1)
    }
}

/// Drive the real eBPF value-differential arm over `iterations` generated
/// socket filters. Unlike core Tcl, BPF-Tcl has its own typed dialect runtime;
/// the restricted shape's registry-declared packet semantics are its oracle.
fn bpf_diff_campaign(
    iterations: u64,
    seed: Option<u64>,
    verbose: bool,
    findings_dir: &Path,
) -> std::process::ExitCode {
    let base_seed = seed.unwrap_or_else(time_seed);
    let diff_findings = findings_dir.join("bpf-diff");
    let reproducers = match ReproducerStore::open(&diff_findings) {
        Ok(store) => store,
        Err(error) => {
            eprintln!(
                "error: creating BPF findings directory {}: {error}",
                diff_findings.display()
            );
            return std::process::ExitCode::from(2);
        }
    };
    let (mut matched, mut failed) = (0u64, 0u64);

    eprintln!("bpf-diff: {iterations} registry-described socket filters from seed {base_seed}");
    for offset in 0..iterations {
        let current_seed = base_seed.wrapping_add(offset);
        let case = bpf_diff::Case::from_seed(current_seed);
        match bpf_diff::check(case) {
            Ok((actual, expected)) if actual == expected => matched += 1,
            Ok((actual, expected)) => {
                failed += 1;
                if verbose {
                    eprintln!(
                        "  divergence @ seed {current_seed}: actual {actual}, expected {expected}"
                    );
                }
                if let Ok(source) = bpf_diff::source(case) {
                    let file_name = format!("seed-{current_seed}.bpftcl");
                    if let Err(error) = reproducers.write(
                        &file_name,
                        format!("# actual={actual}; expected={expected}\n{source}"),
                    ) {
                        eprintln!("error: writing BPF reproducer {file_name}: {error}");
                        return std::process::ExitCode::from(2);
                    }
                }
            }
            Err(error) => {
                failed += 1;
                if verbose {
                    eprintln!("  backend error @ seed {current_seed}: {error}");
                }
                if let Ok(source) = bpf_diff::source(case) {
                    let file_name = format!("error-{current_seed}.bpftcl");
                    if let Err(write_error) =
                        reproducers.write(&file_name, format!("# {error}\n{source}"))
                    {
                        eprintln!("error: writing BPF reproducer {file_name}: {write_error}");
                        return std::process::ExitCode::from(2);
                    }
                }
            }
        }
    }

    eprintln!(
        "\nbpf-diff complete:\n  matched {matched} | failed {failed}\n  findings: {failed} (programs in {})",
        diff_findings.display(),
    );
    if failed == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(1)
    }
}

/// Verify that one backend reports the Tcl 9.0 release line before trusting a
/// three-way campaign. Cross-version semantics otherwise become false backend
/// findings, including when both Rust implementations agree with each other.
fn verify_tcl90_backend(
    label: &str,
    binary: &Path,
    args: &[String],
    scratch: &Path,
    timeout: Duration,
) -> Result<version::EngineVersion, String> {
    let probe = write_script(scratch, u64::MAX, "puts [info patchlevel]\n")
        .map_err(|e| format!("could not write Tcl 9 oracle probe: {e}"))?;
    let outcome = run_backend(binary, args, &probe, timeout);
    let _ = std::fs::remove_file(probe);
    match outcome {
        Outcome::Ran {
            stdout,
            errored: false,
            ..
        } => {
            let patchlevel = stdout.lines().next().unwrap_or_default();
            let reported = version::EngineVersion::parse(patchlevel);
            if reported.line == Some(tcl_dialect::TclVersion::V9_0) {
                Ok(reported)
            } else {
                Err(format!(
                    "the configured {label} reports {:?}; this arm requires Tcl 9.0",
                    reported.patchlevel,
                ))
            }
        }
        other => Err(format!(
            "could not run the configured {label}: {}",
            render(&other)
        )),
    }
}

/// A concrete command invocation for one characterisation backend.
#[derive(Debug, PartialEq, Eq)]
struct BackendInvocation {
    binary: PathBuf,
    args: Vec<String>,
}

/// All three invocations required by the characterisation campaign, resolved
/// before any backend is probed so missing-command failures remain deterministic.
#[derive(Debug, PartialEq, Eq)]
struct CharacteriseInvocations {
    oracle: BackendInvocation,
    tclvm: BackendInvocation,
    runtime: BackendInvocation,
}

impl CharacteriseInvocations {
    /// Resolve every backend while preserving the public command's oracle,
    /// `tcl-vm`, then runtime error order.
    fn resolve(cli: &Cli) -> Result<Self, String> {
        let resolve = |engine| {
            resolve_engine(engine, cli)
                .map(|(binary, args)| BackendInvocation { binary, args })
                .ok_or_else(|| {
                    format!(
                        "could not find `{}` (pass --{})",
                        engine.label(),
                        cli_flag_for(engine)
                    )
                })
        };
        Ok(Self {
            oracle: resolve(Engine::Tclsh)?,
            tclvm: resolve(Engine::Tclvm)?,
            runtime: resolve(Engine::RuntimeRust)?,
        })
    }

    /// Verify the Tcl release line reported by all resolved invocations.
    fn verify(self, scratch: &Path, timeout: Duration) -> Result<CharacteriseBackends, String> {
        let verify =
            |label: &str, invocation: BackendInvocation| -> Result<VerifiedBackend, String> {
                let version = verify_tcl90_backend(
                    label,
                    &invocation.binary,
                    &invocation.args,
                    scratch,
                    timeout,
                )?;
                Ok(VerifiedBackend {
                    invocation,
                    version,
                })
            };
        Ok(CharacteriseBackends {
            oracle: verify("C Tcl oracle", self.oracle)?,
            tclvm: verify("tcl-vm backend", self.tclvm)?,
            runtime: verify("runtime/rust backend", self.runtime)?,
        })
    }
}

/// One characterisation backend after its Tcl 9.0 probe passes.
struct VerifiedBackend {
    invocation: BackendInvocation,
    version: version::EngineVersion,
}

/// The three validated characterisation backends.
struct CharacteriseBackends {
    oracle: VerifiedBackend,
    tclvm: VerifiedBackend,
    runtime: VerifiedBackend,
}

/// Count only classifications that represent a semantic backend finding.
fn characterise_finding_count(counts: &std::collections::BTreeMap<Classification, u64>) -> u64 {
    counts
        .iter()
        .filter(|(classification, _)| classification.is_finding())
        .map(|(_, count)| count)
        .sum()
}

/// Render the stable characterisation summary shared by interactive and CI
/// campaigns.
fn print_characterise_summary(
    counts: &std::collections::BTreeMap<Classification, u64>,
    findings_dir: &Path,
) {
    eprintln!(
        "\ncharacterise complete:\n  all-agree {} | tcl-vm {} | runtime/rust {} | shared {} | three-way {} | incomplete {}\n  findings: {} (scripts in {})",
        counts.get(&Classification::AllAgree).copied().unwrap_or(0),
        counts
            .get(&Classification::TclVmDiverges)
            .copied()
            .unwrap_or(0),
        counts
            .get(&Classification::RuntimeRustDiverges)
            .copied()
            .unwrap_or(0),
        counts
            .get(&Classification::SharedDivergence)
            .copied()
            .unwrap_or(0),
        counts
            .get(&Classification::ThreeWayDivergence)
            .copied()
            .unwrap_or(0),
        counts
            .get(&Classification::Incomplete)
            .copied()
            .unwrap_or(0),
        characterise_finding_count(counts),
        findings_dir.display(),
    );
}

/// Drive the three-way native-runtime characterisation campaign.
fn characterise_campaign(
    cli: &Cli,
    config: &GenConfig,
    timeout: Duration,
    iterations: u64,
    seed: Option<u64>,
    verbose: bool,
    compare_error_text: bool,
) -> std::process::ExitCode {
    let invocations = match CharacteriseInvocations::resolve(cli) {
        Ok(invocations) => invocations,
        Err(message) => {
            eprintln!("error: {message}");
            return std::process::ExitCode::from(2);
        }
    };

    let scratch =
        std::env::temp_dir().join(format!("tcl-fuzz-characterise-{}", std::process::id()));
    if let Err(error) = std::fs::create_dir_all(&scratch) {
        eprintln!("error: creating scratch directory: {error}");
        return std::process::ExitCode::from(2);
    }
    let backends = match invocations.verify(&scratch, timeout) {
        Ok(backends) => backends,
        Err(message) => {
            let _ = std::fs::remove_dir_all(&scratch);
            eprintln!("error: {message}");
            return std::process::ExitCode::from(2);
        }
    };

    let findings_dir = cli.findings.join("characterise");
    let reproducers = match ReproducerStore::open(&findings_dir) {
        Ok(store) => store,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&scratch);
            eprintln!(
                "error: creating characterise findings directory {}: {error}",
                findings_dir.display()
            );
            return std::process::ExitCode::from(2);
        }
    };
    let base_seed = seed.unwrap_or_else(time_seed);
    let mut counts = std::collections::BTreeMap::<Classification, u64>::new();
    eprintln!(
        "characterise: {iterations} programs from seed {base_seed}\n  oracle: C Tcl {} ({})\n  tcl-vm: {} ({})\n  runtime/rust: {} ({})",
        backends.oracle.version.patchlevel,
        backends.oracle.invocation.binary.display(),
        backends.tclvm.version.patchlevel,
        backends.tclvm.invocation.binary.display(),
        backends.runtime.version.patchlevel,
        backends.runtime.invocation.binary.display(),
    );

    for offset in 0..iterations {
        let current_seed = base_seed.wrapping_add(offset);
        let script = generate(current_seed, config);
        let Ok(path) = write_script(&scratch, current_seed, &script) else {
            *counts.entry(Classification::Incomplete).or_default() += 1;
            continue;
        };
        let oracle = run_backend(
            &backends.oracle.invocation.binary,
            &backends.oracle.invocation.args,
            &path,
            timeout,
        );
        let tclvm = run_backend(
            &backends.tclvm.invocation.binary,
            &backends.tclvm.invocation.args,
            &path,
            timeout,
        );
        let runtime = run_backend(
            &backends.runtime.invocation.binary,
            &backends.runtime.invocation.args,
            &path,
            timeout,
        );
        let _ = std::fs::remove_file(path);
        let classification = characterize::classify(&oracle, &tclvm, &runtime, compare_error_text);
        *counts.entry(classification).or_default() += 1;
        if classification.is_finding() {
            if verbose {
                eprintln!("  {classification:?} @ seed {current_seed}");
            }
            let file_name = format!("{current_seed}-{classification:?}.tcl");
            if let Err(error) = reproducers.write(&file_name, script) {
                let _ = std::fs::remove_dir_all(&scratch);
                eprintln!("error: writing characterise reproducer {file_name}: {error}");
                return std::process::ExitCode::from(2);
            }
        } else if !verbose && offset % 100 == 0 {
            eprintln!("  {offset}/{iterations}");
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);

    print_characterise_summary(&counts, &findings_dir);
    if characterise_finding_count(&counts) > 0 {
        std::process::ExitCode::from(1)
    } else {
        std::process::ExitCode::SUCCESS
    }
}

/// Drive the WASM value-differential arm over `iterations` generated programs.
fn wasm_diff_campaign(
    iterations: u64,
    seed: Option<u64>,
    verbose: bool,
    config: &GenConfig,
    findings_dir: &Path,
) -> std::process::ExitCode {
    let base_seed = seed.unwrap_or_else(time_seed);
    let diff_findings = findings_dir.join("wasm-diff");
    let reproducers = match ReproducerStore::open(&diff_findings) {
        Ok(store) => store,
        Err(error) => {
            eprintln!(
                "error: creating WASM diff findings directory {}: {error}",
                diff_findings.display()
            );
            return std::process::ExitCode::from(2);
        }
    };
    let engine = wasm_diff::engine();

    eprintln!("wasm-diff: {iterations} programs from seed {base_seed}");
    // A codegen panic prints a backtrace by default; silence it for the run.
    let prior_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let (mut matched, mut diverged, mut unrunnable, mut hung) = (0u64, 0u64, 0u64, 0u64);
    for i in 0..iterations {
        let s = base_seed.wrapping_add(i);
        let script = generate(s, config);
        match wasm_diff::check(&engine, &script) {
            wasm_diff::DiffVerdict::Match => matched += 1,
            wasm_diff::DiffVerdict::Unrunnable(reason) => {
                unrunnable += 1;
                if verbose {
                    eprintln!(
                        "  unrunnable @ seed {s}: {}",
                        reason.lines().next().unwrap_or("")
                    );
                }
            }
            wasm_diff::DiffVerdict::WasmHang => {
                hung += 1;
                if verbose {
                    eprintln!("  WASM HANG @ seed {s} (non-terminating compiled control flow)");
                }
                let file_name = format!("hang-{s}.tcl");
                if let Err(error) = reproducers.write(&file_name, &script) {
                    std::panic::set_hook(prior_hook);
                    eprintln!("error: writing WASM hang reproducer {file_name}: {error}");
                    return std::process::ExitCode::from(2);
                }
            }
            wasm_diff::DiffVerdict::Divergence { wasm, direct } => {
                diverged += 1;
                if verbose {
                    eprintln!("  DIVERGENCE @ seed {s}: wasm={wasm:?} direct={direct:?}");
                }
                let file_name = format!("seed-{s}.tcl");
                if let Err(error) = reproducers.write(&file_name, &script) {
                    std::panic::set_hook(prior_hook);
                    eprintln!("error: writing WASM reproducer {file_name}: {error}");
                    return std::process::ExitCode::from(2);
                }
            }
        }
        if !verbose && i % 50 == 0 {
            eprintln!("  {i}/{iterations} — {} findings", diverged + hung);
        }
    }

    std::panic::set_hook(prior_hook);
    let findings = diverged + hung;
    eprintln!(
        "\nwasm-diff complete:\n  matched {matched} | diverged {diverged} | hung {hung} | unrunnable {unrunnable}\n  findings: {findings} (scripts in {})",
        diff_findings.display(),
    );
    if findings > 0 {
        std::process::ExitCode::from(1)
    } else {
        std::process::ExitCode::SUCCESS
    }
}

/// Drive the WASM-runnability arm over `iterations` generated programs.
fn wasm_check(
    iterations: u64,
    seed: Option<u64>,
    verbose: bool,
    config: &GenConfig,
    findings_dir: &Path,
) -> std::process::ExitCode {
    if !wasm::have_wasmtime() {
        eprintln!("error: `wasmtime` not found on PATH — the WASM arm needs it");
        return std::process::ExitCode::from(2);
    }
    let base_seed = seed.unwrap_or_else(time_seed);
    let scratch = std::env::temp_dir().join(format!("tcl-fuzz-wasm-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&scratch);
    let wasm_findings = findings_dir.join("wasm");
    let reproducers = match ReproducerStore::open(&wasm_findings) {
        Ok(store) => store,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&scratch);
            eprintln!(
                "error: creating WASM findings directory {}: {error}",
                wasm_findings.display()
            );
            return std::process::ExitCode::from(2);
        }
    };

    eprintln!("wasm-check: {iterations} programs from seed {base_seed}");
    // A generated program that makes codegen panic prints a backtrace by
    // default; silence the hook for the campaign so the report stays readable
    // (the verdict still captures the panic message).
    let prior_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let (mut ran, mut codegen_failed, mut trapped) = (0u64, 0u64, 0u64);
    for i in 0..iterations {
        let s = base_seed.wrapping_add(i);
        let script = generate(s, config);
        match wasm::check(&script, &scratch, s) {
            wasm::WasmVerdict::Ran => ran += 1,
            wasm::WasmVerdict::Unavailable => {}
            verdict => {
                let (kind, reason) = match &verdict {
                    wasm::WasmVerdict::CodegenFailed(r) => {
                        codegen_failed += 1;
                        ("codegen", r.as_str())
                    }
                    wasm::WasmVerdict::Trapped(r) => {
                        trapped += 1;
                        ("trap", r.as_str())
                    }
                    _ => unreachable!(),
                };
                if verbose {
                    eprintln!(
                        "  {kind} finding @ seed {s}: {}",
                        reason.lines().next().unwrap_or("")
                    );
                }
                let file_name = format!("seed-{s}.tcl");
                if let Err(error) = reproducers.write(&file_name, &script) {
                    std::panic::set_hook(prior_hook);
                    let _ = std::fs::remove_dir_all(&scratch);
                    eprintln!("error: writing WASM reproducer {file_name}: {error}");
                    return std::process::ExitCode::from(2);
                }
            }
        }
        if !verbose && i % 50 == 0 {
            eprintln!("  {i}/{iterations} — {} findings", codegen_failed + trapped);
        }
    }

    std::panic::set_hook(prior_hook);
    let _ = std::fs::remove_dir_all(&scratch);
    let findings = codegen_failed + trapped;
    eprintln!(
        "\nwasm-check complete:\n  ran {ran} | codegen-failed {codegen_failed} | trapped {trapped}\n  findings: {findings} (scripts in {})",
        wasm_findings.display(),
    );
    if findings > 0 {
        std::process::ExitCode::from(1)
    } else {
        std::process::ExitCode::SUCCESS
    }
}

/// Regenerate `seed`, run both engines, and print a side-by-side comparison.
fn replay(seed: u64, config: &GenConfig, pair: &ResolvedPair, timeout: Duration) {
    let script = generate(seed, config);
    println!("=== script (seed {seed}) ===\n{script}");
    let scratch = std::env::temp_dir();
    let Ok(path) = write_script(&scratch, seed, &script) else {
        eprintln!("error: could not write scratch script");
        return;
    };
    let reference = run_backend(&pair.reference_bin, &pair.reference_args, &path, timeout);
    let subject = run_backend(&pair.subject_bin, &pair.subject_args, &path, timeout);
    let _ = std::fs::remove_file(&path);
    println!(
        "=== reference: {} ({}) ===\n{}",
        pair.reference.label(),
        pair.reference_bin.display(),
        render(&reference)
    );
    println!(
        "=== subject: {} ({}) ===\n{}",
        pair.subject.label(),
        pair.subject_bin.display(),
        render(&subject)
    );
    println!(
        "=== verdict: {:?} ===",
        compare_outcomes(&reference, &subject, true)
    );
}

fn render(o: &Outcome) -> String {
    match o {
        Outcome::Ran {
            stdout,
            stderr,
            errored,
        } => {
            if stderr.is_empty() {
                format!("[errored={errored}]\n{stdout}")
            } else {
                format!("[errored={errored}]\n{stdout}--- stderr ---\n{stderr}")
            }
        }
        Outcome::Timeout => "<timeout>".to_owned(),
        Outcome::Unavailable(m) => format!("<unavailable: {m}>"),
    }
}

/// The extra arguments that pin `engine` to Tcl release `want` (`"8.6"`).
///
/// `Err` carries a ready-to-print message when the release is unrecognised or
/// the engine has no way to emulate another release. Refusing beats silently
/// ignoring: a campaign told to match its reference must either do so or stop,
/// never quietly run version-skewed (issue #1328).
fn version_flag_for(engine: Engine, want: &str) -> Result<Vec<String>, String> {
    if tcl_dialect::TclVersion::from_package_version(want).is_none() {
        return Err(format!(
            "unknown --subject-tcl-version {want:?} (want 8.4, 8.5, 8.6, 9.0 or 9.1)"
        ));
    }
    match engine {
        Engine::RuntimeRust | Engine::Tclvm => {
            Ok(vec!["--tcl-version".to_owned(), want.to_owned()])
        }
        // `tclsh` *is* a released build — point --tclsh at a different one.
        Engine::Tclsh => Err(format!(
            "cannot pin `tclsh` to Tcl {want}: point --tclsh at a {want} build instead"
        )),
    }
}

fn print_stats(stats: &Stats) {
    eprintln!(
        "\ncampaign complete:\n  total {} | matched {} | skipped {}\n  findings: stdout {} | status {} | error-text {} | timeout {} (new this run: {})",
        stats.total,
        stats.matched,
        stats.skipped,
        stats.stdout_mismatch,
        stats.status_mismatch,
        stats.error_text_mismatch,
        stats.timeout,
        stats.new_findings,
    );
}

/// Resolve an [`Engine`] to its `(binary, leading-args)` invocation, honouring
/// the matching `--tclvm`/`--tclsh`/`--runtime-rust` override.
fn resolve_engine(engine: Engine, cli: &Cli) -> Option<(PathBuf, Vec<String>)> {
    let explicit = match engine {
        Engine::Tclvm => cli.tclvm.as_deref(),
        Engine::Tclsh => cli.tclsh.as_deref(),
        Engine::RuntimeRust => cli.runtime_rust.as_deref(),
    };
    let bin = engine.resolve(explicit)?;
    Some((bin, engine.args()))
}

/// The `--<flag>` a user should pass to point this engine at an explicit
/// binary, for an error message.
fn cli_flag_for(engine: Engine) -> &'static str {
    match engine {
        Engine::Tclvm => "tclvm",
        Engine::Tclsh => "tclsh",
        Engine::RuntimeRust => "runtime-rust",
    }
}

/// The findings directory for a backend pair: the plain `base` for the
/// original default pair (`tclsh` reference / `tclvm` subject — keeps
/// existing findings directories meaningful with no migration), else
/// `base/<subject>-vs-<reference>/` so a different pair never collides with
/// another pair's seeds.
fn pair_findings_dir(base: &Path, reference: Engine, subject: Engine) -> PathBuf {
    if matches!(reference, Engine::Tclsh) && matches!(subject, Engine::Tclvm) {
        base.to_owned()
    } else {
        base.join(format!("{}-vs-{}", subject.label(), reference.label()))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory as _;

    #[test]
    fn cli_definition_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn default_pair_keeps_the_plain_findings_dir() {
        // Issue #1313: the original `tclvm` subject / `tclsh` reference pair
        // must keep writing to `<findings>/` so existing registries need no
        // migration when other pairs are added.
        let base = Path::new("fuzz-findings");
        assert_eq!(
            pair_findings_dir(base, Engine::Tclsh, Engine::Tclvm),
            base.to_owned()
        );
    }

    #[test]
    fn other_pairs_are_namespaced_so_seeds_never_collide() {
        let base = Path::new("fuzz-findings");
        assert_eq!(
            pair_findings_dir(base, Engine::Tclsh, Engine::RuntimeRust),
            base.join("runtime-rust-vs-tclsh")
        );
        assert_eq!(
            pair_findings_dir(base, Engine::RuntimeRust, Engine::Tclvm),
            base.join("tclvm-vs-runtime-rust")
        );
        // The same two engines swapped is a *different* registry — subject and
        // reference are not interchangeable.
        assert_ne!(
            pair_findings_dir(base, Engine::Tclsh, Engine::RuntimeRust),
            pair_findings_dir(base, Engine::RuntimeRust, Engine::Tclsh)
        );
    }

    #[test]
    fn every_engine_names_the_flag_that_overrides_it() {
        for engine in [Engine::Tclvm, Engine::Tclsh, Engine::RuntimeRust] {
            assert_eq!(cli_flag_for(engine), engine.label());
        }
    }

    #[test]
    fn version_pinning_reaches_both_emulated_runtime_subjects() {
        let want = vec!["--tcl-version".to_owned(), "8.6".to_owned()];
        // TP: both engines own a release-selection flag, so the differential
        // pair can match a Tcl 8.6 oracle instead of recording deliberate
        // cross-version behaviour as a finding.
        assert_eq!(
            version_flag_for(Engine::RuntimeRust, "8.6"),
            Ok(want.clone())
        );
        assert_eq!(version_flag_for(Engine::Tclvm, "8.6"), Ok(want));
        // TN: the C interpreter must be selected by binary, never pretended
        // to be mutable at run time.
        assert!(version_flag_for(Engine::Tclsh, "8.6").is_err());
        // FN: a typo cannot silently select the subject's default release.
        assert!(version_flag_for(Engine::Tclvm, "8.7").is_err());
    }

    #[test]
    fn identical_backend_pairs_are_rejected() {
        let cli = Cli::parse_from(["tcl-fuzz", "run"]);
        let pair = PairArgs {
            reference: Engine::Tclvm,
            subject: Engine::Tclvm,
        };
        assert!(ResolvedPair::resolve(pair, &cli).is_err());
    }

    #[test]
    fn an_explicit_override_resolves_that_engine_and_its_args() {
        let cli = Cli::parse_from([
            "tcl-fuzz",
            "--runtime-rust",
            "/opt/run_script",
            "run",
            "--subject",
            "runtime-rust",
        ]);
        let (bin, args) =
            resolve_engine(Engine::RuntimeRust, &cli).expect("explicit path resolves");
        assert_eq!(bin, PathBuf::from("/opt/run_script"));
        // `runtime/rust`'s runner needs `--quiet` to match `tclsh script.tcl`'s
        // stdout contract — see `engine::Engine::args`.
        assert_eq!(args, vec!["--quiet".to_string()]);
    }

    #[test]
    fn characterise_resolution_keeps_each_backend_invocation_together() {
        let cli = Cli::parse_from([
            "tcl-fuzz",
            "--tclsh",
            "/opt/tclsh9.0",
            "--tclvm",
            "/opt/tclvm",
            "--runtime-rust",
            "/opt/run_script",
            "characterise",
        ]);
        let resolved = CharacteriseInvocations::resolve(&cli).expect("explicit paths resolve");
        assert_eq!(resolved.oracle.binary, PathBuf::from("/opt/tclsh9.0"));
        assert!(resolved.oracle.args.is_empty());
        assert_eq!(resolved.tclvm.binary, PathBuf::from("/opt/tclvm"));
        assert!(resolved.tclvm.args.is_empty());
        assert_eq!(resolved.runtime.binary, PathBuf::from("/opt/run_script"));
        assert_eq!(resolved.runtime.args, vec!["--quiet".to_string()]);
    }

    #[test]
    fn replay_version_warning_compares_both_recorded_backends() {
        let versions = version::PairVersions {
            reference: Some(version::EngineVersion::parse("9.0.3")),
            subject: Some(version::EngineVersion::parse("9.0.4")),
        };
        assert!(!replay_versions_changed(
            Some("9.0.3"),
            Some("9.0.4"),
            &versions,
        ));
        assert!(replay_versions_changed(
            Some("9.0.3"),
            Some("9.0.5"),
            &versions,
        ));
        assert!(replay_versions_changed(None, Some("9.0.4"), &versions));
    }

    #[test]
    fn characterise_finding_count_excludes_agreement_and_incomplete_runs() {
        let counts = std::collections::BTreeMap::from([
            (Classification::AllAgree, 11),
            (Classification::TclVmDiverges, 2),
            (Classification::SharedDivergence, 3),
            (Classification::Incomplete, 7),
        ]);
        assert_eq!(characterise_finding_count(&counts), 5);
    }
}
