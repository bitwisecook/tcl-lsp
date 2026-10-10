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

//! The value-transfer performance report
//! (`docs/design/compiler/value-evaluation.md` § *Budgets and cancellation*:
//! "performance acceptance compares the unchanged tree, direct-core
//! evaluation, expression evaluation, and declared execution on the same
//! workloads").
//!
//! Two parts, printed as tables:
//!
//! 1. **Large literal command substitutions** — `array set m [list 0 1 …]`
//!    with thousands of words, the shape of tcllib's `stringprep_data.tcl`:
//!    the whole analysis and the optimiser's fixpoint against the parts the
//!    value transfers own, the constant-substitution engine's fold and the
//!    `list` route over the same literal words, so the share the routes take
//!    is measured rather than assumed.
//! 2. **The comparison** — one procedure of the same statement shape whose
//!    call is a command with no route (the unchanged tree), a direct core
//!    (`string length`), the expression engine (`expr`) and a declared
//!    implementation (the design's executable example, `tenant::label`, on
//!    the bounded host): cold host setup and first build, warm builds, a
//!    rebuild of the same text (the hook memo's hits), a rebuild with one
//!    input changed, the route entries the solver made (`RouteTally`), the
//!    latency of a cancelled evaluation's decline, and the resident memory
//!    the build kept.
//!
//! `cargo bench -p tcl-compiler --bench value_transfers` runs the full
//! sizes; `cargo test -p tcl-compiler --bench value_transfers` runs the same
//! report at smoke sizes (set `VT_BENCH_FULL=1` for the full ones), under
//! the test profile.

use std::fmt::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::value_transfer::RouteTally;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::{
    Budget, EvalAnswer, LiteralInputs, evaluate_literal, resolve_semantics,
};

/// The design's executable example, a workspace pack whose `tenant::label`
/// is a declared implementation.
const TENANT_PACK: &str = include_str!("../tests/fixtures/value_transfers/tenant.tclspec");

/// The dialect every workload is analysed under.
const DIALECT: &str = "tcl9.0";

fn main() {
    let full =
        std::env::args().any(|arg| arg == "--bench") || std::env::var_os("VT_BENCH_FULL").is_some();
    let (sizes, statements, runs): (&[usize], usize, usize) = if full {
        (&[500, 1000, 2000, 4000], 200, 5)
    } else {
        (&[200, 400], 40, 3)
    };
    println!("# Value-transfer performance report\n");
    println!(
        "Profile: {}; dialect `{DIALECT}`; {runs} warm runs per row (the median).\n",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "optimised"
        }
    );
    large_literals(sizes);
    comparison(statements, runs);
}

/// The median of `samples`.
fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples.get(samples.len() / 2).copied().unwrap_or_default()
}

/// Milliseconds, for a table cell.
fn ms(duration: Duration) -> String {
    format!("{:.2}", duration.as_secs_f64() * 1000.0)
}

/// Time `run`, once.
fn timed<T>(run: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let value = run();
    (value, start.elapsed())
}

/// The process's resident set, in KiB, where `/proc` reports it.
fn resident_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
}

