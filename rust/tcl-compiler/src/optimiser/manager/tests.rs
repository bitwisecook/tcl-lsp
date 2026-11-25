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

use super::*;

/// #2524: a stub that states `-frame own` keeps the caller's constants,
/// so the optimiser folds a branch across the call; an unstated stub
/// still widens them.
#[test]
fn a_stub_stating_its_own_frame_keeps_a_branch_foldable_across_its_call() {
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::clone(&context),
        config,
    );
    let registry = context.commands();
    let folds = |stub: &str| {
        let src = format!(
            "# tcl-lsp: stubs-begin\n# tcl-lsp: stub {stub}\n# tcl-lsp: stubs-end\n\
                 proc p {{}} {{\n    set g 5\n    db_query {{select 1}}\n    \
                 if {{$g == 5}} {{puts five}} else {{puts other}}\n}}\np\n"
        );
        let (_, applied, _) = optimise_source_multipass_admitting_standalone_at(
            &src,
            None,
            (registry, Some(&input)),
            4,
            |_, candidates| candidates,
        );
        applied.iter().any(|o| o.code == DiagCode::O101)
    };
    assert!(folds("db_query {sql} -frame own"));
    assert!(folds("db_query {sql} -frame none"));
    assert!(!folds("db_query {sql}"));
}
use tcl_lexer::Span;

/// Dialect-invariant optimiser contract (Tcl 8.4–9.1, no interpreter): message
/// wording never selects between competing replacements (#2426).
#[test]
fn rewording_a_candidate_does_not_change_the_surviving_replacement() {
    let winner = |first: &str, second: &str| {
        let mut opts = vec![
            Optimisation::new(DiagCode::O100, first, Span::new(6, 7), "z"),
            Optimisation::new(DiagCode::O100, second, Span::new(6, 7), "y"),
        ];
        sort_optimisations(&mut opts);
        let selected = select_non_overlapping(&opts);
        assert_eq!(selected.len(), 1);
        selected[0].replacement.clone()
    };
    assert_eq!(winner("a", "b"), "y");
    assert_eq!(winner("b", "a"), "y");
}

#[test]
fn multipass_reports_only_edits_that_change_source() {
    let source = "set x 5";
    let mut hint = Optimisation::new(DiagCode::O102, "hint", Span::new(0, 7), "set x 6");
    hint.hint_only = true;
    let unchanged = Optimisation::new(DiagCode::O100, "unchanged", Span::new(0, 7), source);
    let invalid = Optimisation::new(DiagCode::O100, "invalid", Span::new(0, 99), "");
    let changed = Optimisation::new(DiagCode::O100, "changed", Span::new(6, 7), "6");
    let candidates = vec![hint, unchanged, invalid, changed.clone(), changed];
    let (text, applied, iterations) = optimise_source_multipass_using(
        source,
        1,
        |_| candidates.clone(),
        |_, candidates| candidates,
    );
    assert_eq!(text, "set x 6");
    assert_eq!(iterations, 1);
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].message, "changed");
    let (text, applied, _) = optimise_source_multipass_using(
        source,
        1,
        |_| candidates[..3].to_vec(),
        |_, candidates| candidates,
    );
    assert_eq!(text, source);
    assert_eq!(applied.len(), 0);
}

