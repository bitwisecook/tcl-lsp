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

//! Residual-coverage suite for the two largest remaining gaps in the
//! `tcl-compiler` analysis surface: `src/inlining/mod.rs` and
//! `src/interprocedural.rs`.
//!
//! The existing sibling suites —
//!   * `inlining.rs`         (decision + call-site transform shapes),
//!   * `inlining_rename.rs`  (the α-renamer descent),
//!   * `compiler_analysis_residual.rs` (a first slice of
//!     `classify_proc` / `inline_module` / `count_statements` declines),
//!     — cover the *common* inlining paths.  This suite deliberately targets
//!     the **remaining** uncovered branches, enumerated by walking every match
//!     arm / if-branch / early-return the sibling suites do not reach:
//!
//!   inlining/mod.rs
//!     * `count_one` over `Block` / `UpFrame` / `For` / `While` / `If`-with-else
//!       (the kinds the sibling suites' count tests skip — only Try / Switch / Foreach /
//!       Catch were exercised).
//!     * `rewrite_stmt` recursion that mutates a `Catch` / `While` / `Foreach` /
//!       `For` / `Switch` / `UpFrame` / `Try` body (changed == true arms).
//!     * the verbatim-splice allow-list (`command_is_splice_safe` for
//!       `list` / `string` / `lindex` …), the command-substitution decline
//!       (`arg_has_command_subst`), and the namespace-invariance gate.
//!     * `build_with_defaults` on the variadic-with-defaults path, the
//!       `params_raw.is_empty()` bail, and a `None` default decline.
//!     * `parse_params_with_defaults` quoted / braced / empty-brace defaults.
//!     * `list_clean_for_splice` decline shapes for variadic extras.
//!     * `substitute_irreturn_stmt` over a `Switch` arm (nested return → break).
//!     * the `inline_module` empty-map early return.
//!
//!   interprocedural.rs
//!     * the **call-by-name** machinery —
//!       `build_proc_index_from_summaries` + `collect_call_by_name_reads`
//!       (+ `add_call_by_name` / `scan_value_cmd_subst`) — which no other
//!       test drives directly (it is only reached indirectly through the
//!       optimiser / analyser today).
//!     * `direct_calls` population, transitive `writes_global` /
//!       `has_unknown_calls` (the `transitive_flag` closure), `has_barrier`
//!       via `Statement::Barrier` and `UpFrame`.
//!     * `classify_return` / `summarise_returns` over float / bool / quoted /
//!       braced / `${param}`-passthrough / `UsesParam` shapes.
//!     * call-graph edges discovered inside assigned values / `return` /
//!       `incr` substitutions (`scan_value_substitutions`).
//!     * the wire-form lowerings `ProcArgTrait::as_str` /
//!       `ConstantReturn::as_kind_text`.
//!     * method-summary effect propagation + return summarisation.
//!
//! ## Proof discipline (C-Tcl)
//!
//! The inliner and the interprocedural summaries are pre-codegen IR transforms /
//! compiler-internal analyses.  Their *structure* — which `InlineDecision` a
//! proc gets, which IR node a splice emits, what a `ProcSummary` field holds,
//! which names a call-by-name scan returns — is **not** directly observable
//! from a Tcl program's output, so it is asserted structurally.
//!
//! The **load-bearing** property is that every inlining transform is
//! *semantics-preserving*: where a snippet computes an observable value, the
//! expected value is pinned to real `tclsh8.6` and `tclsh9.0` via
//! `scripts/dev/tclsh_check.sh` and cited with a `// tclsh:` comment (8.4 / 8.5
//! are not installed in this environment), and the test asserts the transformed
//! program still routes that value.  Declining to inline / summarise is always
//! semantics-safe (the runtime call stays), so the many decline pins below need
//! no execution proof beyond the tclsh fact that the un-inlined program's value
//! is unchanged.

use std::collections::HashSet;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::inlining::{count_statements, inline_module};
use tcl_compiler::interprocedural::{
    Arity, ConstantReturn, InterproceduralAnalysis, ObjectTypeMap, ProcArgTrait, ProcIndex,
    build_interprocedural_analysis, build_proc_index_from_summaries, collect_call_by_name_reads,
};
use tcl_compiler::ir::{Module, Statement};
use tcl_compiler::side_effects::EffectRegion;
use tcl_registry::CommandRegistry;

// ===========================================================================
// Shared harness
// ===========================================================================

fn reg() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// `source → Module`.
fn module_for(source: &str) -> Module {
    CompilationUnit::build_for(source, &reg(), false).ir_module
}

/// Run the whole-module inline transform over a fresh module.
fn inlined(source: &str) -> Module {
    inline_module(module_for(source), &reg())
}

/// Build the interprocedural analysis for `source` (no dialect).
fn interproc(source: &str) -> InterproceduralAnalysis {
    let r = reg();
    let cu = CompilationUnit::build_for(source, &r, false);
    build_interprocedural_analysis(
        &cu.ir_module,
        &r,
        None,
        ObjectTypeMap::none(),
        tcl_compiler::realm::CommandBindingRealm::none(),
    )
}

/// Count statement-position calls to `command` in a flat statement list.
fn calls_to(stmts: &[Statement], command: &str) -> usize {
    stmts
        .iter()
        .filter(|s| matches!(s, Statement::Call { command: c, .. } if c == command))
        .count()
}

/// Count statement-position calls to `command` across the module top level.
fn top_calls_to(module: &Module, command: &str) -> usize {
    calls_to(&module.top_level.statements, command)
}

/// Recursively collect every assignment-target name reachable from a
/// statement list, descending into nested bodies — used to find the mangled
/// `__inline_<cid>__<name>` slots a v3 splice emits.
fn collect_assign_names(stmts: &[Statement], out: &mut Vec<String>) {
    for s in stmts {
        match s {
            Statement::AssignConst { name, .. }
            | Statement::AssignValue { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::Incr { name, .. } => out.push(name.clone()),
            Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
                collect_assign_names(&body.statements, out);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    collect_assign_names(&c.body.statements, out);
                }
                if let Some(b) = else_body {
                    collect_assign_names(&b.statements, out);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                collect_assign_names(&init.statements, out);
                collect_assign_names(&next.statements, out);
                collect_assign_names(&body.statements, out);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. } => collect_assign_names(&body.statements, out),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_assign_names(&body.statements, out);
                for h in handlers {
                    collect_assign_names(&h.body.statements, out);
                }
                if let Some(b) = finally_body {
                    collect_assign_names(&b.statements, out);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        collect_assign_names(&b.statements, out);
                    }
                }
                if let Some(b) = default_body {
                    collect_assign_names(&b.statements, out);
                }
            }
            _ => {}
        }
    }
}

