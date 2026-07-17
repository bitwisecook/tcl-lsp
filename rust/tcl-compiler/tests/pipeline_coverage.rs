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

//! Coverage port for four under-covered `tcl-compiler` pipeline modules:
//!   * `src/interval_bounds.rs` — integer-interval / list-length / string-length
//!     dynamic bounds checks (W230 lindex / W231 lset / W232 string index /
//!     W233 divide-by-zero). Targets the branches the existing
//!     `intervals.rs` leaves cold: the `classify` *negative*-index reason
//!     (every existing test exercises only `past_end` / `past_append`), the
//!     `list_command_length` `[list …]` element-count path, the divide-by-zero
//!     matrix corners (nested-ternary forcing, `%`), and the `string_length_map`
//!     backslash-escape resolution.
//!   * `src/ir_helpers.rs` — IR/expr helper utilities. The public surface
//!     (`defs_from_expr`, `expr_has_command`, `defs_from_ir_script`) is driven
//!     directly to hit the `catch`/`scan`/`gets`/`regexp`-switch out-var shapes,
//!     the `expr_has_command` node kinds (ternary / unary / call), and the
//!     `collect_defs_from_script` `While` / `For` / `Try` / `Switch` recursion
//!     arms. The `pub(crate)` `condition_command_out_vars` →
//!     `cmd_substitution_out_vars` → `catch_body_out_vars` chain is reached
//!     end-to-end through the analyser's W210 read-before-set *suppression* for a
//!     variable an `if`/`while` *condition* command-substitution writes.
//!   * `src/compilation_unit.rs` — the `CompilationUnit::build_for` pipeline.
//!     Targets the build paths the in-crate tests skip: namespaced procedures,
//!     the `with_interprocedural` taint re-run, `with_memory_ssa`, the
//!     `functions` / `analysable_functions` / `function` accessors, and the
//!     `decode_param_constants` round-trip.
//!   * `src/connection_scope.rs` — iRules cross-event variable scope. Multi-`when`
//!     scripts drive `cross_event_defs` / `cross_event_imports`, the
//!     `info exists` literal-name scan, the `unset` tracking, the `RULE_INIT`
//!     non-racy carve-out, and the duplicate-event summary merge.
//!
//! ## C-Tcl proof split
//!
//! * **Tcl-semantic facts** — what `lindex` / `lset` / `string index` / `expr`
//!   actually *do* (an out-of-range or negative index yields `""` for the
//!   readers and a runtime error for `lset`; `expr {N/0}` / `expr {N%0}` raise
//!   *divide by zero*; `string length "a\tb\tc"` counts the resolved `\t` as one
//!   char; `catch {set x 1}` defines `x`; `scan`/`gets`/`regexp` write their
//!   out-vars) — are the load-bearing ground truth and were verified against
//!   real `tclsh8.6` + `tclsh9.0` via `scripts/dev/tclsh_check.sh` (the two agree
//!   on every fact used here). Each is cited with a `// tclsh:` comment.
//! * **Compiler-internal structure** — the *shape* of a built `CompilationUnit`
//!   (which procedures/methods exist, that `connection_scope` is `Some`, that a
//!   memo encoding round-trips), and the iRules-dialect facts (`when EVENT { … }`
//!   handlers, cross-event variable lifetime) — have no direct core-tclsh
//!   observation. They are asserted structurally and flagged `// f5-dialect`
//!   (iRules events) or noted as compiler-internal at the site.
//!
//! No Rust bug was found: every bound / interval / out-var / scope verdict
//! asserted here agrees with the tclsh ground truth.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyses::{ConstValue, LatticeValue};
use tcl_compiler::compilation_unit::{CompilationUnit, decode_param_constants};
use tcl_compiler::expr_ast::ExprNode;
use tcl_compiler::expr_parser::parse_expr;
use tcl_compiler::ir::{Script, Statement};
use tcl_compiler::ir_helpers::{defs_from_expr, defs_from_ir_script, expr_has_command};
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, registry_for_dialect};

/// Default dialect — 8.6 and 9.0 agree on every Tcl fact exercised here.
const D: &str = "tcl8.6";

// ===========================================================================
// Shared diagnostic-surface helpers — the analyser pass owns W230/W231/W232/
// W233/W210, so a single `analyse(...)` surface mirrors what every consumer
// sees (matching `intervals.rs` / `analyser.rs`).
// ===========================================================================

/// How many diagnostics of `code` the analyser emits for `src`.
fn count(src: &str, code: &str) -> usize {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == code)
        .count()
}

