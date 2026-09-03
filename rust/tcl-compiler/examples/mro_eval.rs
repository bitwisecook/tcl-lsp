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

//! `mro_eval` — the `TclOO` MRO/class-lattice **experiment harness**.
//!
//! Runs the prototype dispatch resolver
//! ([`tcl_compiler::analyser::class_lattice`]) over a corpus of Tcl files
//! and reports the metrics that decide whether the object→class lattice is
//! worth shipping:
//!
//! * **⊤-rate** — the fraction of `$obj method` sites that collapse to ⊙
//!   abstention (the make-or-break number), broken down by collapse reason.
//! * **Resolution split** — of the sites we *did* bind to a class, how many
//!   also resolved the method (vs. named the class but not the method — the
//!   W308 candidate set).
//! * **Ablations** — MRO-only (single class, no join) → +join → +mixins →
//!   +cross-file index, so the marginal value of each layer is visible.
//! * **Cost** — added analysis time (resolver overhead over the base walk).
//!
//! Usage:
//!
//! ```text
//! cargo run -p tcl-compiler --example mro_eval -- \
//!     experiments/corpus tmp/tcllib-2.0 tmp/tcl8.6.16/tests
//! ```
//!
//! With no arguments it scans a default corpus set (the vendored
//! `experiments/corpus`, tcllib, and the Tcl 8.6/9.0 `oo` tests) when
//! present.  Nothing here touches shipping diagnostics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tcl_compiler::analyser::class_lattice::{
    AblationConfig, DispatchStats, DispatchVerdict, NsContext, TopReason, analyse_dispatch,
};
use tcl_compiler::analyser::state::Analyser;
use tcl_compiler::analyser::types::ClassDef;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_registry::CommandRegistry;

/// Lossless-for-realistic-counts `usize`→`f64` conversion used for the
/// percentage/throughput metrics below (counts never exceed `u32::MAX`).
fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// One ablation row.
struct Ablation {
    label: &'static str,
    cfg: AblationConfig,
    /// When true, resolve against the corpus-merged class index rather
    /// than the file's own classes.
    cross_file: bool,
}

fn ablations() -> Vec<Ablation> {
    vec![
        Ablation {
            label: "A0 MRO-only (single-class, no join)",
            cfg: AblationConfig {
                join: false,
                mixins_filters: false,
                cross_file: false,
            },
            cross_file: false,
        },
        Ablation {
            label: "A1 +join (CFG-merge lattice)",
            cfg: AblationConfig {
                join: true,
                mixins_filters: false,
                cross_file: false,
            },
            cross_file: false,
        },
        Ablation {
            label: "A2 +mixins/filters (full MRO)",
            cfg: AblationConfig {
                join: true,
                mixins_filters: true,
                cross_file: false,
            },
            cross_file: false,
        },
        Ablation {
            label: "A3 +cross-file index (FULL)",
            cfg: AblationConfig {
                join: true,
                mixins_filters: true,
                cross_file: true,
            },
            cross_file: true,
        },
    ]
}

/// Recursively collect `.tcl` / `.test` files under `root`.
fn collect_tcl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip VCS / build dirs.
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(name, ".git" | "target" | "node_modules" | ".venv") {
                continue;
            }
            collect_tcl_files(&path, out);
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext == "tcl" || ext == "test")
        {
            out.push(path);
        }
    }
}

/// Per-file analysis product we need for resolution.
struct Analysed {
    cu: CompilationUnit,
    sites: Vec<tcl_compiler::analyser::state::VarCommandSite>,
    classes: HashMap<String, ClassDef>,
    ns: NsContext,
    lines: usize,
}

/// Analyse one file; `None` on read error / oversize / panic.
fn analyse_file(path: &Path, reg: &CommandRegistry) -> Option<Analysed> {
    let src = std::fs::read_to_string(path).ok()?;
    if src.len() > 2_000_000 {
        return None; // skip pathologically large files
    }
    let lines = src.lines().count();
    // The analyser walk collects `var_command_sites` and populates
    // `all_classes`; contain any adversarial-input panic per file.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut a = Analyser::new();
        // `analyse` moves the `AnalysisResult` out of the analyser, so the
        // class index lives on the returned value, not on `a.result`.  The
        // deferred `var_command_sites` stay on the analyser field.
        let result = a.analyse(&src, "tcl8.6");
        let cu = CompilationUnit::build_for(&src, reg, false).with_interprocedural(
            reg,
            Some(tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()),
        );
        let ns = NsContext::from_result(&result);
        Analysed {
            cu,
            sites: a.var_command_sites.clone(),
            classes: result.all_classes,
            ns,
            lines,
        }
    }))
    .ok()
}