/// Every assignment-target name reachable from a statement list.
fn all_assign_names(stmts: &[Statement]) -> Vec<String> {
    let mut v = Vec::new();
    collect_assign_names(stmts, &mut v);
    v
}

/// Find the single `(name, value)` of a mangled binding ending in `__<suffix>`.
fn binding_value(stmts: &[Statement], suffix: &str) -> Option<String> {
    stmts.iter().find_map(|s| match s {
        Statement::AssignConst { name, value, .. } | Statement::AssignValue { name, value, .. }
            if name.starts_with("__inline_") && name.ends_with(suffix) =>
        {
            Some(value.clone())
        }
        _ => None,
    })
}

// ###########################################################################
// PART A — inlining/mod.rs residual branches
// ###########################################################################

// ---------------------------------------------------------------------------
// A1. count_statements — the per-statement-kind arms the sibling suites skip.
//
// The existing count tests cover empty / flat / nested-if (`inlining.rs`) and
// Try / Switch / Foreach / Catch (`compiler_analysis_residual.rs`).  These pin
// the still-uncovered arms: Block, UpFrame, For (init+1+next+body), While
// (1+body), and If with an else_body.  Counts are compiler-internal, asserted
// structurally; the exact numbers come from the documented `count_one` formula.
// ---------------------------------------------------------------------------

#[test]
fn count_statements_block_recurses() {
    // `eval {literal}` inside a proc body lowers to a `Block` whose two inner
    // statements are counted transitively (the `Block` node is transparent to
    // the count).  (`namespace eval` lowers to an opaque Barrier — see
    // inlining.rs — so `eval` is used to get a real recursable Block.)
    let m = module_for("proc f {} {\n eval { set a 1\nset b 2 }\n}\n");
    assert_eq!(
        count_statements(&m.procedures["::f"].body),
        2,
        "Block counts its inner statements transitively"
    );
}

#[test]
fn count_statements_upframe_recurses() {
    // `uplevel 1 { set a 1 }` is an `UpFrame`; its single inner statement is
    // counted (the UpFrame node itself is transparent to the count).
    let m = module_for("proc f {} {\n uplevel 1 { set a 1 }\n}\n");
    assert_eq!(
        count_statements(&m.procedures["::f"].body),
        1,
        "UpFrame counts its body, not itself"
    );
}

#[test]
fn count_statements_for_counts_all_subscripts() {
    // For = count(init) + 1 + count(next) + count(body).
    // init `set i 0` (1) + 1 + next `incr i` (1) + body `set a 1` (1) = 4.
    let m = module_for("proc f {} {\n for {set i 0} {$i < 2} {incr i} { set a 1 }\n}\n");
    assert_eq!(
        count_statements(&m.procedures["::f"].body),
        4,
        "For counts init + 1 + next + body"
    );
}

#[test]
fn count_statements_while_counts_construct_plus_body() {
    // While = 1 + count(body). 1 + `set a 1` (1) = 2.
    let m = module_for("proc f {} {\n while {1} { set a 1 }\n}\n");
    assert_eq!(
        count_statements(&m.procedures["::f"].body),
        2,
        "While counts 1 (itself) + its body"
    );
}

#[test]
fn count_statements_if_with_else_counts_both_arms() {
    // If = sum over clauses (1 + body) + else body.
    // clause: 1 + `set a 1` (1) = 2; else: `set b 2` (1) = 1; total 3.
    let m = module_for("proc f {} {\n if {1} { set a 1 } else { set b 2 }\n}\n");
    assert_eq!(
        count_statements(&m.procedures["::f"].body),
        3,
        "If counts each clause (1 + body) plus the else body"
    );
}

#[test]
fn count_statements_bare_call_is_one() {
    // The `_ => 1` fallback arm: a single leaf `Call` counts as 1.
    let m = module_for("proc f {} { puts hi }\n");
    assert_eq!(count_statements(&m.procedures["::f"].body), 1);
}

// ---------------------------------------------------------------------------
// A2. rewrite_stmt — recursion into a nested control-flow body that produces a
// change (the `changed == true` arms).  The sibling suites cover If / For bodies; these
// pin Catch / While / Foreach / Switch / UpFrame / Try recursion by inlining an
// empty-body `noop` *inside* each construct and asserting the inner call
// vanishes while the construct survives.
//
// tclsh (8.6, 9.0): `proc noop {} {}` is a no-op; placing `noop` inside any
// control construct and inlining it away cannot change observable behaviour —
// the construct still runs the same (now empty) body.
// ---------------------------------------------------------------------------

#[test]
fn rewrite_recurses_into_catch_body() {
    let out = inlined("proc noop {} {}\ncatch { noop } r\n");
    let catch = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Catch { .. }))
        .expect("the catch survives");
    let Statement::Catch { body, .. } = catch else {
        unreachable!()
    };
    assert_eq!(
        calls_to(&body.statements, "noop"),
        0,
        "the noop inside catch is inlined away"
    );
}

#[test]
fn rewrite_recurses_into_while_body() {
    let out = inlined("proc noop {} {}\nwhile {0} { noop }\n");
    let w = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::While { .. }))
        .expect("the while survives");
    let Statement::While { body, .. } = w else {
        unreachable!()
    };
    assert_eq!(calls_to(&body.statements, "noop"), 0);
}

#[test]
fn rewrite_recurses_into_foreach_body() {
    let out = inlined("proc noop {} {}\nforeach x {1 2} { noop }\n");
    let fe = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Foreach { .. }))
        .expect("the foreach survives");
    let Statement::Foreach { body, .. } = fe else {
        unreachable!()
    };
    assert_eq!(calls_to(&body.statements, "noop"), 0);
}