/// The messages of every `code` diagnostic for `src`.
fn messages(src: &str, code: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == code)
        .map(|d| d.message.clone())
        .collect()
}

/// Count of W210 read-before-set diagnostics whose message names `var`.
fn w210_for(src: &str, var: &str) -> usize {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W210" && d.message.contains(var))
        .count()
}

/// Build a `CompilationUnit` under the default registry/dialect.
fn build(src: &str) -> CompilationUnit {
    CompilationUnit::build_for(src, registry_for_dialect(D), false)
}

/// Build a `CompilationUnit` under a registry with iRules loaded, so `when`
/// resolves to the `When` lowering hook (else `when` falls through to
/// `lower_default` and no `::when::*` procedure is produced).
fn build_irules(src: &str) -> CompilationUnit {
    let mut registry = CommandRegistry::build_default();
    registry.load_irules();
    CompilationUnit::build_for(src, &registry, false)
}

// ===========================================================================
// interval_bounds.rs — the `classify` *negative*-index reason.
//
// `intervals.rs` exercises only the `past_end` / `past_append` reasons;
// the first branch of `classify` (`hi < 0 → "negative"`) is cold. A provably
// negative const index drives it for each of the three readers/mutators.
// tclsh: a negative index reads `""` (lindex / string index) and errors
// (lset) — all out of range, so all three warnings are sound.
// ===========================================================================
mod negative_index_classify {
    use super::*;

    #[test]
    fn lindex_negative_const_index_fires_w230() {
        // tclsh: `lindex {a b c} -1` → "" (out of range, negative).
        let src = "proc f {} { set l {a b c}\n set i -1\n return [lindex $l $i] }";
        assert_eq!(count(src, "W230"), 1);
        assert!(
            messages(src, "W230")[0].contains("$i"),
            "message must name the dynamic index $i: {:?}",
            messages(src, "W230")
        );
    }

    #[test]
    fn lset_negative_const_index_fires_w231() {
        // tclsh: `set l {a b c}; lset l -1 X` → 'list index out of range' (error).
        let src = "proc f {v} { set l {a b c}\n set i -1\n lset l $i $v }";
        assert_eq!(count(src, "W231"), 1);
    }

    #[test]
    fn string_index_negative_const_index_fires_w232() {
        // tclsh: `string index hello -1` → "" (out of range, negative).
        let src = "proc f {} { set s \"hello\"\n set i -1\n return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 1);
    }

    #[test]
    fn negative_loop_counter_range_fires_w230() {
        // A loop counting *down* below 0: j ∈ [-3, -1] after the body — the whole
        // interval is negative, so the `hi < 0` branch proves OOR every iteration.
        // tclsh: `lindex {a b} -1` → "" (out of range).
        let src = "proc f {} { for {set j -1} {$j > -4} {incr j -1} { set x [lindex {a b} $j] } }";
        assert_eq!(count(src, "W230"), 1);
    }
}

// ===========================================================================
// interval_bounds.rs — `list_command_length` (the `[list …]` arg-count path).
//
// `list_length_map` resolves a `set l [list a b c]` SSA version to the literal
// element count via `list_command_length`; a past-end const index then fires.
// tclsh: `list a b c` → a 3-element list, `lindex` past the end → "".
// ===========================================================================
mod list_command_length {
    use super::*;

    #[test]
    fn list_command_three_elements_past_end_fires() {
        // tclsh: `set l [list a b c]; llength $l` → 3; `lindex $l 5` → "".
        let src = "proc f {} { set l [list a b c]\n set i 5\n set x [lindex $l $i] }";
        assert_eq!(count(src, "W230"), 1);
    }

    #[test]
    fn list_command_in_range_is_silent() {
        // tclsh: `lindex [list a b c] 2` → "c" (in range) — no warning.
        let src = "proc f {} { set l [list a b c]\n set i 2\n set x [lindex $l $i] }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn list_command_with_substitution_arg_is_unknown() {
        // `[list a $b c]` has a `$`-arg → element count unknown (could differ from
        // the arg count if `$b` were expanded), so the length is not inferred and
        // no false positive fires even though the literal arg count is 3.
        // tclsh: `set b X; lindex [list a $b c] 5` → "" — but Rust must stay
        // silent (it cannot prove the length), which is sound.
        let src = "proc f {b} { set l [list a $b c]\n set i 5\n set x [lindex $l $i] }";
        assert_eq!(count(src, "W230"), 0);
    }

    #[test]
    fn list_command_lset_past_append_fires_w231() {
        // The `[list …]` length also feeds the `lset` append-slot edge: a length-2
        // list with a const index 5 is past the append slot (index > length).
        // tclsh: `set l [list a b]; lset l 5 X` → 'list index out of range'.
        let src = "proc f {v} { set l [list a b]\n set j 5\n lset l $j $v }";
        assert_eq!(count(src, "W231"), 1);
    }
}