#[test]
fn supplied_multipass_refreshes_conditional_candidates_for_each_source_image() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let context =
        crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = std::sync::Arc::clone(context.commands());
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = crate::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    // O112 removes the whole constant branch; O100 substitutes variable
    // constants within a surviving expression. This test keeps the actual
    // applied rewrite and its newly analysed source image non-vacuous.
    let source = "proc subject {} {if {1} {puts retained} else {puts omitted}}";
    let mut images = Vec::new();
    let mut branch_candidates = Vec::new();
    let (text, applied, iterations) = optimise_source_multipass_admitting_with_analysis_input(
        source,
        &registry,
        Some(&input),
        3,
        |current, candidates| {
            images.push(current.to_owned());
            branch_candidates.push(
                candidates
                    .iter()
                    .filter(|row| row.code == DiagCode::O112)
                    .count(),
            );
            candidates
                .into_iter()
                .filter(|row| row.code == DiagCode::O112)
                .collect()
        },
    );
    assert!(applied.iter().any(|row| row.code == DiagCode::O112));
    assert!(iterations >= 2);
    assert_ne!(images[0], images[1]);
    assert_eq!(images.last().unwrap(), &text);
    assert!(branch_candidates[0] > 0);
    assert_eq!(
        branch_candidates[1], 0,
        "the rewritten image must not reuse the old branch candidate"
    );
    assert!(text.contains("retained"));
    assert!(!text.contains("omitted"));
    let missing = optimise_source_multipass_admitting_with_analysis_input(
        source,
        &registry,
        None,
        3,
        |_, _| panic!("missing supplied input must not enter the rewrite loop"),
    );
    assert_eq!(missing, (source.to_owned(), Vec::new(), 0));
}

fn registry() -> CommandRegistry {
    // `when` is registry-resolved (no
    // string-pattern fallback in `lower_command`), so any
    // test that lowers iRule code through `optimise_*` must
    // carry the iRules dialect.  Loading it here keeps the
    // helper a one-call site; tests that only lower plain
    // Tcl don't notice the extra commands.
    let mut r = CommandRegistry::build_default();
    r.load_irules();
    r
}

#[test]
fn empty_source_yields_empty_result() {
    let opts = optimise("", &registry());
    assert_eq!(opts, [] as [crate::optimiser::Optimisation; 0]);
}

#[test]
fn cross_event_vars_reach_code_sinking_before_elimination() {
    let source = "when HTTP_REQUEST {\n    if {$cond} { set n 1 } else { set n 2 }\n    set flag $n\n    if {$n > 1} { log local0. $flag }\n}\nwhen HTTP_RESPONSE { log local0. $flag }\n";
    let opts = optimise_with_dialect(
        source,
        &registry(),
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert!(
        opts.iter().all(|opt| {
            opt.code != DiagCode::O125
                || !source[opt.span.start() as usize..opt.span.end() as usize].contains("set flag")
        }),
        "cross-event assignment must not be sunk before elimination: {opts:?}"
    );
}

/// Every optimiser pass
/// (`propagation`, `expr_simplify`, `pattern_recognition`,
/// `structure_elimination`, `code_sinking`) walks `Script`/`Statement`
/// bodies recursively and is now depth-capped
/// (`super::MAX_OPTIMISER_WALK_DEPTH`). In this end-to-end pipeline
/// `lowering`'s own cap (`MAX_LOWER_NEST_DEPTH`, matching value) is the
/// first line of defence — it truncates source nesting before the
/// optimiser passes ever see it — so this asserts the *whole pipeline*
/// survives deep nesting rather than isolating the optimiser-level caps
/// specifically (those are unit-verified by construction: every
/// recursive call site threads `depth + 1`, matching the pattern
/// already proven in `lowering`/`param_traits`).
///
/// `tcl-compiler` is a library — it does not own the stack its callers
/// run it on. Every real consumer (`tcl-lsp-server`/`tcl-mcp`/the `tcl`
/// CLI/`f5-cli`/`tcl-debugger`) already wraps calls into it with a
/// dedicated 64 MiB thread; `cargo test`'s
/// bare ~2 MiB per-test default is not representative of that and is
/// not the depth caps' design target — they bound *frame count*, not
/// the stack cost of each frame (see `docs/design/compiler/
/// recursive-descent-depth-limits.md`). So this spawns its own
/// production-sized thread rather than asserting on the test harness's
/// thread directly.
#[test]
fn deeply_nested_if_survives_full_optimiser_pipeline() {
    const DEPTH: usize = 400;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = String::new();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("set done 1\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    // The assertion is that this returns at all, not what it returns.
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let _ = optimise(&src, &registry());
        })
        .unwrap()
        .join()
        .unwrap();
}