/// Part 1: one literal substitution of `size` words, through the analysis,
/// the optimiser, the constant-substitution engine and the route.
fn large_literals(sizes: &[usize]) {
    println!("## Large literal command substitutions\n");
    println!(
        "`namespace eval ::x {{variable m; array set m [list 0 1 … N-1]}}`: the analysis \
         (`CompilationUnit::build_for_dialect`), the optimiser's fixpoint \
         (`optimise_source_multipass`), and within them the constant-substitution engine's \
         fold of the substitution (`ConstSubstCtx::fold_cmd_subst`) and the `list` route over \
         the same words (`evaluate_literal`).\n"
    );
    println!("| N | analysis ms | optimiser ms | engine fold ms | route ms | route share % |");
    println!("|---|---|---|---|---|---|");
    let registry = Arc::clone(tcl_registry::model::ingress::static_context_for(DIALECT).commands());
    let profile = tcl_dialect::DialectProfile::find(DIALECT);
    for &size in sizes {
        let words: Vec<String> = (0..size).map(|n| n.to_string()).collect();
        let interior = format!("list {}", words.join(" "));
        let source = format!("namespace eval ::x {{\nvariable m\narray set m [{interior}]\n}}\n");
        let ((), analysis) = timed(|| {
            let unit = CompilationUnit::build_for_dialect(&source, &registry, false, DIALECT);
            drop(unit);
        });
        let ((), optimiser) = timed(|| {
            let (text, _) =
                tcl_compiler::optimiser::optimise_source_multipass(&source, &registry, profile, 8);
            drop(text);
        });
        let trusts = |_: &str| true;
        let lookup = |_: &str| None;
        let engine = tcl_compiler::const_subst::ConstSubstCtx {
            registry: &registry,
            resolution_namespace: "::",
            version: profile.and_then(tcl_dialect::DialectProfile::const_fold_version),
            defining_class: None,
            trusts: &trusts,
            lookup_var: &lookup,
        };
        let (folded, fold) = timed(|| engine.fold_cmd_subst(&interior));
        let spec = registry.get("list").expect("list");
        let resolved = resolve_semantics(spec, None, None);
        let semantics = resolved.semantics().expect("list declares its route");
        let args: Vec<&str> = words.iter().map(String::as_str).collect();
        let version = profile.and_then(tcl_dialect::DialectProfile::const_fold_version);
        let (routed, route) = timed(|| evaluate_literal(semantics, "list", None, &args, version));
        assert_eq!(folded, routed, "the engine's fold is the route's answer");
        let share = 100.0 * route.as_secs_f64() / optimiser.as_secs_f64().max(f64::EPSILON);
        println!(
            "| {size} | {} | {} | {} | {} | {share:.2} |",
            ms(analysis),
            ms(optimiser),
            ms(fold),
            ms(route)
        );
    }
    println!();
}

/// One workload's variant: the call each statement makes, and the
/// registry and host it runs under.
struct Variant {
    name: &'static str,
    call: fn(usize) -> String,
}

const VARIANTS: &[Variant] = &[
    Variant {
        name: "unchanged tree (no route)",
        call: |at| format!("opaque::call w{at}"),
    },
    Variant {
        name: "direct core (`string length`)",
        call: |at| format!("string length w{at}"),
    },
    Variant {
        name: "expression (`expr`)",
        call: |at| format!("expr {{{at} + 1}}"),
    },
    Variant {
        name: "declared implementation (`tenant::label`)",
        call: |at| format!("tenant::label w{at}"),
    },
];

/// The workload: one procedure of `statements` statements, each setting a
/// variable from the variant's call, the last returning them all.
fn workload(variant: &Variant, statements: usize, changed: Option<usize>) -> String {
    let mut body = String::new();
    for at in 0..statements {
        let call = (variant.call)(if changed == Some(at) {
            at + 1_000_000
        } else {
            at
        });
        let _ = writeln!(body, "    set v{at} [{call}]");
    }
    body.push_str("    return [list");
    for at in 0..statements {
        let _ = write!(body, " $v{at}");
    }
    body.push_str("]\n");
    format!("proc p {{}} {{\n{body}}}\n")
}

/// The route entries the solver made over the procedure.
fn tally(unit: &CompilationUnit) -> RouteTally {
    unit.procedures
        .get("::p")
        .map(|procedure| procedure.sccp.route_tally)
        .unwrap_or_default()
}