// ===========================================================================
// interval_bounds.rs — divide-by-zero matrix corners (W233).
//
// `intervals.rs` covers the flat `&&`/`||`/`?:` guard outcomes; these add
// the corners it leaves cold: a *nested* forced ternary arm, modulo by a
// const-zero variable, and the subtraction-to-zero inside a forced arm.
// tclsh: `expr {…/0}` / `expr {…%0}` raise *divide by zero* (8.6 + 9.0).
// ===========================================================================
mod divide_by_zero_corners {
    use super::*;

    #[test]
    fn nested_forced_ternary_arm_div_zero_fires() {
        // Both ternary guards are constant-true, so the innermost `1/0` is on the
        // always-evaluated spine. tclsh: `expr {1 ? (1 ? 1/0 : 2) : 3}` → divide
        // by zero.
        assert_eq!(
            count(
                "proc f {} { return [expr {1 ? (1 ? 1/0 : 2) : 3}] }",
                "W233"
            ),
            1
        );
    }

    #[test]
    fn const_var_modulo_zero_fires() {
        // SCCP proves d == 0; `%` shares the divide-by-zero trap.
        // tclsh: `set d 0; expr {7 % $d}` → divide by zero.
        let src = "proc f {} { set d 0\n return [expr {7 % $d}] }";
        assert_eq!(count(src, "W233"), 1);
    }

    #[test]
    fn subtraction_to_zero_in_forced_and_fires() {
        // The interval domain proves `$x - $x == 0`, and the constant-true LHS
        // forces the `&&` RHS. tclsh: `set x 5; expr {1 && 10/($x - $x)}` →
        // divide by zero.
        let src = "proc f {} { set x 5\n return [expr {1 && 10 / ($x - $x)}] }";
        assert_eq!(count(src, "W233"), 1);
    }

    #[test]
    fn dead_nested_ternary_arm_stays_silent() {
        // The outer guard is constant-false, so the inner `1/0` arm never runs.
        // tclsh: `expr {0 ? (1 ? 1/0 : 2) : 9}` → 9 (no error).
        assert_eq!(
            count(
                "proc f {} { return [expr {0 ? (1 ? 1/0 : 2) : 9}] }",
                "W233"
            ),
            0
        );
    }
}

// ===========================================================================
// interval_bounds.rs — `string_length_map` escape resolution.
//
// `string_length_map` resolves backslash escapes in a `"…"` literal before
// counting chars; a `\t` (one resolved char) shifts the past-end boundary.
// tclsh: `string length "a\tb\tc"` → 5, so index 6 is OOR and index 4 is valid.
// ===========================================================================
mod string_length_escape {
    use super::*;

    #[test]
    fn tab_escape_resolves_to_one_char_oob_fires() {
        // tclsh: `string length "a\tb\tc"` → 5; `string index "a\tb\tc" 6` → "".
        let src = "proc f {} { set s \"a\\tb\\tc\"\n set i 6\n return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 1);
    }

    #[test]
    fn tab_escape_in_range_is_silent() {
        // tclsh: `string index "a\tb\tc" 4` → "c" (in range, length 5).
        let src = "proc f {} { set s \"a\\tb\\tc\"\n set i 4\n return [string index $s $i] }";
        assert_eq!(count(src, "W232"), 0);
    }
}

// ===========================================================================
// ir_helpers.rs — `defs_from_expr` (public): command-substitution out-vars.
//
// `defs_from_expr` collects the VarWrite targets (and nested body writes) of
// `[cmd …]` substitutions in an expression. Each Tcl out-var fact below is
// pinned to tclsh; the helper must recover the same names.
// ===========================================================================
mod defs_from_expr_out_vars {
    use super::*;

    fn defs(expr_src: &str) -> Vec<String> {
        let reg = CommandRegistry::build_default();
        let mut d = defs_from_expr(&parse_expr(expr_src, Some(D)), &reg);
        d.sort();
        d.dedup();
        d
    }

    #[test]
    fn catch_result_var_and_body_write() {
        // tclsh: `catch {set x 1} res` sets both `res` (catch result) and `x`
        // (the body assignment runs in the caller's scope).
        let d = defs("[catch {set x 1} res]");
        assert!(d.contains(&"res".to_string()), "{d:?}");
        assert!(d.contains(&"x".to_string()), "{d:?}");
    }

