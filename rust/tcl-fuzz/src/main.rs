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
use tcl_dialect::TclVersion;

use campaign::{Backend, Campaign, Stats};
use characterize::Classification;
use engine::Engine;
use findings::{Finding, Registry, ReproducerStore};
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
    /// How often a generated word is given a value that differs from its
    /// source spelling — delimiter-shaped (`"{}"`, `"{$x}"`), escape-bearing
    /// (`a\ b`, `\x41`), or non-ASCII / whitespace-bearing (`"café"`,
    /// `"a b"`, `""`) — in parts per thousand. `0` opts out entirely and
    /// restores the exact pre-existing generator stream, so a registry's
    /// historical findings still replay. Global for the same reason
    /// `--malformed-expr-permille` is: `replay` has to generate a seed the way
    /// the campaign that recorded it did.
    #[arg(long, global = true, default_value_t = GenConfig::default().word_shape_permille)]
    word_shape_permille: u32,
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
    /// Pin the Tcl release both engines emulate (`8.4`…`9.1`). The legacy
    /// `--subject-tcl-version` spelling remains an alias, but its old
    /// subject-only behaviour was unsound: every engine in the pair must
    /// honour this release or the command refuses to run.
    #[arg(
        long,
        value_name = "X.Y",
        value_parser = parse_tcl_version,
        visible_alias = "subject-tcl-version"
    )]
    tcl_version: Option<TclVersion>,
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
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a fuzzing campaign over one backend pair.
    Run(RunArgs),
    /// Replay a single seed and print both engines' output side by side.
    Replay {
        /// The seed to reproduce.
        seed: u64,
        /// The engine pair. When a matching release-aware finding exists,
        /// replay restores its recorded release automatically; otherwise pass
        /// `--tcl-version` explicitly.
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
    tcl_version: Option<TclVersion>,
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
        let mut resolved = Self {
            reference: pair.reference,
            subject: pair.subject,
            tcl_version: None,
            reference_bin,
            reference_args,
            subject_bin,
            subject_args,
        };
        if let Some(tcl_version) = pair.tcl_version {
            resolved.apply_tcl_version(tcl_version);
        }
        Ok(resolved)
    }

    /// Apply one campaign-wide Tcl release to both engines before any process
    /// is spawned. Fixed-release engines receive no flag; the probe below
    /// validates that their selected binary nevertheless honours the request.
    fn apply_tcl_version(&mut self, tcl_version: TclVersion) {
        let reference_args = self.reference.tcl_version_args(tcl_version);
        let subject_args = self.subject.tcl_version_args(tcl_version);
        self.reference_args.extend(reference_args);
        self.subject_args.extend(subject_args);
        self.tcl_version = Some(tcl_version);
    }

    /// This pair's findings directory under `base` — see [`pair_findings_dir`].
    fn findings_dir(&self, base: &Path) -> PathBuf {
        pair_findings_dir(base, self.reference, self.subject, self.tcl_version)
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
        tcl_version: pair.tcl_version,
        reference: Some(probe(
            pair.reference,
            &pair.reference_bin,
            &pair.reference_args,
        )?),
        subject: Some(probe(pair.subject, &pair.subject_bin, &pair.subject_args)?),
    })
}