#[test]
fn rewrite_recurses_into_switch_arm_and_default() {
    // Switch arm + default bodies both recurse (the `arms` loop and the
    // `default_body` match).
    let out = inlined("proc noop {} {}\nswitch x { a { noop } default { noop } }\n");
    let sw = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Switch { .. }))
        .expect("the switch survives");
    let Statement::Switch {
        arms, default_body, ..
    } = sw
    else {
        unreachable!()
    };
    assert_eq!(
        calls_to(
            arms[0]
                .body
                .as_ref()
                .expect("arm body")
                .statements
                .as_slice(),
            "noop"
        ),
        0,
        "arm body noop inlined"
    );
    assert_eq!(
        calls_to(
            &default_body.as_ref().expect("default body").statements,
            "noop"
        ),
        0,
        "default body noop inlined"
    );
}

#[test]
fn rewrite_recurses_into_upframe_body() {
    // `uplevel 1 { noop }` — the UpFrame body recurses. (The wrapper proc is a
    // plain empty-body noop, which is frame-independent, so inlining it inside
    // an uplevel body is sound.)
    let out = inlined("proc noop {} {}\nuplevel 1 { noop }\n");
    let uf = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::UpFrame { .. }))
        .expect("the uplevel survives as an UpFrame");
    let Statement::UpFrame { body, .. } = uf else {
        unreachable!()
    };
    assert_eq!(calls_to(&body.statements, "noop"), 0);
}

#[test]
fn rewrite_recurses_into_try_body_handler_and_finally() {
    // Try body, the `on error` handler body, and the `finally` body all recurse.
    let out = inlined("proc noop {} {}\ntry { noop } on error {e} { noop } finally { noop }\n");
    let try_node = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Try { .. }))
        .expect("the try survives");
    let Statement::Try {
        body,
        handlers,
        finally_body,
        ..
    } = try_node
    else {
        unreachable!()
    };
    assert_eq!(
        calls_to(&body.statements, "noop"),
        0,
        "try body noop inlined"
    );
    assert_eq!(
        calls_to(&handlers[0].body.statements, "noop"),
        0,
        "handler body noop inlined"
    );
    assert_eq!(
        calls_to(&finally_body.as_ref().expect("finally").statements, "noop"),
        0,
        "finally body noop inlined"
    );
}

#[test]
fn rewrite_recurses_into_for_init_and_next() {
    // The For init and next sub-scripts recurse (distinct from the body, which
    // `inlining.rs` already covers).  An empty-body `noop` in init/next vanishes.
    let out = inlined("proc noop {} {}\nfor {noop} {0} {noop} { set a 1 }\n");
    let f = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::For { .. }))
        .expect("the for survives");
    let Statement::For { init, next, .. } = f else {
        unreachable!()
    };
    assert_eq!(calls_to(&init.statements, "noop"), 0, "init noop inlined");
    assert_eq!(calls_to(&next.statements, "noop"), 0, "next noop inlined");
}

// ---------------------------------------------------------------------------
// A3. verbatim-splice allow-list (command_is_splice_safe) + the decline gates.
// `inlining.rs` covers `puts` / `string` wrappers; these pin the other
// allow-list entries and the two decline gates (command-subst arg, non-safe
// command).
// ---------------------------------------------------------------------------

#[test]
fn verbatim_wrapper_of_list_primitive_inlines() {
    // `list` is on the splice-safe allow-list and is pure-leaf, so a zero-param
    // wrapper of it splices verbatim.
    // tclsh (8.6, 9.0): `proc w {} { list 1 2 3 }; w` → "1 2 3"; the spliced
    // `list 1 2 3` computes the same value.
    let out = inlined("proc w {} { list 1 2 3 }\nw\n");
    assert_eq!(top_calls_to(&out, "w"), 0, "wrapper inlined");
    assert_eq!(top_calls_to(&out, "list"), 1, "the list call is spliced in");
}

#[test]
fn verbatim_wrapper_of_lindex_and_concat_inlines() {
    // Two more allow-list members in one wrapper body.
    // tclsh (8.6, 9.0): `proc w {} { concat a b; lindex {x y z} 1 }; w` → "y"
    // (the wrapper's value is the last command's). Splicing both preserves it.
    let out = inlined("proc w {} { concat a b\nlindex {x y z} 1 }\nw\n");
    assert_eq!(top_calls_to(&out, "w"), 0);
    assert_eq!(top_calls_to(&out, "concat"), 1);
    assert_eq!(top_calls_to(&out, "lindex"), 1);
}

#[test]
fn verbatim_wrapper_with_command_subst_arg_declines() {
    // `arg_has_command_subst` is true for `[string length abc]`, so the body
    // statement is NOT splice-eligible — the wrapper is not verbatim-inlined and
    // the call survives. (Declining is semantics-safe.)
    let out = inlined("proc w {} { puts [string length abc] }\nw\n");
    assert_eq!(
        top_calls_to(&out, "w"),
        1,
        "a command-substitution arg in the wrapper body declines the splice"
    );
}

#[test]
fn wrapper_with_local_write_takes_v3_not_verbatim_path() {
    // A body statement that writes a local (`set g 0` / `incr g`) has a
    // non-empty `defs`, so `stmt_is_splice_eligible` rejects it at the
    // `!defs.is_empty()` early-return — the wrapper is NOT verbatim-spliced.
    // Instead the proc is still pure-leaf, so it inlines via the v3 path, which
    // α-renames the local. This pins the verbatim-vs-v3 boundary: the presence
    // of the mangled `__inline_*__g` slot proves the v3 (renaming) path ran, not
    // the verbatim (raw-clone) path.
    // tclsh (8.6, 9.0): `proc w {} { set g 0; incr g; return $g }; w` → 1; the
    // v3 inline preserves that.
    let out = inlined("proc w {} { set g 0\nincr g }\nw\n");
    assert_eq!(
        top_calls_to(&out, "w"),
        0,
        "v3 inlines the local-write body"
    );
    assert!(
        all_assign_names(&out.top_level.statements)
            .iter()
            .any(|n| n.starts_with("__inline_") && n.ends_with("__g")),
        "the local g is α-renamed → the v3 path ran, not the verbatim splice"
    );
}

// ---------------------------------------------------------------------------
// A4. build_with_defaults — the variadic-with-defaults path, the params_raw
// bail, and the None-default decline.
// ---------------------------------------------------------------------------