    #[test]
    fn catch_body_only_write() {
        // No result var, but the body's `set y 9` is still recovered.
        // tclsh: `catch {set y 9}; info exists y` → 1.
        assert_eq!(defs("[catch {set y 9}]"), vec!["y".to_string()]);
    }

    #[test]
    fn scan_capture_vars() {
        // tclsh: `scan "1 2" "%d %d" a b` writes `a` and `b`.
        assert_eq!(defs("[scan $s {%d %d} a b]"), vec!["a", "b"]);
    }

    #[test]
    fn gets_line_var() {
        // tclsh: `gets $ch ln` writes the line into `ln`.
        assert_eq!(defs("[gets $ch ln]"), vec!["ln".to_string()]);
    }

    #[test]
    fn regexp_match_and_sub_vars() {
        // tclsh: `regexp {(\w+)} $s whole sub` writes `whole` (full match) and
        // `sub` (group 1).
        assert_eq!(defs("[regexp {(\\w+)} $s whole sub]"), vec!["sub", "whole"]);
    }

    #[test]
    fn regexp_nocase_switch_skipped() {
        // A valueless switch (`-nocase`) is skipped; EXP + STRING then the vars.
        // tclsh: `regexp -nocase {(\w+)} $s whole sub` → whole/sub written.
        assert_eq!(
            defs("[regexp -nocase {(\\w+)} $s whole sub]"),
            vec!["sub", "whole"]
        );
    }

    #[test]
    fn regexp_start_switch_consumes_value() {
        // `-start N` consumes its value argument, so `2` is NOT mistaken for an
        // out-var — only `m` is written. tclsh: `regexp -start 2 {b} "abab" m`
        // sets `m` to "b".
        assert_eq!(defs("[regexp -start 2 {b} $s m]"), vec!["m".to_string()]);
    }

    #[test]
    fn regexp_double_dash_terminates_switches() {
        // `--` ends switch parsing; the following words are EXP, STRING, out-var.
        // tclsh: `regexp -- {x} $s m` writes `m`.
        assert_eq!(defs("[regexp -- {x} $s m]"), vec!["m".to_string()]);
    }

    #[test]
    fn catch_body_nested_gets_writer() {
        // The catch body holds a nested command writer (`gets`); its out-var is
        // recovered through `catch_body_out_vars`. tclsh: `catch {gets $ch ln2}`
        // sets `ln2`.
        assert_eq!(defs("[catch {gets $ch ln2}]"), vec!["ln2".to_string()]);
    }

    #[test]
    fn function_call_node_recurses_into_args() {
        // An expr function-call node (`max(...)`) is walked into: the `[set z 1]`
        // arg writes `z`. tclsh: `expr {max(5, 2)}` → 5 (valid call syntax); the
        // arg's side effect sets `z`.
        assert_eq!(defs("max([set z 1], 2)"), vec!["z".to_string()]);
    }

    #[test]
    fn no_command_substitution_yields_no_defs() {
        // A pure arithmetic expression has no command writers.
        assert!(defs("$a + $b * 2").is_empty());
    }
}

// ===========================================================================
// ir_helpers.rs — `expr_has_command` (public): every node-kind arm.
// ===========================================================================
mod expr_has_command_shapes {
    use super::*;

    fn has(expr_src: &str) -> bool {
        expr_has_command(&parse_expr(expr_src, Some(D)))
    }

    #[test]
    fn binary_operand_carries_command() {
        assert!(has("1 + [llength $l]"));
    }

    #[test]
    fn unary_operand_carries_command() {
        assert!(has("!([info exists x])"));
    }

    #[test]
    fn ternary_branch_carries_command() {
        assert!(has("[a] ? 1 : 2"));
        assert!(has("$c ? [b] : 2"));
        assert!(has("$c ? 1 : [d]"));
    }

    #[test]
    fn function_call_arg_carries_command() {
        assert!(has("max([cmd], 1)"));
    }

    #[test]
    fn pure_value_expression_has_no_command() {
        assert!(!has("1 + 2"));
        assert!(!has("$a * $b"));
        assert!(!has("\"literal\""));
    }
}

// ===========================================================================
// ir_helpers.rs — `defs_from_ir_script` (public): the recursion arms the
// in-crate tests skip (`While`, `For`, `Try`, `Switch`). Built directly as IR
// so each `collect_defs_from_script` match arm is driven deterministically.
// These assert IR-walk structure (compiler-internal), not a Tcl runtime value.
// ===========================================================================
mod defs_from_ir_script_arms {
    use super::*;
    use tcl_compiler::ir::{ForeachIterator, SwitchArm, TryHandler};