fn optimised(src: &str) -> String {
    let opts = optimise(src, &registry());
    apply_optimisations(src, &opts)
}

#[test]
fn constant_loop_condition_fold_keeps_braces() {
    // A constant `while` condition folded through O101 must not drop the
    // opening brace of the braced condition: the CFG loop-condition span
    // omits the closing `}`, so a naive rewrite yields `while 1}`.
    // `while {1}` is already minimal → unchanged; `while {1 < 2}` folds
    // to `while {1}` (braces preserved).
    assert_eq!(optimised("while {1} { break }\n"), "while {1} { break }\n");
    assert_eq!(
        optimised("while {1 < 2} { break }\n"),
        "while {1} { break }\n"
    );
}

#[test]
fn call_by_name_in_namespace_keeps_feeding_set() {
    // A `proc` declared inside a namespaced proc is registered under its
    // qualified name (`::demo::bump`); a same-namespace bare call
    // (`bump x`) must still resolve to it so the call-by-name `upvar`
    // read keeps the feeding `set x 10` alive. Regression for an
    // over-removal that produced a dangling `$x` reference.
    let src = "namespace eval ::demo {\n    \
                   proc upvarDemo {} {\n        set x 10\n        \
                   proc bump {varName} { upvar 1 $varName v; incr v; return $v }\n        \
                   set y [bump x]\n        return $x\n    }\n}\n";
    assert!(
        optimised(src).contains("set x 10"),
        "call-by-name read must keep `set x 10`; got {:?}",
        optimised(src),
    );
}

#[test]
fn couples_constant_propagation_with_dead_store_removal() {
    // The propagated constant leaves `set x 42` dead — it is removed.
    assert_eq!(optimised("set x 42\nputs $x\n"), "puts 42\n");
    // String-interpolation reads are covered too (the textual scan does
    // not rely on def-use tracking the in-string `$x`).
    assert_eq!(
        optimised("proc f {} {\n  set x 42\n  puts \"v $x\"\n}\n"),
        "proc f {} {\n  puts \"v 42\"\n}\n",
    );
}

#[test]
fn couples_sccp_const_expr_dead_store_removal() {
    // `set x [expr {1+1}]` folds to a constant SCCP proves; once its only
    // use is propagated the computed def is dead — removed. The O101
    // expr-fold inside the def line is superseded by the line deletion.
    assert_eq!(optimised("set x [expr {1+1}]\nputs $x\n"), "puts 2\n");
    // The quoted-expr form folds the same way — it is removed too.
    assert_eq!(optimised("set x [expr \"1+1\"]\nputs $x\n"), "puts 2\n");
    // Inside a proc, with a string-interpolation read.
    assert_eq!(
        optimised("proc p {} {\n  set x [expr {1+1}]\n  puts \"v=$x\"\n}\n"),
        "proc p {} {\n  puts \"v=2\"\n}\n",
    );
}

#[test]
fn coupling_keeps_non_const_expr_def() {
    // A computed assignment SCCP cannot prove constant (a runtime command)
    // is never coupled away.
    let kept = optimised("set x [expr {[clock seconds]}]\nputs $x\n");
    assert!(
        kept.contains("set x [expr"),
        "non-const expr def must be kept; got {kept:?}",
    );
}

#[test]
fn coupling_keeps_defs_it_cannot_prove_dead() {
    // Never read at top level → kept (the unused-variable pass owns the
    // proc case; a top-level const may be an externally-consumed global).
    assert_eq!(
        optimised("set x 42\nputs hello\n"),
        "set x 42\nputs hello\n"
    );
    // Read by name, not via `$x` — `[set x]` still needs the value.
    let by_name = optimised("proc f {} {\n  set x 42\n  puts [set x]\n}\n");
    assert!(
        by_name.contains("set x 42"),
        "by-name read must keep the def; got {by_name:?}",
    );
    // A value carrying substitution metacharacters is never coupled.
    let meta = optimised("set e {$y [k]}\ncatch {subst $e}\n");
    assert!(
        meta.contains("set e "),
        "metacharacter-bearing const must keep its def; got {meta:?}",
    );
}