#[test]
fn variadic_with_default_fills_default_and_empty_args() {
    // `proc f {a {b 9} args}` called as `f 1`: a=1 (arg), b=9 (default), args=""
    // (empty variadic). Drives `build_with_defaults(..., has_variadic = true)`.
    // tclsh (8.6, 9.0): `proc f {a {b 9} args} { return [list $a $b $args] };
    // f 1` → "1 9 {}"; the inlined bindings reproduce a=1, b=9, args="".
    let out = inlined("proc f {a {b 9} args} { puts $a }\nf 1\n");
    assert_eq!(top_calls_to(&out, "f"), 0, "the call inlined");
    assert_eq!(
        binding_value(&out.top_level.statements, "__a").as_deref(),
        Some("1")
    );
    assert_eq!(
        binding_value(&out.top_level.statements, "__b").as_deref(),
        Some("9"),
        "the missing b is filled from its default"
    );
    assert_eq!(
        binding_value(&out.top_level.statements, "__args").as_deref(),
        Some(""),
        "the variadic args binds to the empty string"
    );
}

#[test]
fn missing_positional_with_a_later_default_but_a_gap_declines() {
    // `proc f {a b {c 3}}` called as `f 1`: `b` has no default and is missing,
    // so `build_with_defaults` hits the `None => return None` arm and declines.
    let out = inlined("proc f {a b {c 3}} { puts $a }\nf 1\n");
    assert_eq!(
        top_calls_to(&out, "f"),
        1,
        "a missing non-default positional declines the inline"
    );
}

// ---------------------------------------------------------------------------
// A5. parse_params_with_defaults — quoted / braced / empty defaults.
// Exercised through the v3 default-fill: the parsed default value must land in
// the binding stripped of its quoting (Tcl proc defaults are literal).
// ---------------------------------------------------------------------------

#[test]
fn default_quoted_string_is_stripped() {
    // `{y "hello world"}` — the quoted default parses to the literal
    // `hello world` (quotes stripped, embedded space kept as one value).
    // tclsh (8.6, 9.0): `proc f {x {y "hello world"}} { return $y }; f 1` →
    // "hello world".
    let out = inlined("proc f {x {y \"hello world\"}} { puts $x }\nf 1\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    assert_eq!(
        binding_value(&out.top_level.statements, "__y").as_deref(),
        Some("hello world"),
        "the quoted default is stripped to its literal text"
    );
}

#[test]
fn default_braced_value_is_stripped() {
    // `{y {a b}}` — the braced default parses to the literal `a b`.
    // tclsh (8.6, 9.0): `proc f {x {y {a b}}} { return $y }; f 1` → "a b".
    let out = inlined("proc f {x {y {a b}}} { puts $x }\nf 1\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    assert_eq!(
        binding_value(&out.top_level.statements, "__y").as_deref(),
        Some("a b"),
        "the braced default is stripped to its literal text"
    );
}

#[test]
fn default_bare_word_value_kept() {
    // `{y 5}` — a bare-word default keeps its text verbatim (no stripping).
    // tclsh (8.6, 9.0): `proc f {x {y 5}} { return $y }; f 1` → 5.
    let out = inlined("proc f {x {y 5}} { puts $x }\nf 1\n");
    assert_eq!(
        binding_value(&out.top_level.statements, "__y").as_deref(),
        Some("5")
    );
}

// ---------------------------------------------------------------------------
// A6. list_clean_for_splice — variadic-extra decline shapes.
// When variadic extras are packed into `[list e1 e2 …]`, an extra word that is
// unsafe to splice verbatim (whitespace, an empty word, an unbalanced `[`/`${`)
// declines the whole inline.  The clean case packs successfully.
// ---------------------------------------------------------------------------

#[test]
fn variadic_clean_extras_pack_into_list() {
    // Plain bareword extras pack into `[list 1 2 3]`.
    // tclsh (8.6, 9.0): `proc f {args} { return $args }; f 1 2 3` → "1 2 3";
    // binding args to `[list 1 2 3]` evaluates to the same list.
    let out = inlined("proc f {args} { puts $args }\nf 1 2 3\n");
    assert_eq!(top_calls_to(&out, "f"), 0);
    assert_eq!(
        binding_value(&out.top_level.statements, "__args").as_deref(),
        Some("[list 1 2 3]")
    );
}

#[test]
fn variadic_empty_extra_word_declines() {
    // An empty quoted extra word (`""`) would collapse inside `[list …]`,
    // dropping the element — so `list_clean_for_splice`/the empty-word guard
    // declines.
    // tclsh (8.6, 9.0): `proc f {args} { return [llength $args] }; f a "" c` →
    // 3 — the empty word is a real (empty) element; the inliner must not lose it,
    // so it keeps the runtime call.
    let out = inlined("proc f {args} { puts $args }\nf a \"\" c\n");
    assert_eq!(
        top_calls_to(&out, "f"),
        1,
        "an empty variadic extra word declines the splice"
    );
}

#[test]
fn variadic_extra_with_unbalanced_bracket_declines() {
    // A braced-literal extra (`{x}`) among the variadic tail is rejected by the
    // `arg_is_braced_literal` guard (a braced word can't be re-spliced into the
    // list synth), so the inline declines.
    let out = inlined("proc f {args} { puts $args }\nf a {b c} d\n");
    assert_eq!(
        top_calls_to(&out, "f"),
        1,
        "a braced-literal variadic extra declines the splice"
    );
}

// ---------------------------------------------------------------------------
// A7. substitute_irreturn over a Switch arm — a `return` nested inside a
// switch arm body is rewritten to `set __RESULT …; break` when the inline is
// wrapped (non-terminal early-return).  `inlining.rs` covers the `if` case;
// this pins the `Switch` arm of `substitute_irreturn_stmt`.
// ---------------------------------------------------------------------------