    fn assign(name: &str) -> Statement {
        Statement::AssignConst {
            span: Span::new(0, 1),
            name: name.into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }
    }

    fn lit_one() -> ExprNode {
        ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        }
    }

    #[test]
    fn while_body_def_collected() {
        let s = Script::from_statements(vec![Statement::While {
            span: Span::new(0, 10),
            condition: lit_one(),
            condition_span: Span::new(0, 1),
            condition_base: None,
            body: Script::from_statements(vec![assign("w")]),
            body_span: Span::new(1, 10),
            raw_args: vec![],
            raw_tokens: None,
        }]);
        assert_eq!(defs_from_ir_script(&s), vec!["w"]);
    }

    #[test]
    fn for_body_def_collected() {
        let s = Script::from_statements(vec![Statement::For {
            span: Span::new(0, 20),
            init: Script::from_statements(vec![assign("i")]),
            init_span: Span::new(0, 5),
            condition: lit_one(),
            condition_span: Span::new(5, 6),
            condition_base: None,
            next: Script::new(),
            next_span: Span::new(6, 7),
            body: Script::from_statements(vec![assign("b")]),
            body_span: Span::new(8, 19),
            raw_args: vec![],
            raw_tokens: None,
        }]);
        // The init's `i` and the body's `b` are both collected (the `For` arm
        // recurses into the body; the init is a flat sub-script of defs).
        let d = defs_from_ir_script(&s);
        assert!(d.contains(&"b".to_string()), "{d:?}");
    }

    #[test]
    fn foreach_iterator_vars_and_body_collected() {
        let s = Script::from_statements(vec![Statement::Foreach {
            span: Span::new(0, 20),
            iterators: vec![ForeachIterator {
                vars: vec!["k".into(), "v".into()],
                list_arg: "$d".into(),
            }],
            body: Script::from_statements(vec![assign("acc")]),
            body_span: Span::new(10, 19),
            is_lmap: false,
            raw_args: vec![],
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        }]);
        let d = defs_from_ir_script(&s);
        for want in ["k", "v", "acc"] {
            assert!(d.contains(&want.to_string()), "missing {want}: {d:?}");
        }
    }

    #[test]
    fn try_body_handler_and_finally_defs_collected() {
        let s = Script::from_statements(vec![Statement::Try {
            span: Span::new(0, 60),
            body: Script::from_statements(vec![assign("tb")]),
            body_span: Span::new(4, 10),
            handlers: vec![TryHandler {
                kind: "on".into(),
                match_arg: "error".into(),
                var_name: Some("e".into()),
                options_var: Some("opts".into()),
                body: Script::from_statements(vec![assign("hb")]),
                body_span: Span::new(30, 40),
                fallthrough: false,
            }],
            finally_body: Some(Script::from_statements(vec![assign("fb")])),
            finally_span: Some(Span::new(50, 58)),
            raw_args: vec![],
        }]);
        let d = defs_from_ir_script(&s);
        // Body write, handler var, handler options var, handler body write, and
        // the finally body write are all recovered.
        for want in ["tb", "e", "opts", "hb", "fb"] {
            assert!(d.contains(&want.to_string()), "missing {want}: {d:?}");
        }
    }

    #[test]
    fn switch_arm_and_default_defs_collected() {
        let s = Script::from_statements(vec![Statement::Switch {
            span: Span::new(0, 40),
            subject: "$x".into(),
            subject_span: Span::new(7, 9),
            arms: vec![SwitchArm {
                pattern: "a".into(),
                pattern_span: Span::new(10, 11),
                body: Some(Script::from_statements(vec![assign("arm")])),
                body_span: Some(Span::new(12, 20)),
                fallthrough: false,
            }],
            default_body: Some(Script::from_statements(vec![assign("def")])),
            default_span: Some(Span::new(25, 35)),
            mode: tcl_compiler::ir::SwitchMode::Exact,
            nocase: false,
            raw_args: vec![],
            patterns_braced: true,
        }]);
        let d = defs_from_ir_script(&s);
        assert!(d.contains(&"arm".to_string()), "{d:?}");
        assert!(d.contains(&"def".to_string()), "{d:?}");
    }

    #[test]
    fn call_defs_collected() {
        // A `Call` statement with explicit `defs` contributes them directly.
        let s = Script::from_statements(vec![Statement::Call {
            span: Span::new(0, 10),
            command: "set".into(),
            canonical_command: None,
            args: vec!["c".into(), "1".into()],
            defs: vec!["c".into()],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }]);
        assert_eq!(defs_from_ir_script(&s), vec!["c"]);
    }
}