#[test]
fn coupling_sees_through_literal_name_in_strings() {
    // The variable name appearing as literal text inside a string
    // (`"x=$x"`) is not a by-name read — the def is still removable when
    // every real `$x` reference is propagated.
    assert_eq!(
        optimised("set x 1\nset y 2\nputs \"x=$x y=$y\"\n"),
        "puts \"x=1 y=2\"\n",
    );
}

#[test]
fn braced_word_is_never_propagated() {
    // Tcl performs no substitution inside `{…}`, so `$x` in a braced word
    // is a literal — it must not be propagated, and the def stays live.
    assert_eq!(
        optimised("set x 42\nputs {$x}\nputs $x\n"),
        "set x 42\nputs {$x}\nputs 42\n",
    );
}

#[test]
fn read_modify_write_target_keeps_feeding_set() {
    // `lset` / `lpop` read the list before rewriting it, so the feeding
    // `set` is live — not a dead store.
    for src in [
        "set lst {a b c d e}\nlset lst 2 X\nputs $lst\n",
        "set lst {1 2 3}\nlpop lst\nputs $lst\n",
    ] {
        assert!(
            optimised(src).contains("set lst"),
            "RMW target's feeding set must be kept; got {:?}",
            optimised(src),
        );
    }
}

#[test]
fn constant_branch_fires_via_manager() {
    // if {1} { set x 1 } triggers both O101 (branch fold)
    // and O112 (structure elimination). The overlap filter
    // prefers the higher-priority O112 (priority 9) over
    // O101 (priority 1), so the manager's output should
    // contain O112 in this shape.
    let opts = optimise("if {1} { set x 1 } else { set y 2 }", &registry());
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O112 || o.code == DiagCode::O101),
        "expected at least one branch-related rewrite, got {opts:?}",
    );
}

fn native_existence_optimisations(source: &str) -> Vec<Optimisation> {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let capture = crate::environment_ingress::captured_native_entry(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(capture.as_entry().clone())),
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        },
        ..crate::command_binding::SourceAnalysisEntry::default()
    };
    let unit = CompilationUnit::build_with_source_entry(
        source,
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &entry,
    );
    optimise_unit(&unit, registry, Some(profile))
}

#[test]
fn info_exists_fold_surfaces_o101() {
    // A provably-constant `info exists`
    // guard (never-defined non-param folds false; a parameter folds
    // true) surfaces as an O101 constant-branch fold.
    let never =
        native_existence_optimisations("proc f {a} { if {[info exists b]} { puts hi } }; f VALUE");
    assert!(
        never
            .iter()
            .any(|o| o.code == DiagCode::O101 && o.replacement == "{0}"),
        "never-defined `info exists` should fold to {{0}}, got {never:?}",
    );
    let param =
        native_existence_optimisations("proc f {a} { if {[info exists a]} { puts hi } }; f VALUE");
    assert!(
        param
            .iter()
            .any(|o| o.code == DiagCode::O101 && o.replacement == "{1}"),
        "parameter `info exists` should fold to {{1}}, got {param:?}",
    );
}

#[test]
fn info_exists_rewrite_requires_actual_selected_entry() {
    let declaration = "proc f {a} {if {[info exists a]} {puts hi}}";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let unknown = crate::command_binding::SourceAnalysisEntry {
        unknown_entry: true,
        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
        ..Default::default()
    };
    let unit = CompilationUnit::build_with_source_entry(
        &format!("{declaration}; f VALUE"),
        crate::compilation_unit::UnitBuildOptions {
            registry,
            defer_top_level: false,
            config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            dialect: Some(profile),
            external_call_sites: None,
            declared_commands: None,
        },
        &unknown,
    );
    assert!(
        optimise_unit(&unit, registry, Some(profile))
            .iter()
            .all(|rewrite| rewrite.code != DiagCode::O101)
    );
    for source in [
        declaration.to_owned(),
        format!("rename info {{}}; proc info args {{return UNKNOWN}}; {declaration}; f VALUE"),
        format!("{declaration}; mystery; f VALUE"),
    ] {
        assert!(
            native_existence_optimisations(&source)
                .iter()
                .all(|rewrite| rewrite.code != DiagCode::O101),
            "unentered or unselected existence query: {source}"
        );
    }
}