#[test]
fn early_return_inside_switch_arm_is_wrapped() {
    // A `return` nested in a switch arm, with a trailing fall-through `return`
    // so the body is v3-eligible (an implicit-return capture off a trailing
    // *switch* is unsupported — `capture_implicit_return_value` returns None —
    // so a trailing literal return is needed for the inline to proceed). Inlined
    // at a non-terminal call site, the switch-arm returns become break-routed
    // inside the one-shot while-wrap.
    // tclsh (8.6, 9.0):
    //   proc f {x} { switch $x { a {return 1} default {return 0} }; return 9 }
    //   f a; set after 1  → f returns 1 (the arm's return wins) and the caller's
    // `set after` still runs; the wrap preserves both.
    let out = inlined(
        "proc f {x} { switch $x { a { return 1 } default { return 0 } }\nreturn 9 }\nf a\nset after 1\n",
    );
    assert_eq!(top_calls_to(&out, "f"), 0, "the call inlined");
    let while_node = out
        .top_level
        .statements
        .iter()
        .find(|s| matches!(s, Statement::While { .. }))
        .expect("the non-terminal early-return is wrapped in a one-shot while");
    let Statement::While { body, .. } = while_node else {
        unreachable!()
    };
    // The switch survives inside the wrap, and its arm bodies now end in a
    // synthetic `break` (the return-as-break lowering).
    let sw = body
        .statements
        .iter()
        .find(|s| matches!(s, Statement::Switch { .. }))
        .expect("switch survives inside the wrap");
    let Statement::Switch { arms, .. } = sw else {
        unreachable!()
    };
    let arm_body = arms[0].body.as_ref().expect("arm a body");
    assert!(
        arm_body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "break")),
        "the switch-arm return is rewritten to set __RESULT + break, got {arm_body:?}"
    );
    // The caller's following statement survives the wrap.
    assert!(
        out.top_level.statements.iter().any(|s| matches!(
            s,
            Statement::AssignConst { name, .. } | Statement::AssignValue { name, .. } if name == "after"
        )),
        "the statement after the inlined call survives"
    );
}

// ---------------------------------------------------------------------------
// A8. inline_module early-return when no proc is inlinable (empty map).
// ---------------------------------------------------------------------------

#[test]
fn inline_module_no_inlinable_proc_returns_unchanged() {
    // Every proc here is non-pure-leaf (`upvar`), so the inlinable map is empty
    // and `inline_module` hits its early-return.  The module is structurally
    // identical to its input.
    let m = module_for("proc p {n} { upvar 1 $n v\nset v 1 }\np x\n");
    let out = inline_module(m.clone(), &reg());
    assert_eq!(
        out, m,
        "an empty inlinable map returns the module unchanged"
    );
}

#[test]
fn inline_module_with_no_procs_at_all_is_unchanged() {
    // A module with no procedures: the inlinable map is trivially empty.
    let m = module_for("set x 1\nputs $x\n");
    let out = inline_module(m.clone(), &reg());
    assert_eq!(out, m);
}

// ###########################################################################
// PART B — interprocedural.rs residual branches
// ###########################################################################
//
// All of Part B asserts compiler-internal summary / call-graph structure
// (analysis-internal, not Tcl-observable), so the assertions are structural,
// per the top-of-file discipline. Where a snippet's *value* under tclsh is
// relevant to why a fact holds (e.g. an upvar write-back), it is cited.

// ---------------------------------------------------------------------------
// B1. The call-by-name machinery — build_proc_index_from_summaries +
// collect_call_by_name_reads (+ add_call_by_name + scan_value_cmd_subst).
// No other test drives these directly. The behaviour: a literal variable NAME
// passed to a callee parameter that the callee consumes via `upvar` must be
// collected (so the optimiser does not flag it dead).
// ---------------------------------------------------------------------------

/// Collect the call-by-name reads a caller proc contributes, via the public
/// `ProcIndex` + `collect_call_by_name_reads` seam.
fn call_by_name_reads(source: &str, caller_qname: &str) -> HashSet<String> {
    let r = reg();
    let cu = CompilationUnit::build_for(source, &r, false);
    let ia = build_interprocedural_analysis(
        &cu.ir_module,
        &r,
        None,
        ObjectTypeMap::none(),
        tcl_compiler::realm::CommandBindingRealm::none(),
    );
    let index = build_proc_index_from_summaries(&ia);
    let fu = cu.function(caller_qname).expect("caller proc");
    collect_call_by_name_reads(&fu.cfg, &index)
}

#[test]
fn call_by_name_direct_call_collects_var_name() {
    // `setvar y 42` passes the literal name `y` to `setvar`'s `n` parameter,
    // which `setvar` writes through `upvar` (VarWrite trait). So `y` is a
    // call-by-name read and must be collected.
    // tclsh (8.6, 9.0): `proc setvar {n v} { upvar 1 $n x; set x $v }; set y 0;
    // setvar y 42; puts $y` → 42 — `y` IS used (written via the alias), so
    // flagging it dead would be wrong.
    let reads = call_by_name_reads(
        "proc setvar {n v} { upvar 1 $n x\nset x $v }\nproc caller {} { set y 0\nsetvar y 42 }\n",
        "::caller",
    );
    assert!(
        reads.contains("y"),
        "the literal name `y` passed to an upvar-writing param is collected, got {reads:?}"
    );
}

#[test]
fn call_by_name_value_command_subst_collects() {
    // The whole-value `[cmd …]` shape: `set out [peek data]` where `peek`
    // reads its first param via `upvar` (VarRead). `data` is the call-by-name
    // arg and is collected by `scan_value_cmd_subst`.
    let reads = call_by_name_reads(
        "proc peek {n} { upvar 1 $n x\nreturn $x }\nproc caller {} { set data 7\nset out [peek data] }\n",
        "::caller",
    );
    assert!(
        reads.contains("data"),
        "a name in a whole-value `[peek data]` substitution is collected, got {reads:?}"
    );
}

#[test]
fn call_by_name_resolves_bare_call_to_namespaced_proc() {
    // A bare `bump y` inside a namespaced caller resolves to `::demo::bump`
    // through the leaf / `::`-prefixed key fallback in `add_call_by_name`
    // (`index.get(cmd).or_else(|| index.get("::{cmd}"))…`).  `bump` writes its
    // first param via `upvar`, so `y` is a call-by-name read.
    // tclsh (8.6, 9.0): `namespace eval ::demo { proc bump {n} { upvar 1 $n x;
    // incr x }; proc caller {} { set y 0; bump y; return $y } }; ::demo::caller`
    // → 1 — `y` is mutated through the alias, so it is live.
    let reads = call_by_name_reads(
        "namespace eval ::demo {\n proc bump {n} { upvar 1 $n x\nincr x }\n proc caller {} { set y 0\nbump y }\n}\n",
        "::demo::caller",
    );
    assert!(
        reads.contains("y"),
        "a bare call to a same-namespace upvar proc collects its name arg, got {reads:?}"
    );
}

