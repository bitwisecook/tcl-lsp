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

//! Rebased function units: shifted spans match a fresh build.

use super::*;

// lattice_rebase.rs — memoised-unit offset rebase across a shift
//
// Structural note: `lattice_rebase` shifts every absolute span in a memoised
// `FunctionUnit` when a cached offset-0 body is reused at a new file offset.
// The contract (documented in the module) is that the *span-carrying* parts of
// the rebased unit — the CFG (block statements + terminators + loop-node spans),
// the SSA statement clones, and the SCCP constant-branch spans — are
// BYTE-IDENTICAL to a fresh whole-file build at the new position.  We drive the
// rebase through the public `build_for_memoized` seam and assert exactly those
// three fields (`cfg` / `ssa` / `sccp`) equal a fresh build.  (The span-free
// `def_use` chains are NOT compared: their `Vec<UseSite>` ordering is a build-
// path artifact independent of the rebase — they carry no spans for the rebase
// to touch, and the existing diagnostics shift-test sidesteps the same way by
// comparing diagnostics rather than the raw unit.)  This exercises the Foreach /
// Try / UpFrame / constant-branch / loop-node statement variants the existing
// shift-test (`memoized_compilation_unit_shift_correctness`) does not cover.

/// Assert the *span-carrying* parts of two function units are identical: the
/// CFG, the SSA, and the SCCP result are exactly what `lattice_rebase` shifts,
/// so equality here proves the rebase reproduced a fresh build's positions.
fn assert_span_carrying_eq(got: &FunctionUnit, want: &FunctionUnit, ctx: &str) {
    assert_eq!(
        got.cfg, want.cfg,
        "{ctx}: rebased CFG must match a fresh build"
    );
    assert_eq!(
        got.ssa, want.ssa,
        "{ctx}: rebased SSA must match a fresh build"
    );
    assert_eq!(
        got.sccp, want.sccp,
        "{ctx}: rebased SCCP constant-branch spans must match a fresh build"
    );
}

/// The registry and profile every rebase build runs under: `D`'s, on the
/// memoised and the fresh path alike, so the lattice evaluates under a
/// target — the routes that fold and a case list's selection need one.
fn rebase_target() -> (
    &'static CommandRegistry,
    &'static tcl_dialect::DialectProfile,
) {
    (
        static_context_for(D).commands(),
        tcl_registry::model::ingress::resolve_environment(D).analyser_profile(),
    )
}

/// A fresh whole-file build of `source` under the rebase target: the ground
/// truth a rebased unit is compared against.
fn fresh_unit(source: &str) -> CompilationUnit {
    let (registry, profile) = rebase_target();
    CompilationUnit::build_for_profile(source, registry, false, profile)
}

/// Build a `FunctionUnit` for every proc via a position-independent memo whose
/// cache is seeded on `base` and reused on `shifted`, so the shifted procs hit
/// and the builder rebases their offset-0 units to the new positions. Each
/// memoised unit is built as the language server's memo builds one: under the
/// request's analysis context and the command trust its snapshot records.
fn rebased_units(base: &str, shifted: &str) -> (CompilationUnit, CompilationUnit) {
    use std::collections::{BTreeSet, HashMap, HashSet};
    let (registry, profile) = rebase_target();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let mut cache: HashMap<String, FunctionUnit> = HashMap::new();
    let build = |s: &str, cache: &mut HashMap<String, FunctionUnit>| -> CompilationUnit {
        CompilationUnit::build_for_memoized(
            s,
            tcl_compiler::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            &mut |req: &tcl_compiler::compilation_unit::LatticeRequest<'_>| -> FunctionUnit {
                let key = format!(
                    "{}\u{0}{:?}\u{0}{:?}\u{0}{:?}",
                    req.qname, req.body, req.params, req.param_constants
                );
                if let Some(fu) = cache.get(&key) {
                    return fu.clone();
                }
                let prepared = tcl_compiler::cfg_builder::PreparedCfgContext::from_source_input(
                    (
                        req.upvar_procs.clone(),
                        req.proc_params.clone(),
                        req.global_write_procs.clone(),
                        req.command_bindings.clone(),
                        req.procedure_summary_owners.clone(),
                    ),
                    registry,
                    req.source_metadata_input,
                );
                let cfg = tcl_compiler::cfg_builder::build_cfg_function_with_prepared_context(
                    req.qname,
                    req.body,
                    true,
                    registry,
                    req.plain_command_dispatch,
                    &prepared,
                    req.lexer_config,
                );
                let pc =
                    tcl_compiler::compilation_unit::decode_param_constants(req.param_constants);
                let known_classes: HashSet<String> = req.known_classes.iter().cloned().collect();
                let traced_variables: BTreeSet<String> =
                    req.traced_variables.iter().cloned().collect();
                let command_trust = req.analysis_context.bindings.to_mutations();
                let fu = FunctionUnit::build_with_param_constants_and_classes_under(
                    req.qname,
                    cfg,
                    req.params,
                    tcl_compiler::compilation_unit::UnitDialect {
                        registry,
                        config: req.lexer_config,
                        source_metadata_input: req.source_metadata_input,
                    },
                    pc.as_ref(),
                    &known_classes,
                    tcl_compiler::compilation_unit::ModuleAnalysisFacts {
                        trace: tcl_compiler::compilation_unit::ModuleTraceFacts {
                            traced_variables: &traced_variables,
                            has_dynamic_variable_trace: req.has_dynamic_variable_trace,
                            deferred_writes: &req.analysis_context.deferred_writes,
                        },
                        analysis_context: req.analysis_context,
                        command_trust: &command_trust,
                    },
                );
                cache.insert(key, fu.clone());
                fu
            },
        )
    };
    let cu_base = build(base, &mut cache);
    let cu_shifted = build(shifted, &mut cache);
    (cu_base, cu_shifted)
}