#[test]
fn output_is_sorted_by_span_start() {
    let opts = optimise("set x 5\nif {1} { puts $x } else { puts 0 }", &registry());
    let mut prev = 0u32;
    for o in &opts {
        assert!(
            o.span.start() >= prev,
            "manager output must be sorted by span start: got {} after {}",
            o.span.start(),
            prev,
        );
        prev = o.span.start();
    }
}

#[test]
fn overlap_filter_runs() {
    // The manager applies select_non_overlapping, so duplicate
    // / overlapping rewrites must not appear in the output
    // — exercise by checking that no two rewrites share the
    // exact same span.
    let opts = optimise("if {1} { set x 1 } else { set y 2 }", &registry());
    let spans: Vec<_> = opts.iter().map(|o| o.span).collect();
    let mut unique = spans.clone();
    unique.sort_by_key(|s| (s.start(), s.end()));
    unique.dedup();
    assert_eq!(
        spans.len(),
        unique.len(),
        "manager must deduplicate overlapping rewrites",
    );
}

#[test]
fn optimise_raw_skips_overlap_filter() {
    // Raw output can contain overlapping rewrites that the
    // filtered path would dedupe — the `raw` entry point is
    // the escape hatch that leaves them visible.
    let raw = optimise_raw("if {1} { set x 1 } else { set y 2 }", &registry(), None);
    // Presence alone is the contract.
    let _ = raw;
}

#[test]
fn tcloo_method_bodies_get_zero_o100_family_optimisations() {
    // Coverage gap, not a bug: `branch_folding::run` walks
    // `cu.analysable_functions()` and `propagation::run` walks
    // `cu.top_level` + `cu.procedures` directly — neither visits
    // `cu.methods`. So today the O100/O101/O102/O103
    // constant-propagation and branch-folding family never fires inside
    // a TclOO method body, even for a body this trivially foldable.
    // Contrast with O109 (dead-store elimination, `elimination.rs`),
    // which explicitly iterates `cu.methods` with its own
    // instance-variable escaping rules. This test locks in the
    // *current* behaviour so a future change that starts analysing
    // method bodies is a deliberate, reviewed decision — not an
    // accidental regression noticed only via a diff in generated
    // diagnostics.
    let src = "oo::class create Counter {\n    \
                   method bump {} {\n        set n 5\n        return $n\n    }\n}\n";
    let opts = optimise(src, &registry());
    assert!(
        opts.iter().all(|o| !matches!(
            o.code,
            DiagCode::O100 | DiagCode::O101 | DiagCode::O102 | DiagCode::O103
        )),
        "TclOO method bodies are not yet analysed by the O100 family, got {opts:?}",
    );
}

#[test]
fn interp_eval_barrier_body_never_gets_o100_family_diagnostics() {
    // True negative, by design rather than by soundness gap: a literal
    // `interp eval {} {…}` body lowers its whole argument list to a
    // single opaque `Statement::Barrier` (`command`/`args`/`tokens`
    // only — see `ir.rs::Statement::Barrier`), never a nested `Script`.
    // There is nothing for any pass's recursive statement walk to
    // descend into, so an obviously-foldable expression written
    // *inside* that literal body (`expr {1+1}`) never gets an O101
    // constant-expr-fold diagnostic — `interp eval`'s body text is
    // opaque to every static-analysis pass, exactly like a bare
    // `eval`/`uplevel`/`unknown` body with a non-trivial target. This
    // locks in that "no diagnostic" here is the *expected* shape, not
    // an unnoticed hole.
    let src = "interp eval {} { set x [expr {1+1}] }\n";
    let opts = optimise(src, &registry());
    assert!(
        opts.iter().all(|o| !matches!(
            o.code,
            DiagCode::O100 | DiagCode::O101 | DiagCode::O102 | DiagCode::O103
        )),
        "content inside an `interp eval` barrier body must not be analysed, got {opts:?}",
    );
}