/// Render a stats block as a compact table row set.
fn print_stats(label: &str, s: &DispatchStats) {
    let pct = |n: usize| {
        if s.total_sites == 0 {
            0.0
        } else {
            100.0 * as_f64(n) / as_f64(s.total_sites)
        }
    };
    println!("\n### {label}");
    println!(
        "  sites={}  resolved={} ({:.1}%)  ⊤={} ({:.1}%)  top-rate={:.3}",
        s.total_sites,
        s.resolved,
        pct(s.resolved),
        s.abstain,
        pct(s.abstain),
        s.top_rate()
    );
    println!(
        "  of resolved: method-known={}  method-unknown(W308-cand)={}",
        s.method_known, s.method_unknown
    );
    if !s.top_by_reason.is_empty() {
        println!("  ⊤ breakdown:");
        // Stable order following the taxonomy.
        for r in TopReason::all() {
            if let Some(n) = s.top_by_reason.get(r.as_str()) {
                let note = if r == TopReason::Unknown && s.extern_receiver_abstains > 0 {
                    format!(
                        "  [of which {} are extern/param receivers → need interprocedural flow]",
                        s.extern_receiver_abstains
                    )
                } else {
                    String::new()
                };
                println!(
                    "    {:<18} {:>6}  ({:.1}% of all sites){note}",
                    r.as_str(),
                    n,
                    pct(*n)
                );
            }
        }
    }
}

/// Cached products from Pass A plus the corpus-wide roll-up metrics.
struct PassA {
    analysed: Vec<(PathBuf, Analysed)>,
    merged_index: HashMap<String, ClassDef>,
}

/// Qualitative + quantitative products from Pass B.
struct PassB {
    per_ablation: Vec<DispatchStats>,
    resolve_time: Duration,
    sample_abstain: Vec<(String, String, TopReason)>,
    sample_resolved: Vec<(String, String, String)>,
}

/// Resolve the corpus roots from CLI args (or the default set).
fn corpus_roots(args: &[String]) -> Vec<PathBuf> {
    if args.is_empty() {
        // Default corpus set (only those that exist).
        [
            "experiments/corpus",
            "tmp/tcllib-2.0",
            "tmp/tcl8.6.16/tests",
            "tmp/tcl9.0.4/tests",
        ]
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
    } else {
        args.iter().map(PathBuf::from).collect()
    }
}

/// Pass A: analyse every file, cache the product, build the corpus-merged
/// class index for the cross-file ablation, and print the roll-up summary.
fn run_pass_a(files: &[PathBuf], reg: &CommandRegistry) -> PassA {
    let mut analysed: Vec<(PathBuf, Analysed)> = Vec::new();
    let mut merged_index: HashMap<String, ClassDef> = HashMap::new();
    let mut total_lines = 0usize;
    let mut analyse_time = Duration::ZERO;
    let mut files_with_classes = 0usize;
    let mut files_with_sites = 0usize;
    for path in files {
        let t0 = Instant::now();
        let Some(a) = analyse_file(path, reg) else {
            continue;
        };
        analyse_time += t0.elapsed();
        total_lines += a.lines;
        if !a.classes.is_empty() {
            files_with_classes += 1;
        }
        if !a.sites.is_empty() {
            files_with_sites += 1;
        }
        for (k, v) in &a.classes {
            merged_index.entry(k.clone()).or_insert_with(|| v.clone());
        }
        analysed.push((path.clone(), a));
    }
    println!(
        "analysed {} files, {} total lines ({} with classes, {} with $obj-dispatch sites)",
        analysed.len(),
        total_lines,
        files_with_classes,
        files_with_sites,
    );
    println!("merged class index: {} classes", merged_index.len());
    println!(
        "base analysis time: {:.2}s ({:.2} ms/file, {:.1} KLOC/s)",
        analyse_time.as_secs_f64(),
        1000.0 * analyse_time.as_secs_f64() / as_f64(analysed.len().max(1)),
        as_f64(total_lines) / 1000.0 / analyse_time.as_secs_f64().max(1e-9),
    );
    PassA {
        analysed,
        merged_index,
    }
}