#[test]
fn call_by_name_extra_args_beyond_params_are_ignored() {
    // `add_call_by_name` stops at the param count (`params.get(i)` → break): a
    // trailing extra arg past the declared params can't land on any param, so it
    // is never collected even if it looks like a name.  Here `setvar` takes
    // `{n v}`; the third arg `extra` has no param slot.
    let reads = call_by_name_reads(
        "proc setvar {n v} { upvar 1 $n x\nset x $v }\nproc caller {} { set y 0\nsetvar y 1 extra }\n",
        "::caller",
    );
    assert!(
        reads.contains("y"),
        "the in-range upvar name y is still collected, got {reads:?}"
    );
    assert!(
        !reads.contains("extra"),
        "an out-of-range extra arg lands on no param and is not collected, got {reads:?}"
    );
}

#[test]
fn call_by_name_substituted_arg_is_not_collected() {
    // `setvar $dynamic 1` passes a *substituted* arg ($dynamic), not a literal
    // name — `add_call_by_name` rejects anything containing `$`/`[`, so nothing
    // is collected (preserving genuine read-before-set FPs elsewhere).
    let reads = call_by_name_reads(
        "proc setvar {n v} { upvar 1 $n x\nset x $v }\nproc caller {} { set dynamic q\nsetvar $dynamic 1 }\n",
        "::caller",
    );
    assert!(
        !reads.contains("dynamic") && !reads.contains("q"),
        "a substituted (non-literal) call-by-name arg is not collected, got {reads:?}"
    );
}

#[test]
fn call_by_name_non_upvar_param_not_collected() {
    // A literal-name arg landing on a *plain value* parameter (no VarRead /
    // VarWrite trait) is NOT a call-by-name read — `add_call_by_name` only
    // collects when the target param carries the upvar trait.
    let reads = call_by_name_reads(
        "proc plain {a b} { return [expr {$a + $b}] }\nproc caller {} { set v 1\nplain v 2 }\n",
        "::caller",
    );
    assert!(
        !reads.contains("v"),
        "a literal arg to a non-upvar value param is not collected, got {reads:?}"
    );
}

#[test]
fn call_by_name_empty_index_short_circuits() {
    // When the proc index is empty, `collect_call_by_name_reads` returns the
    // empty set without scanning (the `index.is_empty()` early return).
    let r = reg();
    let cu = CompilationUnit::build_for("proc f {} { setvar y 1 }\n", &r, false);
    let fu = cu.function("::f").expect("proc f");
    let empty = ProcIndex::new();
    assert!(
        collect_call_by_name_reads(&fu.cfg, &empty).is_empty(),
        "an empty proc index yields no call-by-name reads"
    );
}

#[test]
fn proc_index_registers_bare_qualified_and_stripped_keys() {
    // `build_proc_index_from_summaries` registers each proc under its leaf,
    // qualified, and leading-colon-stripped names so a bare same-namespace call
    // resolves.  A proc `::demo::bump` must be findable by `bump`, `::demo::bump`,
    // and `demo::bump`.
    let ia = interproc("namespace eval ::demo {\n proc bump {n} { upvar 1 $n x\nincr x }\n}\n");
    let index = build_proc_index_from_summaries(&ia);
    assert!(index.contains_key("bump"), "leaf key registered");
    assert!(
        index.contains_key("::demo::bump"),
        "qualified key registered"
    );
    assert!(
        index.contains_key("demo::bump"),
        "leading-colon-stripped key registered"
    );
}

// ---------------------------------------------------------------------------
// B2. direct_calls vs calls (transitive). The sibling suites assert `calls` (transitive
// closure); `direct_calls` is the local, non-transitive set the callgraph verb
// consumes — pin that it carries only the immediate callee, not A→C.
// ---------------------------------------------------------------------------

#[test]
fn direct_calls_excludes_transitive_edges() {
    // a → b → c. `a.calls` is the transitive closure {b, c}; `a.direct_calls` is
    // only {b}.
    let ia = interproc("proc ::a {} { ::b }\nproc ::b {} { ::c }\nproc ::c {} { set x 1 }\n");
    let a = ia.procedures.get("::a").expect("::a summary");
    assert!(
        a.direct_calls.contains(&"::b".to_string()),
        "direct_calls includes the immediate callee b, got {:?}",
        a.direct_calls
    );
    assert!(
        !a.direct_calls.contains(&"::c".to_string()),
        "direct_calls excludes the transitive callee c, got {:?}",
        a.direct_calls
    );
    assert!(
        a.calls.contains(&"::c".to_string()),
        "calls (transitive) DOES include c, got {:?}",
        a.calls
    );
}

// ---------------------------------------------------------------------------
// B3. transitive writes_global / has_unknown_calls — the `transitive_flag`
// closure ORs in each transitive callee's local flag (the doc'd-but-previously
// buggy propagation).
// ---------------------------------------------------------------------------

#[test]
fn writes_global_propagates_transitively() {
    // `a` itself writes nothing global, but it calls `b` which writes `::g`.
    // `a.writes_global` must be true via the transitive OR.
    let ia = interproc("proc ::a {} { ::b }\nproc ::b {} { set ::g 1 }\n");
    let a = ia.procedures.get("::a").expect("::a summary");
    assert!(
        a.writes_global,
        "writes_global propagates from a transitive callee"
    );
    // Control: a leaf that writes nothing global stays false.
    let ia2 = interproc("proc ::pure {} { return 1 }\n");
    assert!(!ia2.procedures.get("::pure").unwrap().writes_global);
}

#[test]
fn has_unknown_calls_propagates_transitively() {
    // `a` calls `b`, and `b` calls an unknown command. `a.has_unknown_calls` is
    // true via the transitive OR.
    let ia = interproc("proc ::a {} { ::b }\nproc ::b {} { nosuchcmd_xyz }\n");
    let a = ia.procedures.get("::a").expect("::a summary");
    assert!(
        a.has_unknown_calls,
        "has_unknown_calls propagates from a transitive callee"
    );
}

// ---------------------------------------------------------------------------
// B4. has_barrier via Statement::Barrier and via UpFrame.
// ---------------------------------------------------------------------------