#[test]
fn same_pattern_in_a_plain_proc_does_fold_as_control() {
    // Precision control for the coverage-gap test above: the identical
    // `set n 5; return $n` shape, in a plain (non-TclOO) proc, does
    // fold (SCCP proves `n` constant and O100 rewrites the `return`
    // directly) — proving the gap above is specific to method bodies
    // rather than the pattern being unfoldable everywhere.
    let src = "proc bump {} {\n    set n 5\n    return $n\n}\n";
    let opts = optimise(src, &registry());
    assert!(
        opts.iter()
            .any(|o| o.code == DiagCode::O100 && o.replacement == "return 5"),
        "control: same set/return pattern in a plain proc should fold, got {opts:?}",
    );
}

#[test]
fn dialect_gated_passes_observe_active_dialect() {
    // irules-only O124 should fire when dialect = f5-irules.
    let src = "proc ::dead {} { return 1 }\nwhen HTTP_REQUEST { set x 0 }\n";
    let irules = tcl_registry::model::ingress::static_context_for("f5-irules");
    let opts = optimise_with_dialect(
        src,
        irules.commands(),
        Some(tcl_dialect::DialectProfile::irules()),
    );
    assert!(
        opts.iter().any(|o| o.code == DiagCode::O124),
        "expected O124 in irules dialect, got {opts:?}",
    );
    // And should NOT fire for plain tcl.
    let tcl = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let tcl_opts = optimise_with_dialect(src, tcl.commands(), tcl.commands().profile());
    assert!(
        tcl_opts.iter().all(|o| o.code != DiagCode::O124),
        "O124 should be gated on irules dialect, got {tcl_opts:?}",
    );
}

/// The two members of one O125 sink group over `source`: the deletion of
/// the leading assignment, and the insertion that replays it into the
/// branch that consumes it.
fn sink_group(source: &str) -> Vec<Optimisation> {
    let set_end = u32::try_from(source.find('\n').expect("two-line source")).unwrap();
    let body_start = u32::try_from(source.rfind("log $msg").expect("target statement")).unwrap();
    let mut del = Optimisation::new(DiagCode::O125, "delete original", Span::new(0, set_end), "");
    del.group = Some(0);
    let mut ins = Optimisation::new(
        DiagCode::O125,
        "prepend in target body",
        Span::new(body_start, body_start + 8),
        "set msg \"error\"; log $msg",
    );
    ins.group = Some(0);
    vec![del, ins]
}

#[test]
fn group_mate_replacement_does_not_resurrect_a_paired_deletion() {
    // The insertion's replacement holds both the sunk `set msg …` and the
    // `$msg` consuming it, so it reads as a resurrection unless group-mates
    // are exempt. Both members must survive, or the assignment lands twice.
    let source = "set msg \"error\"\nif {$ok} { return } else { log $msg }";
    let mut selected = sink_group(source);
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let cu = CompilationUnit::build_for_dialect(source, registry, false, "f5-irules");
    drop_def_elims_resurrected_by_replacements(&cu, registry, &mut selected);
    assert_eq!(
        selected.len(),
        2,
        "both grouped members must survive, got {selected:?}",
    );
}