/// Verify that both engines actually selected the release requested for this
/// campaign. Accepting an argument is not sufficient: a wrapper could ignore
/// it, and that would recreate the version-skew findings this flag prevents.
fn verify_requested_tcl_version(versions: &version::PairVersions) -> Result<(), String> {
    let Some(requested) = versions.tcl_version else {
        return Ok(());
    };
    for (role, actual) in [
        ("reference", versions.reference.as_ref()),
        ("subject", versions.subject.as_ref()),
    ] {
        let Some(actual) = actual else {
            return Err(format!(
                "{role} engine did not report a Tcl release after --tcl-version {}",
                requested.version_string(),
            ));
        };
        if actual.line != Some(requested) {
            return Err(format!(
                "{role} engine reports Tcl {} after --tcl-version {}; it cannot honour the requested release",
                actual.patchlevel,
                requested.version_string(),
            ));
        }
    }
    Ok(())
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
    let findings_dir = pair.findings_dir(&cli.findings);
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
    if let Err(error) = verify_requested_tcl_version(&versions) {
        let _ = std::fs::remove_dir_all(&scratch);
        eprintln!("error: {error}");
        return std::process::ExitCode::from(2);
    }
    let registry = match Registry::open(&findings_dir) {
        Ok(registry) => registry,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&scratch);
            eprintln!("error: opening findings dir: {error}");
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
    let stats = match stats {
        Ok(stats) => stats,
        Err(error) => {
            eprintln!("error: recording finding: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    print_stats(&stats);
    // Exit non-zero when a divergence was found, so CI can gate on it.
    if stats.findings() > 0 {
        std::process::ExitCode::from(1)
    } else {
        std::process::ExitCode::SUCCESS
    }
}

/// A recorded finding selected for replay, plus the release its invocation
/// must restore. `finding` is absent for an ad-hoc seed that has no record.
#[derive(Debug)]
struct ReplaySelection {
    tcl_version: Option<TclVersion>,
    finding: Option<Finding>,
}

/// Load a seed only when its registry already exists. Looking for a recorded
/// release must never create empty namespaces for every supported release.
fn load_existing_finding(dir: &Path, seed: u64) -> Result<Option<Finding>, String> {
    match dir.try_exists() {
        Ok(false) => return Ok(None),
        Ok(true) => {}
        Err(error) => {
            return Err(format!("checking findings dir {}: {error}", dir.display()));
        }
    }
    Registry::open(dir)
        .map_err(|error| format!("opening findings dir {}: {error}", dir.display()))
        .and_then(|registry| {
            registry
                .load_checked(seed)
                .map_err(|error| format!("loading finding {seed} from {}: {error}", dir.display()))
        })
}

/// Load every release-aware record for one seed, rejecting a record whose
/// directory and own metadata disagree.
fn release_findings(
    findings: &Path,
    pair: PairArgs,
    seed: u64,
) -> Result<Vec<(TclVersion, Finding)>, String> {
    let mut matches = Vec::new();
    for (release, dir) in pair_registry_dirs(findings, pair)
        .into_iter()
        .filter_map(|(release, dir)| release.map(|release| (release, dir)))
    {
        let tcl_version = release;
        if let Some(finding) = load_existing_finding(&dir, seed)? {
            let Some(recorded) = finding.campaign_tcl_version()? else {
                return Err(format!(
                    "finding {seed} is stored under Tcl {} but records an unpinned campaign; refusing inconsistent replay metadata",
                    tcl_version.version_string(),
                ));
            };
            if recorded != tcl_version {
                return Err(format!(
                    "finding {seed} is stored under Tcl {} but records Tcl {}; refusing inconsistent replay metadata",
                    tcl_version.version_string(),
                    recorded.version_string(),
                ));
            }
            matches.push((tcl_version, finding));
        }
    }
    Ok(matches)
}

/// Every registry namespace belonging to a backend pair. Consumers that need
/// to discover records (rather than create a campaign registry) share this
/// owner so release children cannot be forgotten by one command.
fn pair_registry_dirs(findings: &Path, pair: PairArgs) -> Vec<(Option<TclVersion>, PathBuf)> {
    std::iter::once((
        None,
        pair_findings_dir(findings, pair.reference, pair.subject, None),
    ))
    .chain(TclVersion::ALL.into_iter().map(|tcl_version| {
        (
            Some(tcl_version),
            pair_findings_dir(findings, pair.reference, pair.subject, Some(tcl_version)),
        )
    }))
    .collect()
}

/// Find the persisted campaign release for `seed`, if any.
///
/// A seed is arbitrary only when it occurs in no registry at all. An explicit
/// `--tcl-version` selects its exact release-aware record even when the same
/// seed also exists in other release namespaces. It never permits an older
/// unversioned record to be ignored.
fn select_replay(findings: &Path, pair: PairArgs, seed: u64) -> Result<ReplaySelection, String> {
    let legacy_dir = pair_findings_dir(findings, pair.reference, pair.subject, None);
    let mut unpinned_legacy = None;
    if let Some(finding) = load_existing_finding(&legacy_dir, seed)? {
        // The pre-#1467 directory has no release component. Even a manually
        // copied record that happens to carry one is inconsistent, so reject
        // it rather than guessing which namespace owns the seed.
        let campaign = finding.campaign_tcl_version()?;
        match campaign {
            None if pair.tcl_version.is_none() => {
                if finding.replay_metadata_version.is_some() {
                    unpinned_legacy = Some(finding);
                } else {
                    return Ok(ReplaySelection {
                        tcl_version: None,
                        finding: Some(finding),
                    });
                }
            }
            None => {
                if finding.replay_metadata_version.is_some() {
                    unpinned_legacy = Some(finding);
                } else {
                    return Err(format!(
                        "finding {seed} records an unpinned campaign but replay requested Tcl {}; refusing mismatched replay",
                        pair.tcl_version.expect("checked above").version_string(),
                    ));
                }
            }
            Some(recorded)
                if pair
                    .tcl_version
                    .is_some_and(|requested| recorded != requested) =>
            {
                return Err(format!(
                    "finding {seed} in the legacy unversioned registry records Tcl {} but replay requested Tcl {}; refusing mismatched replay",
                    recorded.version_string(),
                    pair.tcl_version.expect("checked above").version_string(),
                ));
            }
            Some(_) => {}
        }
        if campaign.is_some() {
            return Err(format!(
                "finding {seed} has release metadata but is in the legacy unversioned registry; replay it from its Tcl release namespace",
            ));
        }
    }

    let matches = release_findings(findings, pair, seed)?;
    if let Some(requested) = pair.tcl_version {
        if let Some((_, finding)) = matches.iter().find(|(recorded, _)| *recorded == requested) {
            return Ok(ReplaySelection {
                tcl_version: Some(requested),
                finding: Some(finding.clone()),
            });
        }
        return if matches.is_empty() {
            if unpinned_legacy.is_some() {
                Err(format!(
                    "finding {seed} records an unpinned campaign but replay requested Tcl {}; refusing mismatched replay",
                    requested.version_string(),
                ))
            } else {
                Ok(ReplaySelection {
                    tcl_version: Some(requested),
                    finding: None,
                })
            }
        } else {
            let releases = matches
                .iter()
                .map(|(tcl_version, _)| tcl_version.version_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "finding {seed} records Tcl {releases}, but replay requested Tcl {}; refusing mismatched replay",
                requested.version_string(),
            ))
        };
    }

    match (unpinned_legacy, matches.as_slice()) {
        (Some(finding), []) => Ok(ReplaySelection {
            tcl_version: None,
            finding: Some(finding),
        }),
        (None, []) => Ok(ReplaySelection {
            tcl_version: None,
            finding: None,
        }),
        (None, [(tcl_version, finding)]) => Ok(ReplaySelection {
            tcl_version: Some(*tcl_version),
            finding: Some(finding.clone()),
        }),
        (Some(_) | None, _) => {
            let releases = matches
                .iter()
                .map(|(tcl_version, _)| tcl_version.version_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "finding {seed} exists in multiple Tcl configurations (unpinned, {releases}); replay with --tcl-version X.Y",
            ))
        }
    }
}

/// Whether the concrete backend builds used for replay differ from those
/// persisted with a finding. A campaign release pins language semantics, but
/// patchlevel provenance remains useful when narrowing a regression.
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
    let selection = match select_replay(&cli.findings, pair_args, seed) {
        Ok(selection) => selection,
        Err(error) => {
            eprintln!("error: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    let pair_args = PairArgs {
        tcl_version: selection.tcl_version,
        ..pair_args
    };
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
    if let Err(error) = verify_requested_tcl_version(&versions) {
        eprintln!("error: {error}");
        return std::process::ExitCode::from(2);
    }
    // The generator configuration the campaign ran with, restored from the
    // record rather than taken from today's CLI defaults. A rate is a `chance`
    // draw per candidate statement, so a changed rate re-phases the PRNG and
    // the same seed regenerates a different script — replay would "reproduce"
    // something that is not the finding. The same reason `tcl_version` is
    // recorded and restored above.
    let mut config = *config;
    if let Some(prior) = selection.finding.as_ref() {
        match prior.generator_rates {
            Some(rates) => {
                let cli_rates = findings::GeneratorRates::of(&config);
                if rates != cli_rates {
                    eprintln!(
                        "note: restoring the finding's generator rates (malformed-expr {}‰, word-shape {}‰); the command line asked for {}‰ / {}‰",
                        rates.malformed_expr_permille,
                        rates.word_shape_permille,
                        cli_rates.malformed_expr_permille,
                        cli_rates.word_shape_permille,
                    );
                }
                rates.restore(&mut config);
            }
            None => eprintln!(
                "WARNING: finding {seed} predates recorded generator rates; replaying at the current \
                 rates (malformed-expr {}‰, word-shape {}‰), which may not regenerate the recorded script",
                config.malformed_expr_permille, config.word_shape_permille,
            ),
        }
    }
    let config = &config;
    if let Some(prior) = selection.finding {
        eprintln!(
            "note: seed {seed} is a recorded finding ({:?}) using {}",
            prior.category,
            pair.tcl_version.map_or_else(
                || "the recorded unpinned configuration".to_owned(),
                |tcl_version| format!("Tcl {}", tcl_version.version_string()),
            ),
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
    let selected = summary_registry_dirs(findings, pair);
    if selected.is_empty() {
        let dir = pair_findings_dir(findings, pair.reference, pair.subject, pair.tcl_version);
        println!("no findings in {}", dir.display());
        return std::process::ExitCode::SUCCESS;
    }
    for (release, dir) in selected {
        let registry = match Registry::open(&dir) {
            Ok(registry) => registry,
            Err(error) => {
                eprintln!("error: {error}");
                return std::process::ExitCode::from(2);
            }
        };
        let summary = match registry.summary() {
            Ok(summary) => summary,
            Err(error) => {
                eprintln!("error: {error}");
                return std::process::ExitCode::from(2);
            }
        };
        let identity = release.map_or_else(
            || "unpinned".to_owned(),
            |tcl_version| format!("Tcl {}", tcl_version.version_string()),
        );
        if summary.is_empty() {
            println!("no findings in {} ({identity})", dir.display());
        } else {
            println!("findings in {} ({identity}):", dir.display());
            for (category, count) in summary {
                println!("  {category:?}: {count}");
            }
        }
    }
    std::process::ExitCode::SUCCESS
}

/// Existing registries selected by `summary`. Without a release filter, every
/// pair namespace is reported independently so equal seeds at different Tcl
/// releases remain distinct findings rather than colliding in one count.
fn summary_registry_dirs(findings: &Path, pair: PairArgs) -> Vec<(Option<TclVersion>, PathBuf)> {
    pair_registry_dirs(findings, pair)
        .into_iter()
        .filter(|(release, _)| {
            pair.tcl_version
                .is_none_or(|requested| *release == Some(requested))
        })
        .filter(|(_, dir)| dir.exists())
        .collect()
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    let timeout = Duration::from_millis(cli.timeout_ms);
    let config = GenConfig {
        malformed_expr_permille: cli.malformed_expr_permille,
        word_shape_permille: cli.word_shape_permille,
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

/// Parse a CLI release spelling through the dialect owner's release
/// vocabulary. Package-style patchlevels and canonical dialect names are both
/// useful aliases, but the resulting value is always the canonical release
/// line used in engine arguments, metadata, and directory names.
fn parse_tcl_version(raw: &str) -> Result<TclVersion, String> {
    TclVersion::from_package_version(raw)
        .or_else(|| TclVersion::from_dialect(Some(raw)))
        .ok_or_else(|| {
            format!("unknown Tcl release {raw:?} (want 8.4, 8.5, 8.6, 9.0, 9.1, or tclX.Y)")
        })
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
/// another pair's seeds. A release-selected campaign adds `tclX.Y/` below
/// that path, preventing the same seed from being deduplicated across Tcl
/// language versions.
fn pair_findings_dir(
    base: &Path,
    reference: Engine,
    subject: Engine,
    tcl_version: Option<TclVersion>,
) -> PathBuf {
    let pair_dir = if matches!(reference, Engine::Tclsh) && matches!(subject, Engine::Tclvm) {
        base.to_owned()
    } else {
        base.join(format!("{}-vs-{}", subject.label(), reference.label()))
    };
    tcl_version.map_or(pair_dir.clone(), |tcl_version| {
        pair_dir.join(format!("tcl{}", tcl_version.version_string()))
    })
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

    fn test_pair(reference: Engine, subject: Engine) -> ResolvedPair {
        ResolvedPair {
            reference,
            subject,
            tcl_version: None,
            reference_bin: PathBuf::from("/reference"),
            reference_args: reference.args(),
            subject_bin: PathBuf::from("/subject"),
            subject_args: subject.args(),
        }
    }

    fn pair_args(reference: Engine, subject: Engine, tcl_version: Option<TclVersion>) -> PairArgs {
        PairArgs {
            reference,
            subject,
            tcl_version,
        }
    }

    fn finding(seed: u64, tcl_version: TclVersion) -> Finding {
        let outcome = Outcome::Ran {
            stdout: "reference output\n".to_owned(),
            stderr: String::new(),
            errored: false,
        };
        Finding::new(
            seed,
            findings::Category::StdoutMismatch,
            "puts replayable",
            &findings::FindingContext {
                reference: &outcome,
                subject: &outcome,
                reference_engine: Engine::RuntimeRust.label(),
                subject_engine: Engine::Tclvm.label(),
                versions: &version::PairVersions {
                    tcl_version: Some(tcl_version),
                    reference: Some(version::EngineVersion::parse(tcl_version.patchlevel())),
                    subject: Some(version::EngineVersion::parse(tcl_version.patchlevel())),
                },
                generator_rates: findings::GeneratorRates::of(&generator::GenConfig::default()),
            },
        )
    }

    fn replay_tmp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "tcl-fuzz-replay-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn replay_refuses_an_unreadable_existing_record() {
        let base = replay_tmp("unreadable");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 37;
        let dir = pair_findings_dir(&base, args.reference, args.subject, None);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("finding-{seed}.json")), b"{ truncated").unwrap();

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(error.contains("loading finding 37"));
        assert!(error.contains("finding-37.json"));
    }

    #[test]
    fn replay_refuses_orphan_and_staged_record_companions() {
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 38;
        for (name, write, is_json) in [
            ("tcl-orphan", "finding-38.tcl", false),
            ("json-orphan", "finding-38.json", true),
            ("staged", "finding-38.tcl.stage", false),
        ] {
            let base = replay_tmp(name);
            let dir = pair_findings_dir(&base, args.reference, args.subject, None);
            std::fs::create_dir_all(&dir).unwrap();
            let contents = if is_json {
                serde_json::to_vec_pretty(&finding(seed, TclVersion::V8_6)).unwrap()
            } else {
                b"puts interrupted".to_vec()
            };
            std::fs::write(dir.join(write), contents).unwrap();

            let error = select_replay(&base, args, seed).unwrap_err();
            assert!(
                error.contains("incomplete or interrupted paired record"),
                "{name}: {error}"
            );
            let _ = std::fs::remove_dir_all(&base);
        }
    }

    #[test]
    fn release_flag_accepts_owner_defined_aliases_and_rejects_unknown_values() {
        assert_eq!(parse_tcl_version("8.6"), Ok(TclVersion::V8_6));
        assert_eq!(parse_tcl_version("8.6.16"), Ok(TclVersion::V8_6));
        assert_eq!(parse_tcl_version("tcl8.6"), Ok(TclVersion::V8_6));
        assert!(parse_tcl_version("8.7").is_err());

        let modern = Cli::try_parse_from(["tcl-fuzz", "run", "--tcl-version", "8.6"])
            .expect("canonical release flag parses");
        let legacy = Cli::try_parse_from(["tcl-fuzz", "run", "--subject-tcl-version", "tcl8.6"])
            .expect("subject-only spelling remains a compatibility alias");
        let Cmd::Run(modern) = modern.command else {
            panic!("expected run command");
        };
        let Cmd::Run(legacy) = legacy.command else {
            panic!("expected run command");
        };
        assert_eq!(modern.pair.tcl_version, Some(TclVersion::V8_6));
        assert_eq!(legacy.pair.tcl_version, Some(TclVersion::V8_6));
        assert!(Cli::try_parse_from(["tcl-fuzz", "run", "--tcl-version", "8.7"]).is_err());
    }

    #[test]
    fn replay_discovers_a_recorded_release_and_propagates_it_to_both_engines() {
        let base = replay_tmp("propagation");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 19;
        let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V8_6));
        Registry::open(&dir)
            .unwrap()
            .record(&finding(seed, TclVersion::V8_6))
            .unwrap();

        let selection = select_replay(&base, args, seed).unwrap();
        assert_eq!(selection.tcl_version, Some(TclVersion::V8_6));
        assert_eq!(
            selection.finding.as_ref().map(|finding| finding.seed),
            Some(seed)
        );
        let mut resolved = test_pair(args.reference, args.subject);
        resolved.apply_tcl_version(selection.tcl_version.unwrap());
        assert!(
            resolved
                .reference_args
                .windows(2)
                .any(|args| args == ["--tcl-version", "8.6"])
        );
        assert!(
            resolved
                .subject_args
                .windows(2)
                .any(|args| args == ["--tcl-version", "8.6"])
        );
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn replay_accepts_a_markerless_pinned_finding_from_the_release_namespace() {
        let base = replay_tmp("markerless-pinned");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 22;
        let mut pinned = finding(seed, TclVersion::V8_6);
        pinned.replay_metadata_version = None;
        let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V8_6));
        Registry::open(&dir).unwrap().record(&pinned).unwrap();

        let selection = select_replay(&base, args, seed).unwrap();
        assert_eq!(selection.tcl_version, Some(TclVersion::V8_6));
        assert_eq!(selection.finding.map(|finding| finding.seed), Some(seed));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn replay_refuses_legacy_findings_without_release_metadata() {
        let base = replay_tmp("legacy");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 23;
        let mut legacy = finding(seed, TclVersion::V9_0);
        legacy.tcl_version = None;
        legacy.replay_metadata_version = None;
        let dir = pair_findings_dir(&base, args.reference, args.subject, None);
        Registry::open(&dir).unwrap().record(&legacy).unwrap();

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(error.contains("predates replay-safe campaign metadata"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn replay_immediately_reuses_a_new_unpinned_campaign_finding() {
        let base = replay_tmp("new-unpinned");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 231;
        let mut unpinned = finding(seed, TclVersion::V9_0);
        unpinned.tcl_version = None;
        assert_eq!(unpinned.replay_metadata_version, Some(1));
        let dir = pair_findings_dir(&base, args.reference, args.subject, None);
        Registry::open(&dir).unwrap().record(&unpinned).unwrap();

        let selection = select_replay(&base, args, seed).unwrap();
        assert_eq!(selection.tcl_version, None);
        assert_eq!(selection.finding.map(|finding| finding.seed), Some(seed));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn marked_unpinned_and_pinned_records_are_ambiguous_without_a_release() {
        let base = replay_tmp("marked-unpinned-ambiguous");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 232;
        let mut unpinned = finding(seed, TclVersion::V9_0);
        unpinned.tcl_version = None;
        Registry::open(pair_findings_dir(&base, args.reference, args.subject, None))
            .unwrap()
            .record(&unpinned)
            .unwrap();
        Registry::open(pair_findings_dir(
            &base,
            args.reference,
            args.subject,
            Some(TclVersion::V8_6),
        ))
        .unwrap()
        .record(&finding(seed, TclVersion::V8_6))
        .unwrap();

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(error.contains("multiple Tcl configurations"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn explicit_release_selects_pinned_record_over_marked_unpinned_record() {
        let base = replay_tmp("marked-unpinned-explicit");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V8_6));
        let seed = 233;
        let mut unpinned = finding(seed, TclVersion::V9_0);
        unpinned.tcl_version = None;
        Registry::open(pair_findings_dir(&base, args.reference, args.subject, None))
            .unwrap()
            .record(&unpinned)
            .unwrap();
        Registry::open(pair_findings_dir(
            &base,
            args.reference,
            args.subject,
            Some(TclVersion::V8_6),
        ))
        .unwrap()
        .record(&finding(seed, TclVersion::V8_6))
        .unwrap();

        let selection = select_replay(&base, args, seed).unwrap();
        assert_eq!(selection.tcl_version, Some(TclVersion::V8_6));
        assert_eq!(
            selection
                .finding
                .and_then(|finding| finding.campaign_tcl_version().ok().flatten()),
            Some(TclVersion::V8_6),
        );
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn explicit_replay_refuses_a_legacy_release_less_finding() {
        let base = replay_tmp("explicit-legacy");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));
        let seed = 24;
        let mut legacy = finding(seed, TclVersion::V9_0);
        legacy.tcl_version = None;
        legacy.replay_metadata_version = None;
        let dir = pair_findings_dir(&base, args.reference, args.subject, None);
        Registry::open(&dir).unwrap().record(&legacy).unwrap();

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(error.contains("predates replay-safe campaign metadata"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn explicit_replay_refuses_a_finding_from_a_different_release() {
        let base = replay_tmp("explicit-mismatch");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));
        let seed = 25;
        let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V8_6));
        Registry::open(&dir)
            .unwrap()
            .record(&finding(seed, TclVersion::V8_6))
            .unwrap();

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(error.contains("records Tcl 8.6"));
        assert!(error.contains("requested Tcl 9.0"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn explicit_replay_selects_its_release_when_the_seed_exists_at_multiple_releases() {
        let base = replay_tmp("explicit-duplicate");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));
        let seed = 27;
        for tcl_version in [TclVersion::V8_6, TclVersion::V9_0] {
            let dir = pair_findings_dir(&base, args.reference, args.subject, Some(tcl_version));
            Registry::open(&dir)
                .unwrap()
                .record(&finding(seed, tcl_version))
                .unwrap();
        }

        let selection = select_replay(&base, args, seed).unwrap();
        assert_eq!(selection.tcl_version, Some(TclVersion::V9_0));
        assert_eq!(
            selection
                .finding
                .as_ref()
                .and_then(|finding| finding.campaign_tcl_version().ok().flatten()),
            Some(TclVersion::V9_0),
        );
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn explicit_replay_refuses_metadata_mismatched_to_its_release_namespace() {
        let base = replay_tmp("explicit-metadata-mismatch");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));
        let seed = 28;
        let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V9_0));
        let mut mismatched = finding(seed, TclVersion::V8_6);
        mismatched.replay_metadata_version = None;
        Registry::open(&dir).unwrap().record(&mismatched).unwrap();

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(error.contains("stored under Tcl 9.0"));
        assert!(error.contains("records Tcl 8.6"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn replay_refuses_an_invalid_pinned_release_even_without_the_marker() {
        let base = replay_tmp("invalid-markerless-pinned");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 281;
        let mut invalid = finding(seed, TclVersion::V8_6);
        invalid.tcl_version = Some("8.7".to_owned());
        invalid.replay_metadata_version = None;
        let dir = pair_findings_dir(&base, args.reference, args.subject, Some(TclVersion::V8_6));
        Registry::open(&dir).unwrap().record(&invalid).unwrap();

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(error.contains("unrecognised recorded Tcl release"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn explicit_replay_allows_an_arbitrary_seed_absent_from_every_registry() {
        let base = replay_tmp("arbitrary-seed");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0));

        let selection = select_replay(&base, args, 26).unwrap();
        assert_eq!(selection.tcl_version, Some(TclVersion::V9_0));
        assert!(selection.finding.is_none());
        assert!(
            !base.exists(),
            "an arbitrary replay must not create registries"
        );
    }

    #[test]
    fn replay_refuses_a_seed_which_exists_at_multiple_releases() {
        let base = replay_tmp("ambiguous");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let seed = 29;
        for tcl_version in [TclVersion::V8_6, TclVersion::V9_0] {
            let dir = pair_findings_dir(&base, args.reference, args.subject, Some(tcl_version));
            Registry::open(&dir)
                .unwrap()
                .record(&finding(seed, tcl_version))
                .unwrap();
        }

        let error = select_replay(&base, args, seed).unwrap_err();
        assert!(error.contains("multiple Tcl configurations"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn default_summary_discovers_each_release_namespace_without_seed_collisions() {
        let base = replay_tmp("summary-all");
        let args = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        let shared_seed = 30;
        for tcl_version in [TclVersion::V8_6, TclVersion::V9_0] {
            let dir = pair_findings_dir(&base, args.reference, args.subject, Some(tcl_version));
            Registry::open(&dir)
                .unwrap()
                .record(&finding(shared_seed, tcl_version))
                .unwrap();
        }
        let mut unpinned = finding(31, TclVersion::V9_0);
        unpinned.tcl_version = None;
        let legacy_dir = pair_findings_dir(&base, args.reference, args.subject, None);
        Registry::open(&legacy_dir)
            .unwrap()
            .record(&unpinned)
            .unwrap();

        let selected = summary_registry_dirs(&base, args);
        assert_eq!(
            selected
                .iter()
                .map(|(release, _)| *release)
                .collect::<Vec<_>>(),
            vec![None, Some(TclVersion::V8_6), Some(TclVersion::V9_0)],
        );
        assert_eq!(
            selected
                .iter()
                .map(|(_, dir)| Registry::open(dir)
                    .unwrap()
                    .summary()
                    .unwrap()
                    .values()
                    .sum::<usize>())
                .sum::<usize>(),
            3,
            "same seed is kept as two findings because the release namespace is part of identity",
        );
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn explicit_summary_selects_only_its_release_namespace() {
        let base = replay_tmp("summary-explicit");
        let unpinned = pair_args(Engine::RuntimeRust, Engine::Tclvm, None);
        for tcl_version in [TclVersion::V8_6, TclVersion::V9_0] {
            let dir = pair_findings_dir(
                &base,
                unpinned.reference,
                unpinned.subject,
                Some(tcl_version),
            );
            Registry::open(&dir)
                .unwrap()
                .record(&finding(32, tcl_version))
                .unwrap();
        }
        let selected = summary_registry_dirs(
            &base,
            pair_args(Engine::RuntimeRust, Engine::Tclvm, Some(TclVersion::V9_0)),
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].0, Some(TclVersion::V9_0));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn default_pair_keeps_the_plain_findings_dir() {
        // Issue #1313: the original `tclvm` subject / `tclsh` reference pair
        // must keep writing to `<findings>/` so existing registries need no
        // migration when other pairs are added.
        let base = Path::new("fuzz-findings");
        assert_eq!(
            pair_findings_dir(base, Engine::Tclsh, Engine::Tclvm, None),
            base.to_owned()
        );
    }

    #[test]
    fn other_pairs_are_namespaced_so_seeds_never_collide() {
        let base = Path::new("fuzz-findings");
        assert_eq!(
            pair_findings_dir(base, Engine::Tclsh, Engine::RuntimeRust, None),
            base.join("runtime-rust-vs-tclsh")
        );
        assert_eq!(
            pair_findings_dir(base, Engine::RuntimeRust, Engine::Tclvm, None),
            base.join("tclvm-vs-runtime-rust")
        );
        // The same two engines swapped is a *different* registry — subject and
        // reference are not interchangeable.
        assert_ne!(
            pair_findings_dir(base, Engine::Tclsh, Engine::RuntimeRust, None),
            pair_findings_dir(base, Engine::RuntimeRust, Engine::Tclsh, None)
        );
    }

    #[test]
    fn release_namespaces_keep_same_pair_seeds_separate() {
        let base = Path::new("fuzz-findings");
        let old = pair_findings_dir(
            base,
            Engine::RuntimeRust,
            Engine::Tclvm,
            Some(TclVersion::V8_6),
        );
        let new = pair_findings_dir(
            base,
            Engine::RuntimeRust,
            Engine::Tclvm,
            Some(TclVersion::V9_0),
        );
        assert_eq!(old, base.join("tclvm-vs-runtime-rust/tcl8.6"));
        assert_eq!(new, base.join("tclvm-vs-runtime-rust/tcl9.0"));
        assert_ne!(old, new, "a seed must not deduplicate across releases");
    }

    #[test]
    fn every_engine_names_the_flag_that_overrides_it() {
        for engine in [Engine::Tclvm, Engine::Tclsh, Engine::RuntimeRust] {
            assert_eq!(cli_flag_for(engine), engine.label());
        }
    }

    #[test]
    fn a_campaign_pin_reaches_both_sides_of_an_emulated_pair() {
        let mut pair = test_pair(Engine::RuntimeRust, Engine::Tclvm);
        pair.apply_tcl_version(TclVersion::V8_6);
        assert_eq!(pair.tcl_version, Some(TclVersion::V8_6));
        assert_eq!(
            pair.reference_args,
            vec![
                "--quiet".to_owned(),
                "--tcl-version".to_owned(),
                "8.6".to_owned(),
            ],
            "reference must receive the same release pin",
        );
        assert_eq!(
            pair.subject_args,
            vec!["--tcl-version".to_owned(), "8.6".to_owned()],
            "subject must receive the same release pin",
        );
    }

    #[test]
    fn a_fixed_release_engine_refuses_when_its_selected_binary_cannot_honour_the_pin() {
        let mut pair = test_pair(Engine::Tclsh, Engine::Tclvm);
        pair.apply_tcl_version(TclVersion::V8_6);
        assert_eq!(pair.tcl_version, Some(TclVersion::V8_6));
        assert!(pair.reference_args.is_empty());
        assert_eq!(
            pair.subject_args,
            vec!["--tcl-version".to_owned(), "8.6".to_owned()]
        );
        let versions = version::PairVersions {
            tcl_version: Some(TclVersion::V8_6),
            reference: Some(version::EngineVersion::parse("9.0.4")),
            subject: Some(version::EngineVersion::parse("8.6.16")),
        };
        let error = verify_requested_tcl_version(&versions).unwrap_err();
        assert!(error.contains("reference engine reports Tcl 9.0.4"));
    }

    #[test]
    fn identical_backend_pairs_are_rejected() {
        let cli = Cli::parse_from(["tcl-fuzz", "run"]);
        let pair = PairArgs {
            reference: Engine::Tclvm,
            subject: Engine::Tclvm,
            tcl_version: None,
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
    fn requested_release_must_be_reported_by_both_backends() {
        let versions = version::PairVersions {
            tcl_version: Some(TclVersion::V8_6),
            reference: Some(version::EngineVersion::parse("8.6.16")),
            subject: Some(version::EngineVersion::parse("8.6.16")),
        };
        assert!(verify_requested_tcl_version(&versions).is_ok());
        let ignored = version::PairVersions {
            subject: Some(version::EngineVersion::parse("9.0.4")),
            ..versions
        };
        assert!(verify_requested_tcl_version(&ignored).is_err());
    }

    #[test]
    fn replay_patch_drift_warning_compares_both_recorded_backends() {
        let versions = version::PairVersions {
            tcl_version: Some(TclVersion::V9_0),
            reference: Some(version::EngineVersion::parse("9.0.1")),
            subject: Some(version::EngineVersion::parse("9.0.4")),
        };
        assert!(!replay_versions_changed(
            Some("9.0.1"),
            Some("9.0.4"),
            &versions,
        ));
        assert!(replay_versions_changed(
            Some("9.0.1"),
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