#[test]
fn upframe_body_marks_barrier_and_impure() {
    // A static-body `uplevel 1 { … }` lowers to an `UpFrame`, which the scanner
    // treats as a conservative barrier (it can touch any caller-scope variable).
    let ia = interproc("proc ::f {} { uplevel 1 { set x 1 } }\n");
    let s = ia.procedures.get("::f").expect("::f summary");
    assert!(s.has_barrier, "a static uplevel body is a barrier");
    assert!(!s.pure, "a barrier proc is impure");
    assert_eq!(
        s.effect_writes,
        EffectRegion::UNKNOWN_STATE,
        "the uplevel conservatively writes UNKNOWN_STATE"
    );
}

#[test]
fn effect_writes_propagate_through_transitive_callee() {
    // `fixpoint_effects` unions each callee's effect set into the caller.
    // `a` calls `b`, and `b` does a dynamic `eval $dyn` (UNKNOWN_STATE write).
    // `a.effect_writes` must include UNKNOWN_STATE via the callee union, even
    // though `a` itself has no local effect.
    let ia = interproc("proc ::a {} { ::b }\nproc ::b {} { eval $dyn }\n");
    let a = ia.procedures.get("::a").expect("::a summary");
    assert!(
        a.effect_writes.contains(EffectRegion::UNKNOWN_STATE),
        "a's effect_writes unions b's UNKNOWN_STATE, got {:?}",
        a.effect_writes
    );
    // Control: a leaf with no effects keeps NONE.
    let ia2 = interproc("proc ::pure {} { return 1 }\n");
    assert_eq!(
        ia2.procedures.get("::pure").unwrap().effect_writes,
        EffectRegion::NONE
    );
}

// ---------------------------------------------------------------------------
// B5. classify_return / summarise_returns over the value shapes the sibling suites skip
// (the sibling suites cover int literal, $param passthrough, fall-through-not-constant,
// fully-covered-if). These pin: float, bool, quoted string, braced literal,
// ${param} brace passthrough, and the UsesParam (depends-on-params) shape.
// ---------------------------------------------------------------------------

#[test]
fn constant_return_float_literal() {
    // tclsh (8.6, 9.0): `proc f {} { return 3.5 }; f` → 3.5.
    let ia = interproc("proc ::f {} { return 3.5 }\n");
    let s = ia.procedures.get("::f").unwrap();
    assert!(s.returns_constant);
    assert_eq!(s.constant_return, Some(ConstantReturn::Float(3.5)));
}

#[test]
fn constant_return_bool_literal() {
    // `return true` classifies as a Bool constant (literal_to_constant_return
    // maps the lower-cased text).
    // tclsh (8.6, 9.0): `proc f {} { return true }; f` → "true".
    let ia = interproc("proc ::f {} { return true }\n");
    let s = ia.procedures.get("::f").unwrap();
    assert!(s.returns_constant);
    assert_eq!(s.constant_return, Some(ConstantReturn::Bool(true)));
}

#[test]
fn constant_return_string_literal() {
    // A bare-word non-numeric, non-bool literal is a Str constant.
    // tclsh (8.6, 9.0): `proc f {} { return hello }; f` → "hello".
    let ia = interproc("proc ::f {} { return hello }\n");
    let s = ia.procedures.get("::f").unwrap();
    assert!(s.returns_constant);
    assert_eq!(s.constant_return, Some(ConstantReturn::Str("hello".into())));
}

#[test]
fn constant_return_quoted_and_braced_multiword() {
    // `return "a b c"` and `return {a b c}` both lower to the value `a b c`,
    // classified as a Str constant.
    // tclsh (8.6, 9.0): both `proc f {} { return "a b c" }; f` and the braced
    // form → "a b c".
    for src in [
        "proc ::f {} { return \"a b c\" }\n",
        "proc ::f {} { return {a b c} }\n",
    ] {
        let ia = interproc(src);
        let s = ia.procedures.get("::f").unwrap();
        assert!(s.returns_constant, "constant for {src}");
        assert_eq!(
            s.constant_return,
            Some(ConstantReturn::Str("a b c".into())),
            "value for {src}"
        );
    }
}

#[test]
fn passthrough_brace_form_param_detected() {
    // `return ${x}` is a passthrough of param `x` (the brace-form strip arm).
    // tclsh (8.6, 9.0): `proc id {x} { return ${x} }; id hello` → "hello".
    let ia = interproc("proc ::id {x} { return ${x} }\n");
    let s = ia.procedures.get("::id").unwrap();
    assert_eq!(s.return_passthrough_param.as_deref(), Some("x"));
    assert!(
        !s.returns_constant,
        "a passthrough is not a constant return"
    );
}

#[test]
fn return_uses_param_records_depends_not_passthrough() {
    // `return [expr {$x + 1}]` USES param x but is not a plain passthrough —
    // it records `return_depends_on_params = [x]` with no passthrough param.
    let ia = interproc("proc ::f {x} { return [expr {$x + 1}] }\n");
    let s = ia.procedures.get("::f").unwrap();
    assert_eq!(s.return_passthrough_param, None, "not a plain passthrough");
    assert_eq!(
        s.return_depends_on_params,
        vec!["x".to_string()],
        "the return depends on x, got {:?}",
        s.return_depends_on_params
    );
}

#[test]
fn mixed_passthrough_params_is_depends_only() {
    // Two returns passing through *different* params → not a single passthrough;
    // the depends set is the union {x, y}.
    let ia = interproc("proc ::f {x y} { if {$x > 0} { return $x } else { return $y } }\n");
    let s = ia.procedures.get("::f").unwrap();
    assert_eq!(s.return_passthrough_param, None);
    assert_eq!(
        s.return_depends_on_params,
        vec!["x".to_string(), "y".to_string()],
        "depends is the union of both passthrough targets, got {:?}",
        s.return_depends_on_params
    );
}

#[test]
fn no_return_proc_is_not_constant() {
    // A proc with no `return` (falls off the end) has an empty return list →
    // `summarise_returns` returns the `(false, None, …)` empty-case tuple.
    let ia = interproc("proc ::f {} { set x 1 }\n");
    let s = ia.procedures.get("::f").unwrap();
    assert!(!s.returns_constant);
    assert_eq!(s.constant_return, None);
    assert!(s.return_depends_on_params.is_empty());
}