/// Part 2: the four variants side by side.
fn comparison(statements: usize, runs: usize) {
    println!("## The comparison\n");
    println!(
        "One procedure of {statements} `set vN [CALL]` statements and a `return` of them all, \
         per route family. *Cold* is the first build in the process, the declared row's host \
         setup (pack load, hook plan, bounded host) included; *warm* the median of {runs} \
         rebuilds; *same text* a rebuild whose evaluations the hook memo can answer; *changed* \
         a rebuild with one call's input changed, the incremental edit; *routes* the solver's \
         route entries (`RouteTally`: direct · expression · implementation); *cancelled µs* \
         how long one evaluation of the command under a cancelled budget takes to decline (— \
         where it declares no route); *RSS KiB* the \
         resident set the warm builds left grown.\n"
    );
    println!(
        "| variant | cold ms | warm ms | same text ms | changed ms | routes | cancelled µs | RSS KiB |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    for variant in VARIANTS {
        let declared = variant.name.starts_with("declared");
        let before = resident_kib();
        let (registry, setup) = timed(|| registry_for(declared));
        let source = workload(variant, statements, None);
        let (unit, first) =
            timed(|| CompilationUnit::build_for_dialect(&source, &registry, false, DIALECT));
        let routes = tally(&unit);
        drop(unit);
        let warm = median(
            (0..runs)
                .map(|_| {
                    timed(|| CompilationUnit::build_for_dialect(&source, &registry, false, DIALECT))
                        .1
                })
                .collect(),
        );
        let ((), same) = timed(|| {
            drop(CompilationUnit::build_for_dialect(
                &source, &registry, false, DIALECT,
            ));
        });
        let edited = workload(variant, statements, Some(statements / 2));
        let ((), changed) = timed(|| {
            drop(CompilationUnit::build_for_dialect(
                &edited, &registry, false, DIALECT,
            ));
        });
        let cancelled = cancelled_decline(&registry, variant);
        let grown = match (before, resident_kib()) {
            (Some(before), Some(after)) => after.saturating_sub(before).to_string(),
            _ => "—".to_owned(),
        };
        println!(
            "| {} | {} | {} | {} | {} | {} · {} · {} | {} | {grown} |",
            variant.name,
            ms(setup + first),
            ms(warm),
            ms(same),
            ms(changed),
            routes.direct,
            routes.expression,
            routes.implementation,
            cancelled.map_or_else(
                || "—".to_owned(),
                |(latency, declined)| format!(
                    "{:.1}{}",
                    latency.as_secs_f64() * 1e6,
                    if declined { "" } else { " (answered)" }
                )
            ),
        );
    }
    println!();
}

/// The registry a variant runs under: the dialect's, and for the declared
/// implementation the workspace pack loaded as the language server loads
/// one, its hook plan published and this thread's bounded host built.
fn registry_for(declared: bool) -> Arc<CommandRegistry> {
    if declared {
        let packs = tcl_spectcl::pack::load_in_memory(vec![(
            tcl_spectcl::PackFile {
                tier: tcl_spectcl::Tier::Workspace,
                path: std::path::PathBuf::from("/workspace/.tcl-lsp/tenant.tclspec"),
                origin: tcl_spectcl::discovery::Origin::DotDir,
                dependency_tier: None,
            },
            TENANT_PACK.to_owned(),
        )]);
        tcl_spectcl::hooks::publish(&packs);
        tcl_spectcl::hooks::ensure_thread_host();
        tcl_spectcl::install::registry_for_dialect_with_packs(DIALECT, &packs)
    } else {
        Arc::clone(tcl_registry::model::ingress::static_context_for(DIALECT).commands())
    }
}

/// How long one evaluation of the variant's command takes to answer under a
/// cancelled budget, and whether the answer is the decline: every route is
/// admitted through `ConstOps::admit`, which refuses a cancelled request.
fn cancelled_decline(registry: &CommandRegistry, variant: &Variant) -> Option<(Duration, bool)> {
    let call = (variant.call)(7);
    let words: Vec<&str> = call.split_whitespace().collect();
    let (&head, rest) = words.split_first()?;
    let spec = registry.get(head)?;
    let (sub, args) = match spec
        .subcommands
        .iter()
        .find(|sub| rest.first() == Some(&sub.name))
    {
        Some(sub) => (Some(sub), &rest[1..]),
        None => (None, rest),
    };
    let resolved = resolve_semantics(spec, sub, None);
    let semantics = resolved.semantics()?;
    let profile = tcl_dialect::DialectProfile::find(DIALECT);
    let inputs = LiteralInputs::new(spec.name, sub.map(|sub| sub.name), args, profile);
    let mut budget = Budget::evaluation();
    budget
        .cancelled
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let (answer, latency) = timed(|| semantics.evaluate(&inputs, &mut budget));
    Some((latency, !matches!(answer, EvalAnswer::Evaluated(_))))
}