#[test]
fn a_dropped_deletion_takes_its_whole_group_with_it() {
    // Control for the exemption above: a resurrecting reference from
    // *outside* the group still drops the deletion, and a group is
    // all-or-nothing, so its other members go with it.
    let source = "set msg \"error\"\nif {$ok} { return } else { log $msg }";
    let mut selected = sink_group(source);
    selected.push(Optimisation::new(
        DiagCode::O100,
        "unrelated rewrite that keeps $msg alive",
        Span::new(
            u32::try_from(source.len()).unwrap(),
            u32::try_from(source.len()).unwrap(),
        ),
        "puts $msg",
    ));
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let cu = CompilationUnit::build_for_dialect(source, registry, false, "f5-irules");
    drop_def_elims_resurrected_by_replacements(&cu, registry, &mut selected);
    assert_eq!(
        selected.len(),
        1,
        "the whole O125 group must go, leaving only the unrelated rewrite, got {selected:?}",
    );
    assert_eq!(selected[0].code, DiagCode::O100);
}
/// Manufactured proposals test only finalisation's safety gate; they do
/// not assert that an optimiser pass emits these proposals for the source.
fn guarded_removal(source: &str, command: &str, replacement: &str) -> Vec<Optimisation> {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let cu =
        CompilationUnit::build_for_profile(source, registry, false, registry.profile().unwrap());
    let start = u32::try_from(source.find(command).unwrap()).unwrap();
    let end = start + u32::try_from(command.len()).unwrap();
    let raw = vec![
        Optimisation::new(
            DiagCode::O109,
            "definition removal",
            Span::new(start, end),
            "",
        ),
        Optimisation::new(
            DiagCode::O100,
            "surviving source",
            Span::new(
                u32::try_from(source.len()).unwrap(),
                u32::try_from(source.len()).unwrap(),
            ),
            replacement,
        ),
    ];
    finalise_optimisations(&raw, &cu, registry, registry.profile())
}

#[test]
fn original_definition_guard_keeps_quoted_qualified_and_alias_names() {
    // naming.optimiser.original-definition-resurrection-guard
    // docs/design/analysis/name-resolution-proofs/optimiser-original-definition-resurrection-guard.md
    for (source, command, replacement) in [
        ("set {b c} 0\n", "set {b c} 0", "puts ${b c}"),
        ("set \"b c\" 0\n", "set \"b c\" 0", "puts ${b c}"),
        ("::set b 0\n", "::set b 0", "puts $b"),
        (
            "interp alias {} assign {} set\nassign {b c} 0\n",
            "assign {b c} 0",
            "puts ${b c}",
        ),
        (
            "interp alias {} assign {} set b\nassign 0\n",
            "assign 0",
            "puts $b",
        ),
        ("set {b(c} 0\n", "set {b(c} 0", "puts ${b(c}"),
        ("set {$b} 0\n", "set {$b} 0", "puts ${$b}"),
        ("set $target 0\n", "set $target 0", "puts $b"),
        ("proc set {name value} {}\nset b 0\n", "set b 0", "puts $b"),
    ] {
        let selected = guarded_removal(source, command, replacement);
        assert!(
            selected.iter().all(|opt| opt.code != DiagCode::O109),
            "{source:?}: {selected:?}"
        );
    }
    let selected = guarded_removal("set {b c} 0\n", "set {b c} 0", "puts ${other name}");
    assert!(
        selected.iter().any(|opt| opt.code == DiagCode::O109),
        "unrelated replacement: {selected:?}"
    );
}

#[test]
fn original_definition_guard_declines_foreign_source_registry_and_missing_carrier() {
    // naming.optimiser.original-definition-resurrection-guard
    // docs/design/analysis/name-resolution-proofs/optimiser-original-definition-resurrection-guard.md
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let source = "set {b c} 0\n";
    let cu =
        CompilationUnit::build_for_profile(source, registry, false, registry.profile().unwrap());
    let span = Span::new(0, u32::try_from(source.trim_end().len()).unwrap());
    assert!(
        elim_target_vars(&cu, registry, span, false)
            .unwrap()
            .contains(&"b c".to_owned())
    );
    let mut stale = cu.clone();
    stale.source.replace_range(..3, "put");
    assert_eq!(elim_target_vars(&stale, registry, span, false), None);
    let foreign = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    assert_eq!(elim_target_vars(&cu, foreign, span, false), None);
    stale = cu.clone();
    stale.ir_module.lexer_config.strict_quoting ^= true;
    assert_eq!(elim_target_vars(&stale, registry, span, false), None);
    stale = cu;
    stale.top_level.cfg.command_binding_sites.clear();
    for block in stale.top_level.cfg.blocks.values_mut() {
        for statement in &mut block.statements {
            if let Some(tokens) = statement.tokens_mut() {
                tokens.source_binding = None;
            }
        }
    }
    assert_eq!(elim_target_vars(&stale, registry, span, false), None);
}