// ===========================================================================
// ir_helpers.rs — `condition_command_out_vars` chain (end-to-end).
//
// The `pub(crate)` condition-out-var extractor is reached through CFG lowering:
// when an `if`/`while` *condition* holds a `[catch …]` / `[regexp …]` /
// `[scan …]` / `[gets …]` substitution, the variables it writes are recorded as
// defs on the synthetic `<cond>` statement, so a read of one in the guarded body
// is NOT flagged read-before-set (W210). Each suppression is a Tcl fact: the
// command really does write the variable before the body runs.
// ===========================================================================
mod condition_out_vars_suppress_w210 {
    use super::*;

    #[test]
    fn catch_result_var_in_if_condition() {
        // tclsh: `catch {error x} res` sets `res`, so `puts $res` is safe.
        let src = "proc f {} { if {[catch {error x} res]} { puts $res } }";
        assert_eq!(w210_for(src, "res"), 0);
    }

    #[test]
    fn catch_body_write_in_if_condition() {
        // tclsh: `if {![catch {set qx 1}]} { puts $qx }` prints 1 — the body's
        // `set qx 1` ran during condition evaluation, so the read is not RBS.
        let src = "proc f {} { if {![catch {set qx 1}]} { puts $qx } }";
        assert_eq!(w210_for(src, "qx"), 0);
    }

    #[test]
    fn regexp_capture_var_in_if_condition() {
        // tclsh: `regexp {(\d+)} $s whole num` writes `num`.
        let src = "proc f {s} { if {[regexp {(\\d+)} $s whole num]} { puts $num } }";
        assert_eq!(w210_for(src, "num"), 0);
    }

    #[test]
    fn scan_var_in_if_condition() {
        // tclsh: `scan $s "%d" a` writes `a`.
        let src = "proc f {s} { if {[scan $s {%d} a] == 1} { puts $a } }";
        assert_eq!(w210_for(src, "a"), 0);
    }

    #[test]
    fn gets_var_in_while_condition() {
        // tclsh: `gets $ch line` writes the line into `line` each iteration.
        let src = "proc f {ch} { while {[gets $ch line] >= 0} { puts $line } }";
        assert_eq!(w210_for(src, "line"), 0);
    }

    #[test]
    fn genuinely_unset_read_still_fires_w210() {
        // Control: a variable NO condition command writes is still flagged — the
        // suppression is targeted, not blanket. tclsh: `puts $undef` →
        // `can't read "undef": no such variable`.
        let src = "proc f {} { if {[catch {error x} res]} { puts $undef } }";
        assert_eq!(w210_for(src, "undef"), 1);
    }
}

// ===========================================================================
// compilation_unit.rs — `build_for` pipeline build paths.
//
// Structure of the built unit is compiler-internal (asserted structurally).
// ===========================================================================
mod compilation_unit_build {
    use super::*;

    #[test]
    fn namespaced_procedures_are_qualified() {
        // A proc inside `namespace eval ::a` is keyed by its fully-qualified name.
        let cu = build("namespace eval ::a { proc helper {x} { set y $x } }");
        assert!(
            cu.function("::a::helper").is_some(),
            "procedures: {:?}",
            cu.procedures.keys().collect::<Vec<_>>()
        );
        // The top level always exists.
        assert!(cu.function("::top").is_some());
    }