#[test]
fn rebase_shifted_unit_spans_match_fresh() {
    // A body exercising the rebase variants the existing shift-test misses:
    //   * foreach (Foreach span + body + raw_tokens)
    //   * try / finally (Try body + handler bodies + finally)
    //   * uplevel (UpFrame body)
    //   * incr / expr (Incr + ExprEval spans)
    //   * a `for` loop populates cfg.loop_nodes (LoopNode span + for_stmt)
    //   * `if {1}` folds an SCCP constant branch (sccp.constant_branches span)
    //   * `subst` records a template-word plan (sccp.template_plans span)
    //   * an opaque `switch` over a constant subject records its selection
    //     (sccp.selections: the statement span and each arm's pattern span);
    //     a literal subject, since the `try` body's unknown `risky` may reach
    //     every local through `upvar`, so no local of `p` stays constant
    let body = "\
proc p {items} {
    set total 0
    subst -nocommands {total $total}
    switch -glob -- abc { a* { set zero 1 } default { set zero 0 } }
    foreach it $items { incr total }
    for {set i 0} {$i < 2} {incr i} { set acc [expr {$i * 2}] }
    if {1} { set always 1 } else { set never 0 }
    try {
        set r [risky]
    } on error {e} {
        set r fallback
    } finally {
        uplevel 1 { set marker done }
    }
    return $total
}
";
    // Prepend several top-level lines so the proc shifts down by a known delta.
    let shifted = format!("set a 0\nset b 1\n# comment\nset c 2\n{body}");
    let (_cu_base, cu_shifted) = rebased_units(body, &shifted);
    // A fresh whole-file build at the shifted position is the ground truth.
    let cu_fresh = fresh_unit(&shifted);

    let got = cu_shifted.fixture_function("::p").expect("rebased proc");
    let want = cu_fresh.fixture_function("::p").expect("fresh proc");
    assert_eq!(
        want.sccp.selections.len(),
        1,
        "the opaque switch records its selection"
    );
    assert_span_carrying_eq(
        got,
        want,
        "shifted foreach/try/finally/uplevel/for/if/switch",
    );
}

#[test]
fn rebase_switch_and_while_shift() {
    // A second shape: switch (arm + default bodies, subject span) and a while
    // loop, shifted, must still rebase to a fresh unit's span-carrying parts.
    let body = "\
proc q {n} {
    while {$n > 0} { incr n -1 }
    switch -- $n {
        0 { set s zero }
        1 { set s one }
        default { set s many }
    }
    return $s
}
";
    let shifted = format!("# pad\n# pad2\nset top 9\n{body}");
    let (_b, cu_shifted) = rebased_units(body, &shifted);
    let cu_fresh = fresh_unit(&shifted);
    assert_span_carrying_eq(
        cu_shifted.fixture_function("::q").expect("rebased"),
        cu_fresh.fixture_function("::q").expect("fresh"),
        "shifted switch/while",
    );
}

#[test]
fn rebase_zero_delta_is_noop() {
    // When the cached body sits at the same offset (no shift), delta == 0 and
    // the rebase early-returns — the memoised unit's span-carrying parts equal a
    // fresh build with no span movement.  (Drives the `delta == 0` early-return
    // arms in both `rebase_function_unit` and `rebase_script`.)
    let body = "proc z {} { set x 1\n set y [expr {$x + 1}]\n return $y }\n";
    let (cu_base, _shifted) = rebased_units(body, body);
    let cu_fresh = fresh_unit(body);
    assert_span_carrying_eq(
        cu_base.fixture_function("::z").expect("memoised"),
        cu_fresh.fixture_function("::z").expect("fresh"),
        "zero-delta",
    );
}