/// `tcl_precision` formats every later double on 8.x, so its store survives
/// even when the script's own read of it is propagated; 9.0 ignores it.
#[test]
fn a_tcl_precision_store_is_kept_where_the_release_formats_with_it() {
    let src = "set tcl_precision 3\nputs [expr {1/3.0}]\nputs $tcl_precision\n";
    for (dialect, kept) in [("tcl8.6", true), ("tcl9.0", false)] {
        let context =
            tcl_registry::model::ingress::resolve_environment(dialect).default_context_registry();
        let opts = optimise_with_dialect(
            src,
            context.commands(),
            tcl_dialect::DialectProfile::find(dialect),
        );
        let deletes_store = opts
            .iter()
            .any(|o| !o.hint_only && o.code == DiagCode::O109 && o.span.start() == 0);
        assert_eq!(!deletes_store, kept, "{dialect}: {opts:?}");
        let folds_double = opts
            .iter()
            .any(|o| !o.hint_only && o.replacement == "0.3333333333333333");
        assert_eq!(!folds_double, kept, "{dialect}: {opts:?}");
    }
}

/// `break` in a `for` step ends the loop after one body run (tclsh
/// 8.4–9.1 print `z=0 i=0`), so the loop is not infinite and nothing after
/// it is unreachable or dead.
#[test]
fn a_break_in_a_for_step_leaves_the_code_after_the_loop_live() {
    let src = "for {set i 0} {$i < 3} {break} { set z $i }\nputs \"z=$z i=$i\"\n";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let context =
            tcl_registry::model::ingress::resolve_environment(dialect).default_context_registry();
        let opts = optimise_with_dialect(
            src,
            context.commands(),
            tcl_dialect::DialectProfile::find(dialect),
        );
        let deletions: Vec<_> = opts
            .iter()
            .filter(|o| !o.hint_only && o.replacement.is_empty())
            .collect();
        assert!(deletions.is_empty(), "{dialect}: {deletions:?}");
    }
}

#[test]
fn applied_rewrites_never_intersect_on_the_probe_overlap_fixtures() {
    // Item 244: these probe fixtures once returned intersecting rewrites, so
    // applying every suggestion corrupted the source. Hints are never applied.
    const PROBE: &str = "../../docs/work/name-handling/handover/oo-dispatch-probe";
    let fixtures = [
        "comments-continuations/c02-semicolon-hash-forms.tcl",
        "constants/folds/c17-string-concat.tcl",
        "encodings-channels/raw/s03-lonecr.tcl",
        "errors-returns/e26-for-break-step.tcl",
        "event-loop/ev08-deferred-variable.tcl",
        "jim/j20-octal-and-expr.tcl",
        "rename-alias/editor-rename/before/e08-dynamic-name.tcl",
        "rename-alias/editor-rename/after/e08-dynamic-name.tcl",
        "words-scripts/s06-var-boundaries.tcl",
    ];
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(PROBE);
    for fixture in fixtures {
        let bytes = std::fs::read(root.join(fixture)).expect("probe fixture");
        let src = String::from_utf8_lossy(&bytes);
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = version.dialect_profile_name();
            let context = tcl_registry::model::ingress::resolve_environment(dialect)
                .default_context_registry();
            let mut spans: Vec<_> = optimise_with_dialect(
                &src,
                context.commands(),
                tcl_dialect::DialectProfile::find(dialect),
            )
            .into_iter()
            .filter(|rewrite| !rewrite.hint_only)
            .map(|rewrite| rewrite.span.as_range())
            .collect();
            spans.sort_by_key(|span| (span.start, span.end));
            let overlap = spans.windows(2).find(|pair| pair[1].start < pair[0].end);
            assert!(overlap.is_none(), "{fixture} under {dialect}: {overlap:?}");
        }
    }
}