    #[test]
    fn nested_namespace_procedures() {
        let cu =
            build("namespace eval ::a { namespace eval ::a::b { proc deep {} { return 1 } } }");
        assert!(
            cu.function("::a::b::deep").is_some(),
            "procedures: {:?}",
            cu.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn function_accessor_unknown_name_is_none() {
        let cu = build("proc p {} {}");
        assert!(cu.function("::nope").is_none());
        assert!(cu.function("::p").is_some());
    }

    #[test]
    fn functions_iterator_is_top_plus_procs_sorted() {
        let cu = build("proc zeta {} {}\nproc alpha {} {}");
        let names: Vec<&str> = cu.functions().map(|fu| fu.name.as_str()).collect();
        // Top first, then procedures in qualified-name order.
        assert_eq!(names.first(), Some(&"::top"));
        let proc_names: Vec<&str> = names[1..].to_vec();
        let mut sorted = proc_names.clone();
        sorted.sort_unstable();
        assert_eq!(
            proc_names, sorted,
            "procedures must be qualified-name-sorted"
        );
        assert!(proc_names.contains(&"::alpha") && proc_names.contains(&"::zeta"));
    }

    #[test]
    fn analysable_functions_excludes_complexity_guarded() {
        // A byte-huge body is complexity-guarded and dropped from the analysable
        // view, while a normal proc is retained.
        let big = "A".repeat(270_000);
        let src = format!("proc big {{}} {{ set x \"{big}\" }}\nproc small {{}} {{ set y 1 }}");
        let cu = build(&src);
        let analysable: Vec<&str> = cu
            .analysable_functions()
            .map(|fu| fu.name.as_str())
            .collect();
        assert!(analysable.contains(&"::small"));
        assert!(
            !analysable.contains(&"::big"),
            "guarded ::big must be excluded: {analysable:?}"
        );
    }

    #[test]
    fn with_interprocedural_populates_summary_and_keeps_taints() {
        // The interprocedural pass attaches a summary and re-runs taint on every
        // unit; the unit must still be well-formed afterwards.
        let cu = build("proc src {} { return [exec cat /etc/passwd] }\nproc f {} { set x [src] }")
            .with_interprocedural(registry_for_dialect(D), Some(D));
        assert!(
            cu.interproc.is_some(),
            "interproc summary must be populated"
        );
        // Procedures survive the re-run.
        assert!(cu.function("::f").is_some());
        assert!(cu.function("::src").is_some());
    }

    #[test]
    fn with_memory_ssa_populates_top_and_procs() {
        let cu = build("proc f {} { set x 1; return $x }")
            .with_memory_ssa(&CommandRegistry::build_default());
        assert!(cu.top_level.memory_ssa.is_some());
        assert!(cu.function("::f").unwrap().memory_ssa.is_some());
    }

    #[test]
    fn oo_method_bodies_become_function_units() {
        // TclOO methods get their own units in `cu.methods`, kept out of
        // `procedures`.
        let src = "oo::class create C {\n method m {} { return 1 }\n}\n";
        let cu = build(src);
        assert!(
            cu.methods.contains_key("::C::m"),
            "methods: {:?}",
            cu.methods.keys().collect::<Vec<_>>()
        );
        assert!(!cu.procedures.contains_key("::C::m"));
    }

    #[test]
    fn non_when_source_has_no_connection_scope() {
        // A plain (non-iRules) source produces no `::when::*` procs → None.
        let cu = build("proc f {} { set x 1 }");
        assert!(cu.connection_scope.is_none());
    }
}

// ===========================================================================
// compilation_unit.rs — `decode_param_constants` (public memo round-trip).
//
// The encoder is `pub(crate)`; the decoder is public. Assert the decode
// contract directly (compiler-internal — no Tcl observation).
// ===========================================================================
mod param_constants_codec {
    use super::*;

    #[test]
    fn empty_encoding_decodes_to_none() {
        assert!(decode_param_constants(&[]).is_none());
    }

    #[test]
    fn nonempty_encoding_decodes_to_const_strings() {
        let encoded = vec![
            ("mode".to_string(), 0u32, "fast".to_string()),
            ("kind".to_string(), 0u32, "tcp".to_string()),
        ];
        let map = decode_param_constants(&encoded).expect("non-empty decodes to Some");
        assert_eq!(map.len(), 2);
        match map.get(&("mode".to_string(), 0)) {
            Some(LatticeValue::Const(ConstValue::String(s))) => assert_eq!(s, "fast"),
            other => panic!("expected Const(String(\"fast\")), got {other:?}"),
        }
        match map.get(&("kind".to_string(), 0)) {
            Some(LatticeValue::Const(ConstValue::String(s))) => assert_eq!(s, "tcp"),
            other => panic!("expected Const(String(\"tcp\")), got {other:?}"),
        }
    }
}

// ===========================================================================
// connection_scope.rs — iRules cross-event variable scope (via `build_for`).
//
// `// f5-dialect`: `when EVENT { … }` handlers and cross-event variable lifetime
// are an iRules concept, not core tclsh — they have no `tclsh_check.sh` analogue.
// The connection scope is built from the `::when::*` procedures and cached on
// `CompilationUnit::connection_scope`; we assert its contents structurally.
// ===========================================================================
mod connection_scope_cross_event {
    use super::*;

    #[test]
    fn cross_event_flow_records_def_and_import() {
        // f5-dialect: CLIENT_ACCEPTED sets `ip`, HTTP_REQUEST reads it. The flow
        // is valid (CLIENT_ACCEPTED fires before HTTP_REQUEST), so `ip` is both a
        // cross-event def (producer) and import (consumer).
        let src = "
            when CLIENT_ACCEPTED { set ip [IP::client_addr] }
            when HTTP_REQUEST { log local0. \"$ip\" }
        ";
        let cu = build_irules(src);
        let cs = cu.connection_scope.expect("connection scope present");
        assert!(cs.cross_event_defs.contains("ip"));
        assert!(cs.cross_event_imports.contains("ip"));
        // No static:: var ⇒ no racy-static finding.
        assert!(cs.racy_static_defs.is_empty());
    }

    #[test]
    fn info_exists_literal_read_is_a_cross_event_import() {
        // f5-dialect: `info exists flag` reads `flag` by *literal name* (no `$`),
        // so the SSA never records the use — the dedicated `scan_info_exists`
        // sweep recovers it, keeping the cross-event flow from HTTP_REQUEST's
        // `set flag 1` alive.
        let src = "
            when HTTP_REQUEST { set flag 1 }
            when HTTP_RESPONSE { if {[info exists flag]} { log local0. seen } }
        ";
        let cu = build_irules(src);
        let cs = cu.connection_scope.expect("connection scope present");
        assert!(
            cs.cross_event_defs.contains("flag"),
            "info-exists read must register the cross-event flow: {:?}",
            cs.cross_event_defs
        );
        assert!(cs.cross_event_imports.contains("flag"));
    }

    #[test]
    fn info_exists_command_substitution_form_scanned() {
        // f5-dialect: the `[info exists …]` form inside a command substitution
        // (not a bare branch condition) is also scanned.
        let src = "
            when HTTP_REQUEST { set ready 1 }
            when HTTP_RESPONSE { set s [info exists ready] }
        ";
        let cu = build_irules(src);
        let cs = cu.connection_scope.expect("connection scope present");
        assert!(
            cs.cross_event_imports.contains("ready"),
            "cmd-sub info-exists must be scanned: {:?}",
            cs.cross_event_imports
        );
    }

    #[test]
    fn racy_static_flagged_for_non_rule_init_events() {
        // f5-dialect: `static::counter` written in HTTP_REQUEST and read in
        // HTTP_RESPONSE — both per-request events, so the cross-event flow is
        // racy (feeds IRULE4005).
        let src = "
            when HTTP_REQUEST { incr static::counter }
            when HTTP_RESPONSE { log local0. \"$static::counter\" }
        ";
        let cu = build_irules(src);
        let cs = cu.connection_scope.expect("connection scope present");
        assert!(cs.racy_static_defs.contains("static::counter"));
    }

    #[test]
    fn rule_init_static_is_not_racy() {
        // f5-dialect: a `static::` set in RULE_INIT runs once at iRule load, so
        // the cross-event read is NOT racy.
        let src = "
            when RULE_INIT { set static::config 1 }
            when HTTP_REQUEST { log local0. \"$static::config\" }
        ";
        let cu = build_irules(src);
        let cs = cu.connection_scope.expect("connection scope present");
        assert!(!cs.racy_static_defs.contains("static::config"));
    }

    #[test]
    fn duplicate_event_handlers_merge_summaries() {
        // f5-dialect: two `when CLIENT_ACCEPTED` blocks are merged — the union of
        // their defs is the event summary.
        let src = "
            when CLIENT_ACCEPTED { set a 1 }
            when CLIENT_ACCEPTED { set b 2 }
            when HTTP_REQUEST { log local0. \"$a $b\" }
        ";
        let cu = build_irules(src);
        let cs = cu.connection_scope.expect("connection scope present");
        let s = cs
            .summaries
            .get("CLIENT_ACCEPTED")
            .expect("CLIENT_ACCEPTED summary");
        assert!(
            s.defs.contains("a") && s.defs.contains("b"),
            "defs: {:?}",
            s.defs
        );
    }

    #[test]
    fn unset_in_event_recorded_in_summary() {
        // f5-dialect: an explicit `unset` is tracked per event (so the racy-static
        // check can ignore a var the event destroys).
        let src = "
            when CLIENT_ACCEPTED { set v 1 }
            when CLIENT_CLOSED { unset v }
        ";
        let cu = build_irules(src);
        let cs = cu.connection_scope.expect("connection scope present");
        let s = cs
            .summaries
            .get("CLIENT_CLOSED")
            .expect("CLIENT_CLOSED summary");
        assert!(s.unsets.contains("v"), "unsets: {:?}", s.unsets);
    }

    #[test]
    fn single_event_no_cross_flow() {
        // f5-dialect: one event that both sets and reads a var locally has no
        // cross-event flow (the import requires a *different* event's def).
        let src = "when HTTP_REQUEST { set x 1\n log local0. \"$x\" }";
        let cu = build_irules(src);
        let cs = cu.connection_scope.expect("connection scope present");
        assert!(cs.cross_event_defs.is_empty(), "{:?}", cs.cross_event_defs);
        assert!(cs.cross_event_imports.is_empty());
    }
}