/// Pass B: run every ablation over the cached products, aggregating stats and
/// a qualitative sample of sites from the FULL config.
fn run_pass_b(ablations: &[Ablation], pa: &PassA) -> PassB {
    let mut resolve_time = Duration::ZERO;
    let mut per_ablation: Vec<DispatchStats> = Vec::new();
    let mut sample_abstain: Vec<(String, String, TopReason)> = Vec::new();
    let mut sample_resolved: Vec<(String, String, String)> = Vec::new();

    for (ai, ab) in ablations.iter().enumerate() {
        let mut agg = DispatchStats::default();
        for (_path, a) in &pa.analysed {
            let index = if ab.cross_file {
                &pa.merged_index
            } else {
                &a.classes
            };
            let t0 = Instant::now();
            let (reports, stats) = analyse_dispatch(
                &a.cu,
                index,
                &a.sites,
                &a.ns,
                &ab.cfg,
                tcl_lexer::LexerConfig::default(),
            );
            resolve_time += t0.elapsed();
            agg.merge(&stats);
            // Sample from the FULL ablation only.
            if ai == ablations.len() - 1 {
                for r in reports {
                    match &r.verdict {
                        DispatchVerdict::Abstain(reason) if sample_abstain.len() < 20 => {
                            sample_abstain.push((
                                r.var.clone(),
                                r.method.clone().unwrap_or_default(),
                                *reason,
                            ));
                        }
                        DispatchVerdict::Resolved { classes, .. } if sample_resolved.len() < 20 => {
                            sample_resolved.push((
                                r.var.clone(),
                                r.method.clone().unwrap_or_default(),
                                classes.iter().cloned().collect::<Vec<_>>().join("|"),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
        per_ablation.push(agg);
    }
    PassB {
        per_ablation,
        resolve_time,
        sample_abstain,
        sample_resolved,
    }
}

/// Print the ablation tables, marginal deltas, cost, and qualitative samples.
fn print_report(ablations: &[Ablation], pa: &PassA, pb: &PassB) {
    println!("\n## Ablation results");
    for (ab, stats) in ablations.iter().zip(&pb.per_ablation) {
        print_stats(ab.label, stats);
    }

    // Marginal deltas in resolved sites between successive ablations.
    println!("\n## Marginal value of each layer (Δ resolved sites vs previous)");
    let mut prev = 0i64;
    for (ab, stats) in ablations.iter().zip(&pb.per_ablation) {
        let resolved = i64::try_from(stats.resolved).unwrap_or(i64::MAX);
        let delta = resolved - prev;
        println!("  {:<40} resolved={:>5}  Δ={:+}", ab.label, resolved, delta);
        prev = resolved;
    }

    println!(
        "\n## Cost\n  resolver time (all ablations): {:.3}s over {} files × {} ablations\n  ≈ {:.3} ms/file/ablation (added on top of base walk)",
        pb.resolve_time.as_secs_f64(),
        pa.analysed.len(),
        ablations.len(),
        1000.0 * pb.resolve_time.as_secs_f64() / as_f64(pa.analysed.len().max(1) * ablations.len()),
    );

    println!("\n## Sample abstentions (FULL config, ⊤ with reason)");
    for (var, method, reason) in pb.sample_abstain.iter().take(20) {
        println!("  ${var} {method:<16} → ⊤ {reason}");
    }
    println!("\n## Sample resolutions (FULL config)");
    for (var, method, classes) in pb.sample_resolved.iter().take(20) {
        println!("  ${var} {method:<16} → {classes}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let roots = corpus_roots(&args);

    if roots.is_empty() {
        eprintln!("no corpus roots found; pass directories as arguments");
        std::process::exit(1);
    }

    println!("# mro_eval — TclOO class-lattice dispatch resolver experiment");
    println!(
        "corpus roots: {:?}",
        roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );

    let mut files = Vec::new();
    for r in &roots {
        collect_tcl_files(r, &mut files);
    }
    files.sort();
    println!("discovered {} Tcl files", files.len());

    let reg = CommandRegistry::build_default();

    let pa = run_pass_a(&files, &reg);
    let ablations = ablations();
    let pb = run_pass_b(&ablations, &pa);

    print_report(&ablations, &pa, &pb);
}