// ---------------------------------------------------------------------------
// B6. call-graph edges discovered inside substitutions (scan_value_substitutions
// / scan_source_for_calls) for assigned values, returns, and incr amounts.
// ---------------------------------------------------------------------------

#[test]
fn edge_via_assigned_value_substitution() {
    // `set y [double $x]` — the `[double $x]` substitution is a call site; the
    // edge a→double must be recorded.
    let ia = interproc(
        "proc ::double {n} { return [expr {$n * 2}] }\nproc ::a {x} { set y [double $x] }\n",
    );
    let a = ia.procedures.get("::a").expect("::a summary");
    assert!(
        a.calls.contains(&"::double".to_string()),
        "a call inside an assigned value is a call-graph edge, got {:?}",
        a.calls
    );
}

#[test]
fn edge_via_return_substitution() {
    // `return [add $x $x]` — the substitution in the return value is a call site.
    let ia = interproc(
        "proc ::add {a b} { return [expr {$a + $b}] }\nproc ::a {x} { return [add $x $x] }\n",
    );
    let a = ia.procedures.get("::a").expect("::a summary");
    assert!(
        a.calls.contains(&"::add".to_string()),
        "a call inside a return value is an edge, got {:?}",
        a.calls
    );
}

#[test]
fn edge_via_incr_amount_substitution() {
    // `incr acc [amount]` — the `[amount]` substitution in the incr amount is a
    // call site (the `Statement::Incr { amount }` substitution scan).
    let ia =
        interproc("proc ::amount {} { return 3 }\nproc ::a {} { set acc 0\nincr acc [amount] }\n");
    let a = ia.procedures.get("::a").expect("::a summary");
    assert!(
        a.calls.contains(&"::amount".to_string()),
        "a call inside an incr amount is an edge, got {:?}",
        a.calls
    );
}

// ---------------------------------------------------------------------------
// B7. wire-form lowerings — ProcArgTrait::as_str and ConstantReturn::as_kind_text
// (the stable serialisation surface consumed by the native LSP server).
// ---------------------------------------------------------------------------

#[test]
fn proc_arg_trait_wire_forms() {
    assert_eq!(ProcArgTrait::Passthrough.as_str(), "passthrough");
    assert_eq!(ProcArgTrait::UsedInCondition.as_str(), "used_in_condition");
    assert_eq!(
        ProcArgTrait::ForwardedToCallee.as_str(),
        "forwarded_to_callee"
    );
    assert_eq!(ProcArgTrait::VarRead.as_str(), "var_read");
    assert_eq!(ProcArgTrait::VarWrite.as_str(), "var_write");
    assert_eq!(ProcArgTrait::Unused.as_str(), "unused");
}

#[test]
fn constant_return_kind_text_wire_forms() {
    assert_eq!(
        ConstantReturn::Int(42).as_kind_text(),
        ("int", "42".to_string())
    );
    assert_eq!(
        ConstantReturn::Float(1.5).as_kind_text(),
        ("float", "1.5".to_string())
    );
    // Bool renders as "1" / "0" in the wire form (distinct from the
    // human-facing "true"/"false" rendering noted in the type's doc).
    assert_eq!(
        ConstantReturn::Bool(true).as_kind_text(),
        ("bool", "1".to_string())
    );
    assert_eq!(
        ConstantReturn::Bool(false).as_kind_text(),
        ("bool", "0".to_string())
    );
    assert_eq!(
        ConstantReturn::Str("hi".into()).as_kind_text(),
        ("str", "hi".to_string())
    );
}

// ---------------------------------------------------------------------------
// B8. method summaries — effect propagation + return summarisation + arity.
// The interprocedural unit tests cover method *purity*; these pin the effect
// union from a proc callee and the method's return/arity fields.
// ---------------------------------------------------------------------------

#[test]
fn method_calling_impure_proc_unions_its_effects() {
    // A method `m` that calls a global-writing proc `helper` inherits the
    // impurity (a proc callee's effects union into the method).
    let ia = interproc(
        "proc ::helper {} { set ::g 1 }\noo::class create C {\n method m {} { helper }\n}\n",
    );
    let m = ia.methods.get("::C::m").expect("method summary");
    assert!(
        !m.base.pure,
        "a method calling a global-writing proc is impure"
    );
    assert!(
        m.base.direct_calls.contains(&"::helper".to_string()),
        "the method records the proc callee in direct_calls, got {:?}",
        m.base.direct_calls
    );
}

#[test]
fn method_return_constant_and_arity_summarised() {
    // A method `get {a b}` returning a constant records the constant + the
    // exact arity (2 params); MRO/`my`/`next` tracking is left empty by design.
    let ia = interproc("oo::class create C {\n method get {a b} { return 7 }\n}\n");
    let m = ia.methods.get("::C::get").expect("method summary");
    assert!(m.base.returns_constant, "constant return summarised");
    assert_eq!(m.base.constant_return, Some(ConstantReturn::Int(7)));
    assert_eq!(m.base.arity, Arity::exact(2), "two-param arity");
    assert!(m.calls_my.is_empty(), "calls_my left empty (not tracked)");
    assert!(!m.calls_next, "calls_next left false (not tracked)");
    assert!(
        m.base.param_traits.is_empty(),
        "method param_traits left empty by design"
    );
}

// ---------------------------------------------------------------------------
// B9. Arity::any vs exact + ProcSummary structural facts already partly covered
// upstream; pin the variadic-arity unbounded mapping for a proc.
// ---------------------------------------------------------------------------

#[test]
fn proc_arity_is_at_least_required_count_for_variadic_decl() {
    // The summary's arity correctly models a trailing `args` slot as
    // unbounded (`at_least(2)`, not `exact(3)`) — matching
    // `interprocedural::arity_from_names` and the identical bare-name
    // formula `taint_interproc` already used, rather than naively
    // counting `args` itself as a required declared parameter.
    let ia = interproc("proc ::f {a b args} { return $a }\n");
    let s = ia.procedures.get("::f").unwrap();
    assert_eq!(
        s.arity,
        Arity::at_least(2),
        "arity is unbounded past the 2 required params, not exact(3)"
    );
    assert_eq!(
        s.params,
        vec!["a".to_string(), "b".to_string(), "args".to_string()]
    );
}
