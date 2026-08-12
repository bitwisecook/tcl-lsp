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

//! Residual coverage: the uncovered branches in four `tcl-compiler`
//! files that the existing depth suites only partially reach —
//!   * `src/codegen/cmd_subst.rs`            (the less-common substitution shapes)
//!   * `src/codegen/mod.rs`                  (the `CodegenCtx` driver / errorInfo machinery)
//!   * `src/var_escape/cfg_propagation/walker.rs` (CFG-propagation arms)
//!   * `src/var_escape/walker.rs`            (intra-procedural escape arms)
//!
//! ## Disjoint from the existing suites
//!
//! This file deliberately does NOT re-assert what
//! `tests/codegen.rs`, `tests/codegen_depth.rs`,
//! `tests/var_escape_typeinfer.rs`, and `tests/var_escape_cfg.rs`
//! already cover (the common emitter paths, the happy-path if/for/while/
//! switch/proc lowering, the `Backend` trait entry points, the conditional-
//! upvar / loop / switch-arm escape cases, the `cfg_result` low-level surface).
//! Every test below drives an arm those suites skip — typically a rare /
//! fallback / option-flag / dynamic shape:
//!
//! * `cmd_subst.rs`: `string equal -nocase` / `compare -length`
//!   (`INVOKE_REPLACE`), `string is integer/double/boolean ?-strict?`,
//!   the `string replace 0 N` fast path vs the generic `STR_REPLACE`,
//!   `regexp -nocase` glob lowering vs plain `REGEXP`, `linsert`/`lreplace`
//!   (`LREPLACE4`), `dict get` multi-key, `array exists`/`array names`,
//!   `info exists` (scalar + stk), the multi-part `$`/`[`-interpolation
//!   decomposition of an arg (`x$y`, `be(a:$a)`), the braced-with-subst
//!   and backslash arg arms, and the `incr ::g <big>` stk-range guard.
//! * `mod.rs`: the `CodegenCtx` lifecycle the emitter free functions don't
//!   exercise directly — `set_source` + `span_text` (with the quoted-word
//!   trailing-`"` widen), `span_line`, `emit` vs `emit_comment` span/line
//!   stamping, `fresh_label`/`place_label`/`into_function_asm`.
//! * the two `var_escape` walkers: the non-frameless command-substitution
//!   head in a value (`record_fallback`), the `direct_callees` population,
//!   the `Return` / `ExprEval` value scans that catch `[info exists ...]`,
//!   the standalone `namespace upvar` handler, and the name-first
//!   `append`/`lappend` literal-vs-dynamic arms.
//!
//! ## C-Tcl proof split
//!
//! Opcode-shape, escape-lattice structure, label resolution, and the
//! per-`(name)` escape tags are **compiler-internal** (how the emitter lays
//! out bytecode / how the analysis tags a name) with no direct Tcl runtime
//! analogue, so they are asserted **structurally**. This note discharges
//! the structural-assertion requirement for the whole file.
//!
//! Where a snippet pins an **observable computed value** that a specialised
//! command produces, or an **escape verdict that mirrors a real scoping
//! fact** (a write through `upvar`/`global`/`namespace upvar`/`info exists`
//! observable in another frame), the value/behaviour is confirmed against
//! real `tclsh` (8.6 + 9.0 agree on every case here) via
//! `scripts/dev/tclsh_check.sh` and cited inline with a `// tclsh:` comment.
//! The `tcl-vm` crate is not a `tcl-compiler` dependency, so an end-to-end
//! VM execution is unavailable here; the specialised opcode the emitter
//! selects is the carrier of that computed value and is what the structural
//! assertion pins, with the value itself proven against tclsh.

use std::collections::HashMap;

use tcl_compiler::cfg_builder::build_cfg;
use tcl_compiler::codegen::{CodegenCtx, FunctionAsm, Op, Operand, codegen_module};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::var_escape::{
    EscapeTag, ProcEscapeSummary, analyse_var_escape, analyse_var_escape_cu,
};
use tcl_lexer::Span;
use tcl_registry::{CommandRegistry, registry_for_dialect};

// ── Shared helpers ──────────────────────────────────────────────────────────

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// A proc-flavour `CodegenCtx` for driving the inline-emission helpers
/// directly (the surface-syntax path to several of these arms is narrow, so
/// the helper is driven on a hand-built context — the same idiom the existing
/// `codegen_depth.rs` uses for `try_list_expand_concat` &c.).
fn proc_ctx<'r>(reg: &'r CommandRegistry, params: &[&str]) -> CodegenCtx<'r> {
    CodegenCtx::new(true, params, reg)
}

fn ctx_ops(ctx: &CodegenCtx) -> Vec<Op> {
    ctx.instructions.iter().map(|i| i.op).collect()
}

fn count(ops: &[Op], op: Op) -> usize {
    ops.iter().filter(|&&o| o == op).count()
}

/// End-to-end: the opcode stream of a named proc lowered from `source`.
fn proc_ops(source: &str, qname: &str) -> Vec<Op> {
    proc_asm(source, qname)
        .instructions
        .iter()
        .map(|i| i.op)
        .collect()
}

fn proc_asm(source: &str, qname: &str) -> FunctionAsm {
    let reg = registry();
    let ir = lower_to_ir(source, &reg);
    let cfg = build_cfg(&ir, false);
    let mut module = codegen_module(&cfg, &ir, &reg);
    module
        .procedures
        .remove(qname)
        .unwrap_or_else(|| panic!("procedure {qname} not found"))
}

// ════════════════════════════════════════════════════════════════════════════
// PART A — codegen/cmd_subst.rs : the less-common substitution shapes
// ════════════════════════════════════════════════════════════════════════════

// ── A.1  string equal/compare with option flags → INVOKE_REPLACE ────────────

#[test]
fn string_equal_nocase_uses_invoke_replace() {
    // `string equal -nocase a b` cannot use the bare STR_EQ fast path (that is
    // the 2-arg form only); the `-nocase` flag routes through INVOKE_REPLACE
    // against `::tcl::string::equal`. (emit_inline_string_invoke_replace.)
    // tclsh: string equal -nocase ABC abc  →  1   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["a", "b"]);
    ctx.emit_inline_cmd_subst("[string equal -nocase $a $b]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::INVOKE_REPLACE),
        "string equal -nocase → INVOKE_REPLACE: {ops:?}"
    );
    assert!(
        !ops.contains(&Op::STR_EQ),
        "the -nocase form must not take the bare STR_EQ path"
    );
    // The FQN sub-emitter target is interned.
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|l| l == "::tcl::string::equal"),
        "FQN target interned: {:?}",
        ctx.literals.entries()
    );
    // The INVOKE_REPLACE start-command end label resolves (regression guard).
    assert_all_labels_resolve(ctx);
}

#[test]
fn string_equal_nocase_end_to_end_through_proc() {
    // The same `-nocase` form reached through real surface syntax in a proc
    // body still selects INVOKE_REPLACE.
    let ops = proc_ops(
        "proc p {a b} { return [string equal -nocase $a $b] }",
        "::p",
    );
    assert!(
        ops.contains(&Op::INVOKE_REPLACE),
        "surface-syntax -nocase → INVOKE_REPLACE: {ops:?}"
    );
}

#[test]
fn string_compare_length_uses_invoke_replace() {
    // `string compare -length 3 a b` → INVOKE_REPLACE with the `-length` flag
    // prefix, distinct from the bare STR_CMP 2-arg form.
    // tclsh: string compare -length 3 abcZ abcQ  →  0   (8.6 + 9.0; first 3 equal)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["a", "b"]);
    ctx.emit_inline_cmd_subst("[string compare -length 3 $a $b]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::INVOKE_REPLACE),
        "string compare -length → INVOKE_REPLACE: {ops:?}"
    );
    assert!(!ops.contains(&Op::STR_CMP));
    assert!(ctx.literals.entries().iter().any(|l| l == "-length"));
}

#[test]
fn string_compare_nocase_uses_invoke_replace() {
    // The 3-arg `-nocase` compare form also routes through INVOKE_REPLACE.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["a", "b"]);
    ctx.emit_inline_cmd_subst("[string compare -nocase $a $b]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::INVOKE_REPLACE),
        "compare -nocase: {ops:?}"
    );
}

// ── A.2  string is CLASS ?-strict? — the per-class lowering arms ─────────────

#[test]
fn string_is_integer_non_strict_lowers_to_numeric_type_chain() {
    // `string is integer $v` (no -strict): the non-strict integer arm emits the
    // numericType/empty-string/LE(3) chain (treats "" as a member). This is the
    // longest of the `emit_inline_string_is` arms and is not exercised elsewhere.
    // tclsh: string is integer 42  →  1 ;  string is integer ""  →  1   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["v"]);
    ctx.emit_inline_cmd_subst("[string is integer $v]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::NUMERIC_TYPE),
        "integer test uses NUMERIC_TYPE: {ops:?}"
    );
    assert!(ops.contains(&Op::LE), "the <=3 type-class check: {ops:?}");
    assert!(
        !ops.contains(&Op::STR_CLASS),
        "integer is not a STR_CLASS class"
    );
    assert_all_labels_resolve(ctx);
}

#[test]
fn string_is_integer_strict_uses_shorter_chain() {
    // `-strict` integer: the empty string is NOT a member, so the strict arm
    // emits a shorter numericType + LE chain (no empty-string STR_EQ branch).
    // tclsh: string is integer -strict ""  →  0   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["v"]);
    ctx.emit_inline_cmd_subst("[string is integer -strict $v]");
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::NUMERIC_TYPE));
    assert!(ops.contains(&Op::LE));
    // Strict form does not need the empty-string STR_EQ that the non-strict
    // form carries.
    assert!(
        !ops.contains(&Op::STR_EQ),
        "strict integer omits the empty-string compare: {ops:?}"
    );
    assert_all_labels_resolve(ctx);
}

#[test]
fn string_is_double_lowers_to_numeric_type() {
    // `string is double $v` → numericType-based test (the double arm).
    // tclsh: string is double 3.5  →  1   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["v"]);
    ctx.emit_inline_cmd_subst("[string is double $v]");
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::NUMERIC_TYPE), "double arm: {ops:?}");
    assert!(
        ops.contains(&Op::STR_EQ),
        "non-strict double compares against \"\""
    );
    assert_all_labels_resolve(ctx);
}

#[test]
fn string_is_double_strict_omits_empty_compare() {
    // `-strict` double: the strict double arm drops the empty-string STR_EQ.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["v"]);
    ctx.emit_inline_cmd_subst("[string is double -strict $v]");
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::NUMERIC_TYPE));
    assert!(
        !ops.contains(&Op::STR_EQ),
        "strict double omits empty compare: {ops:?}"
    );
}

#[test]
fn string_is_boolean_uses_try_cvt() {
    // `string is boolean $v` → tryCvtToBoolean (its own arm, distinct from the
    // numeric classes).
    // tclsh: string is boolean yes  →  1   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["v"]);
    ctx.emit_inline_cmd_subst("[string is boolean $v]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::TRY_CVT_TO_BOOLEAN),
        "boolean arm → tryCvtToBoolean: {ops:?}"
    );
    assert_all_labels_resolve(ctx);
}

#[test]
fn string_is_charclass_non_strict_uses_str_class() {
    // A character class (`alpha`) with no -strict lowers to the single STR_CLASS
    // opcode (the `str_class_id` fast path).
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["v"]);
    ctx.emit_inline_cmd_subst("[string is alpha $v]");
    let ops = ctx_ops(&ctx);
    assert_eq!(
        ops,
        vec![Op::LOAD_SCALAR1, Op::STR_CLASS],
        "string is alpha → load; strClass"
    );
}

#[test]
fn string_is_charclass_strict_defers_to_generic() {
    // STR_CLASS reports "" as a member, so it cannot honour `-strict` for a
    // character class; the strict char-class arm defers to the generic command
    // (keeping the `is` subcommand word). → generic invoke, not STR_CLASS.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["v"]);
    ctx.emit_inline_cmd_subst("[string is alpha -strict $v]");
    let ops = ctx_ops(&ctx);
    assert!(
        !ops.contains(&Op::STR_CLASS),
        "strict char-class must not use STR_CLASS: {ops:?}"
    );
    assert!(
        ops.contains(&Op::INVOKE_STK1),
        "strict char-class → generic invoke: {ops:?}"
    );
    // The generic fallback keeps both `string` and `is` words.
    let lits = ctx.literals.entries();
    assert!(
        lits.iter().any(|l| l == "string"),
        "keeps `string`: {lits:?}"
    );
    assert!(lits.iter().any(|l| l == "is"), "keeps `is`: {lits:?}");
}

// ── A.3  string replace — fast path vs generic ──────────────────────────────

#[test]
fn string_replace_zero_prefix_uses_reverse_range_concat() {
    // `string replace $s 0 N repl` has a fast path: REVERSE + STR_RANGE_IMM
    // (start = N+1) + STR_CONCAT1 — replacing a leading run is `repl` ++ the
    // tail after index N.
    // tclsh: string replace abcdef 0 1 XY  →  XYcdef   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["s"]);
    ctx.emit_inline_cmd_subst("[string replace $s 0 1 XY]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::STR_RANGE_IMM),
        "fast path uses STR_RANGE_IMM: {ops:?}"
    );
    assert!(ops.contains(&Op::REVERSE));
    assert!(ops.contains(&Op::STR_CONCAT1));
    assert!(
        !ops.contains(&Op::STR_REPLACE),
        "the 0-prefix form does not need the generic STR_REPLACE"
    );
}

#[test]
fn string_replace_general_uses_str_replace() {
    // A non-zero start index (`1 2`) falls out of the fast path and emits the
    // general STR_REPLACE opcode with all four operands.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["s"]);
    ctx.emit_inline_cmd_subst("[string replace $s 1 2 XY]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::STR_REPLACE),
        "general string replace → STR_REPLACE: {ops:?}"
    );
    assert!(!ops.contains(&Op::STR_RANGE_IMM));
}

// ── A.4  regexp — -nocase glob lowering vs plain REGEXP ─────────────────────

#[test]
fn regexp_nocase_literal_pattern_lowers_to_str_match() {
    // `regexp -nocase <glob-able-pattern> $s`: when the regex reduces to a glob
    // the emitter lowers it to STR_MATCH (case-insensitive flag) rather than the
    // REGEXP opcode. The `-nocase` option word is consumed at compile time.
    // tclsh: regexp -nocase abc ABCDEF  →  1   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["s"]);
    ctx.emit_inline_cmd_subst("[regexp -nocase abc $s]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::STR_MATCH),
        "regexp -nocase (glob-able) → STR_MATCH: {ops:?}"
    );
    assert!(!ops.contains(&Op::REGEXP), "the glob path skips REGEXP");
}

#[test]
fn regexp_plain_two_arg_uses_regexp_opcode() {
    // A plain (no `-nocase`) 2-arg `regexp` emits the REGEXP opcode with the
    // advanced-flags operand. This is the sibling arm to the glob path.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["s"]);
    ctx.emit_inline_cmd_subst("[regexp {a.c} $s]");
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::REGEXP), "plain regexp → REGEXP: {ops:?}");
    assert!(!ops.contains(&Op::STR_MATCH));
}

// ── A.5  linsert / lreplace → LREPLACE4 (distinct operand discriminator) ─────

#[test]
fn linsert_lowers_to_lreplace4() {
    // `linsert $l 1 X` shares the LREPLACE4 opcode with `lreplace`, distinguished
    // by the trailing operand (2 = insert).
    // tclsh: linsert {a b c} 1 X  →  a X b c   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["l"]);
    ctx.emit_inline_cmd_subst("[linsert $l 1 X]");
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::LREPLACE4), "linsert → LREPLACE4: {ops:?}");
    // The discriminator operand for linsert is 2.
    let lrep = ctx
        .instructions
        .iter()
        .find(|i| i.op == Op::LREPLACE4)
        .expect("LREPLACE4 present");
    assert_eq!(
        lrep.operands.last(),
        Some(&Operand::Imm(2)),
        "linsert discriminator operand is 2: {:?}",
        lrep.operands
    );
}

#[test]
fn lreplace_lowers_to_lreplace4_with_replace_discriminator() {
    // `lreplace $l 1 2 X` → LREPLACE4 with discriminator 1 (replace).
    // tclsh: lreplace {a b c d} 1 2 X  →  a X d   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["l"]);
    ctx.emit_inline_cmd_subst("[lreplace $l 1 2 X]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::LREPLACE4),
        "lreplace → LREPLACE4: {ops:?}"
    );
    let lrep = ctx
        .instructions
        .iter()
        .find(|i| i.op == Op::LREPLACE4)
        .expect("LREPLACE4 present");
    assert_eq!(
        lrep.operands.last(),
        Some(&Operand::Imm(1)),
        "lreplace discriminator operand is 1: {:?}",
        lrep.operands
    );
}

// ── A.6  dict get multi-key + info exists (scalar / stk) ────────────────────

#[test]
fn dict_get_single_and_multi_key() {
    // `dict get $d k` → DICT_GET with key count 1; `dict get $d k1 k2` → key
    // count 2. The multi-key operand is the uncovered arm.
    // tclsh: dict get {a 1 b 2} b  →  2   (8.6 + 9.0)
    let reg = registry();
    let mut one = proc_ctx(&reg, &["d"]);
    one.emit_inline_cmd_subst("[dict get $d k]");
    assert!(ctx_ops(&one).contains(&Op::DICT_GET), "single-key dict get");

    let mut two = proc_ctx(&reg, &["d"]);
    two.emit_inline_cmd_subst("[dict get $d k1 k2]");
    let ops = ctx_ops(&two);
    assert!(ops.contains(&Op::DICT_GET), "multi-key dict get: {ops:?}");
    // Two key pushes + the dict load → the DICT_GET pops 2 keys.
    assert!(
        count(&ops, Op::PUSH1) >= 2,
        "two key literals pushed: {ops:?}"
    );
}

#[test]
fn info_exists_proc_uses_exist_scalar() {
    // `[info exists v]` in a proc with `v` a local → existScalar (+ a NOP pad),
    // not the generic invoke.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["v"]);
    ctx.emit_inline_cmd_subst("[info exists v]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::EXIST_SCALAR),
        "proc info exists → existScalar: {ops:?}"
    );
    assert!(!ops.contains(&Op::EXIST_STK));
}

#[test]
fn info_exists_script_uses_exist_stk() {
    // At script scope (no LVT) the same `info exists` emits push + existStk.
    let reg = registry();
    let mut ctx = CodegenCtx::new(false, &[], &reg);
    ctx.emit_inline_cmd_subst("[info exists v]");
    let ops = ctx_ops(&ctx);
    assert_eq!(
        ops,
        vec![Op::PUSH1, Op::EXIST_STK],
        "script info exists → push; existStk"
    );
}

// ── A.7  array exists (imm) / array names (fqn invoke) ──────────────────────

#[test]
fn array_exists_proc_uses_array_exists_imm() {
    // `array exists arr` in a proc → ARRAY_EXISTS_IMM (an LVT-slot operand).
    // tclsh: array exists undefined  →  0   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["arr"]);
    ctx.emit_inline_cmd_subst("[array exists arr]");
    let ops = ctx_ops(&ctx);
    assert_eq!(
        ops,
        vec![Op::ARRAY_EXISTS_IMM],
        "array exists → arrayExistsImm"
    );
}

#[test]
fn array_names_uses_fqn_invoke() {
    // `array names arr` has no dedicated opcode; it routes through a
    // startCommand-wrapped `::tcl::array::names` invoke.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["arr"]);
    ctx.emit_inline_cmd_subst("[array names arr]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::INVOKE_STK1),
        "array names → invoke: {ops:?}"
    );
    assert!(ops.contains(&Op::START_CMD), "wrapped in a startCommand");
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|l| l == "::tcl::array::names"),
        "FQN array-names target interned"
    );
    assert_all_labels_resolve(ctx);
}

// ── A.8  catch with a ::-qualified result var → generic, not inline ─────────

#[test]
fn catch_with_qualified_result_var_falls_back_to_generic() {
    // `catch {body} ::g` (a qualified result variable) cannot use the inline
    // catch lowering — it routes through the generic command path instead.
    // (emit_inline_cmd_subst's catch arm, the `result_var.starts_with("::")`
    // branch.)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &[]);
    ctx.emit_inline_cmd_subst("[catch {set x 1} ::g]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::INVOKE_STK1),
        "qualified-result catch → generic invoke: {ops:?}"
    );
    assert!(
        !ops.contains(&Op::BEGIN_CATCH4),
        "the qualified-result form must NOT take the inline beginCatch path"
    );
}

// ── A.9  incr ::g <big> — the INCR_STK 1-byte-range guard ───────────────────

#[test]
fn incr_qualified_large_amount_uses_full_incr_stk() {
    // `incr ::g 200` — 200 overflows the 1-byte INCR_STK_IMM operand, so the
    // emitter must push the amount and use the full INCR_STK (the range guard).
    // tclsh: set ::g 0; incr ::g 200  →  200   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = CodegenCtx::new(false, &[], &reg);
    ctx.emit_inline_cmd_subst("[incr ::g 200]");
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::INCR_STK),
        "big amount → INCR_STK: {ops:?}"
    );
    assert!(
        !ops.contains(&Op::INCR_STK_IMM),
        "200 does not fit the 1-byte immediate form"
    );
}

#[test]
fn incr_proc_variable_amount_loads_then_incr_scalar1() {
    // `incr x $y` in a proc: the amount is a variable, so it is loaded then a
    // (non-immediate) INCR_SCALAR1 applies it. (The variable-amount arm.)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["x", "y"]);
    ctx.emit_inline_cmd_subst("[incr x $y]");
    let ops = ctx_ops(&ctx);
    assert_eq!(
        ops,
        vec![Op::LOAD_SCALAR1, Op::INCR_SCALAR1],
        "incr x $y → load y; incrScalar1"
    );
}

// ── A.10  emit_cmd_subst_arg / emit_cmd_word — interpolation & escape arms ──

#[test]
fn cmd_subst_arg_multipart_interpolation_concats() {
    // A composite unbraced arg `x$y` (literal ++ var) decomposes via
    // parse_subst_template into >1 part and concatenates with STR_CONCAT1 — the
    // multi-part early branch of emit_cmd_subst_arg.
    // tclsh: set y 9; list x$y  →  x9   (8.6 + 9.0)
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["y"]);
    ctx.emit_cmd_subst_arg("x$y", false);
    let ops = ctx_ops(&ctx);
    assert_eq!(
        ops,
        vec![Op::PUSH1, Op::LOAD_SCALAR1, Op::STR_CONCAT1],
        "x$y → push \"x\"; load y; strConcat"
    );
}

#[test]
fn cmd_subst_arg_braced_scalar_marker() {
    // The `$={name}` braced-scalar marker → push name; loadStk.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &[]);
    ctx.emit_cmd_subst_arg("$={foo}", false);
    assert_eq!(ctx_ops(&ctx), vec![Op::PUSH1, Op::LOAD_STK]);
    assert!(ctx.literals.entries().iter().any(|l| l == "foo"));
}

#[test]
fn cmd_subst_arg_bare_array_ref_loads_array() {
    // A bare `$arr(idx)` array element reference (not `${...}`-wrapped) is the
    // `split_array_ref` arm → loadArray1 in a proc.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["arr"]);
    ctx.emit_cmd_subst_arg("$arr(idx)", false);
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::LOAD_ARRAY1),
        "bare array ref → loadArray1: {ops:?}"
    );
}

#[test]
fn cmd_subst_arg_braced_with_subst_rewraps() {
    // A braced arg that still holds a `$`/`[` marker is re-wrapped in braces and
    // pushed verbatim (a single PUSH1) — the runtime sees `{$x}`.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["x"]);
    ctx.emit_cmd_subst_arg("$x", true);
    assert_eq!(
        ctx_ops(&ctx),
        vec![Op::PUSH1],
        "braced-with-subst → single push"
    );
    assert!(
        ctx.literals.entries().iter().any(|l| l == "{$x}"),
        "the brace-wrapped literal is interned: {:?}",
        ctx.literals.entries()
    );
}

#[test]
fn cmd_subst_arg_backslash_is_processed_and_pushed() {
    // A plain (no subst) arg containing a backslash escape is backslash-decoded
    // then pushed — the `arg.contains('\\')` arm.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &[]);
    ctx.emit_cmd_subst_arg("a\\tb", false);
    assert_eq!(
        ctx_ops(&ctx),
        vec![Op::PUSH1],
        "backslash arg → single push"
    );
}

#[test]
fn generic_cmd_subst_embedded_array_key_is_substituted() {
    // A command word `be(a:$a)` (an array-element reference with an embedded
    // `$a` in the *key*) must have the `$a` substituted, not pushed raw: the
    // emit_cmd_word interpolated-word arm decomposes it (push "be(a:"; load a;
    // push ")"; strConcat) before the invoke.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["a"]);
    ctx.emit_generic_cmd_subst("foo", &[("be(a:$a)".into(), false)]);
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::STR_CONCAT1),
        "embedded $a in the key is concatenated, not raw: {ops:?}"
    );
    assert!(ops.contains(&Op::LOAD_SCALAR1), "the $a key part is loaded");
    assert!(ops.contains(&Op::INVOKE_STK1));
}

#[test]
fn generic_cmd_subst_leading_dollar_interpolated_word() {
    // A word `$i.x` is a leading-`$` *interpolated* word (not a whole-variable
    // reference), so emit_cmd_word decomposes it: load i; push ".x"; strConcat.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["i"]);
    ctx.emit_generic_cmd_subst("foo", &[("$i.x".into(), false)]);
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::LOAD_SCALAR1), "$i is loaded: {ops:?}");
    assert!(
        ops.contains(&Op::STR_CONCAT1),
        "the .x suffix is concatenated"
    );
}

// ── A.11  empty subst + expanded value-position form ────────────────────────

#[test]
fn empty_cmd_subst_pushes_empty() {
    // `[]` (an empty command substitution) pushes the empty string — the
    // `parts.is_empty()` degenerate arm.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &[]);
    ctx.emit_inline_cmd_subst("[]");
    assert_eq!(ctx_ops(&ctx), vec![Op::PUSH1]);
    assert!(ctx.literals.entries().iter().any(String::is_empty));
}

#[test]
fn expanded_value_position_keeps_braced_word_verbatim() {
    // `[concat {*}$a {b c}]` in value position: the `{*}$a` expands (expandStart
    // / expandStkTop / invokeExpanded) and the braced `{b c}` word is pushed
    // *verbatim* (push_lit_verbatim) without substitution — the `*braced` arm of
    // emit_expanded_cmd_subst. No trailing pop (value position).
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["a"]);
    ctx.emit_inline_cmd_subst("[concat {*}$a {b c}]");
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::EXPAND_START), "expanded form: {ops:?}");
    assert!(ops.contains(&Op::EXPAND_STKTOP));
    assert!(ops.contains(&Op::INVOKE_EXPANDED));
    assert!(
        !ops.contains(&Op::POP),
        "value position leaves the result on TOS"
    );
    assert!(
        ctx.literals.entries().iter().any(|l| l == "b c"),
        "the braced word is interned verbatim: {:?}",
        ctx.literals.entries()
    );
}

// ════════════════════════════════════════════════════════════════════════════
// PART B — codegen/mod.rs : the CodegenCtx driver / errorInfo machinery
//
// These methods (set_source / span_text / span_line / emit-stamping /
// fresh_label / place_label / into_function_asm) are the module-level driver
// state the emitter free functions exercise only indirectly; the existing
// codegen ports never construct the errorInfo surface text directly.
// ════════════════════════════════════════════════════════════════════════════

/// Consume `ctx` into a `FunctionAsm` and assert every `Operand::Label`
/// reference resolves to a recorded label position. `into_function_asm` moves
/// the (crate-private) `label_positions` map into the public `labels` field,
/// so this is the integration-test-visible way to check label resolution —
/// the in-crate `cmd_subst.rs` unit tests inspect `label_positions` directly,
/// which an external test cannot reach.
fn assert_all_labels_resolve(ctx: CodegenCtx) {
    let fa = ctx.into_function_asm("::probe".into());
    for instr in &fa.instructions {
        for op in &instr.operands {
            if let Operand::Label(l) = op {
                assert!(
                    fa.labels.contains_key(l),
                    "unresolved label {l:?} on {:?}",
                    instr.op
                );
            }
        }
    }
}

#[test]
fn emit_stamps_source_cmd_text_with_quoted_word_widen() {
    // `emit` stamps each instruction with the surface text of `current_span`.
    // A command ending in a quoted word has its span end at the word's *inner*
    // end (the segmenter does not widen quoted words), so span_text must include
    // the trailing `"` to quote the whole command for errorInfo.
    let reg = registry();
    let mut ctx = CodegenCtx::new(false, &[], &reg);
    let src = "set x 1\nputs \"hello world\"";
    ctx.set_source(src);
    let start = u32::try_from(src.find("puts").unwrap()).unwrap();
    let inner_end = u32::try_from(src.rfind('"').unwrap()).unwrap(); // points AT the closing quote
    ctx.current_span = Some(Span::new(start, inner_end));
    let idx = ctx.emit(Op::INVOKE_STK1, vec![]);
    let instr = &ctx.instructions[idx];
    assert_eq!(
        instr.source_cmd_text, "puts \"hello world\"",
        "span_text widens to include the closing quote"
    );
    // The command is on the second source line.
    assert_eq!(instr.source_line, 2, "span_line counts the newline");
    assert_eq!(instr.source_span, Some(Span::new(start, inner_end)));
}

#[test]
fn emit_with_no_span_leaves_text_empty_and_line_zero() {
    // With no `current_span` set (the synthetic per-block instruction case),
    // span_text is empty and span_line is 0.
    let reg = registry();
    let mut ctx = CodegenCtx::new(false, &[], &reg);
    ctx.set_source("set x 1");
    // current_span defaults to None.
    let idx = ctx.emit(Op::NOP, vec![]);
    let instr = &ctx.instructions[idx];
    assert_eq!(instr.source_cmd_text, "");
    assert_eq!(instr.source_line, 0);
    assert_eq!(instr.source_span, None);
}

#[test]
fn emit_with_no_source_yields_empty_text() {
    // A hand-built context with a span but no source text (the common test case)
    // yields empty errorInfo text without panicking on the out-of-range slice:
    // `span_text` slices the empty source out of range → "". `span_line`, lacking
    // an `is_empty` short-circuit (unlike the explicit-span `source_line`),
    // counts zero newlines in the empty prefix and so reports the 1-based first
    // line — benign, since the line number is only meaningful with real source
    // present (and the command text is correctly empty regardless).
    let reg = registry();
    let mut ctx = CodegenCtx::new(false, &[], &reg);
    ctx.current_span = Some(Span::new(0, 5));
    let idx = ctx.emit(Op::NOP, vec![]);
    assert_eq!(
        ctx.instructions[idx].source_cmd_text, "",
        "no source ⇒ empty cmd text"
    );
    assert_eq!(
        ctx.instructions[idx].source_line, 1,
        "span_line reports line 1 for an empty source (no is_empty guard)"
    );
}

#[test]
fn emit_comment_stamps_text_line_and_comment() {
    // emit_comment carries the comment AND the same span/line stamping as emit.
    let reg = registry();
    let mut ctx = CodegenCtx::new(false, &[], &reg);
    let src = "set a 1\nset b 2\nincr a";
    ctx.set_source(src);
    let start = u32::try_from(src.rfind("incr").unwrap()).unwrap();
    ctx.current_span = Some(Span::new(start, start + 6)); // "incr a"
    let idx = ctx.emit_comment(Op::INCR_STK_IMM, vec![Operand::Imm(1)], "the comment");
    let instr = &ctx.instructions[idx];
    assert_eq!(instr.comment, "the comment");
    assert_eq!(instr.source_cmd_text, "incr a");
    assert_eq!(instr.source_line, 3, "third source line");
}

#[test]
fn span_text_quoted_widen_only_when_quote_follows() {
    // The trailing-quote widen fires ONLY when a `"` actually sits one byte past
    // the span end; a span ending mid-identifier is not widened.
    let reg = registry();
    let mut ctx = CodegenCtx::new(false, &[], &reg);
    let src = "set name value";
    ctx.set_source(src);
    let start = u32::try_from(src.find("set").unwrap()).unwrap();
    let end = u32::try_from(src.find(" value").unwrap()).unwrap(); // ends right after "set name"
    ctx.current_span = Some(Span::new(start, end));
    let idx = ctx.emit(Op::STORE_STK, vec![]);
    assert_eq!(
        ctx.instructions[idx].source_cmd_text, "set name",
        "no spurious quote when none follows the span"
    );
}

#[test]
fn fresh_label_is_monotonic_and_place_label_records_position() {
    // fresh_label hands out monotonically-numbered names; place_label records
    // the index of the *next* instruction.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &[]);
    let l0 = ctx.fresh_label("blk");
    let l1 = ctx.fresh_label("blk");
    assert_eq!((l0.as_str(), l1.as_str()), ("blk_0", "blk_1"));
    ctx.emit(Op::NOP, vec![]); // index 0
    ctx.place_label(&l1); // points at the next (index 1)
    // `label_positions` is crate-private; inspect it through the public `labels`
    // map that `into_function_asm` moves it into.
    let fa = ctx.into_function_asm("::probe".into());
    assert_eq!(
        fa.labels.get(&l1),
        Some(&1),
        "place_label records the next index"
    );
}

#[test]
fn into_function_asm_carries_name_literals_lvt_and_labels() {
    // into_function_asm consumes the context and produces a FunctionAsm with the
    // accumulated literals / LVT / labels under the supplied name.
    let reg = registry();
    let mut ctx = proc_ctx(&reg, &["p"]);
    let lbl = ctx.fresh_label("end");
    ctx.push_lit("hello");
    ctx.emit(Op::NOP, vec![]);
    ctx.place_label(&lbl);
    let fa = ctx.into_function_asm("::widget".into());
    assert_eq!(fa.name, "::widget");
    assert!(fa.literals.entries().iter().any(|l| l == "hello"));
    assert_eq!(fa.lvt.entries()[0], "p", "the param is in the LVT");
    assert!(
        fa.labels.iter().any(|(k, _)| k == "end_0"),
        "the placed label is carried into the FunctionAsm: {:?}",
        fa.labels
    );
}

#[test]
fn module_dispatch_emits_proc_and_top_level() {
    // The module-level driver lays out the proc body under its qualified name
    // and a `::top` top-level that ends with `done`. (The proc-vs-top-level
    // dispatch arm of codegen_module.)
    let reg = registry();
    let ir = lower_to_ir("proc foo {x} { return $x }\nset z 9", &reg);
    let cfg = build_cfg(&ir, false);
    let m = codegen_module(&cfg, &ir, &reg);
    assert!(
        m.procedures.contains_key("::foo"),
        "proc emitted: {:?}",
        m.procedures.keys()
    );
    assert_eq!(m.top_level.name, "::top");
    assert_eq!(
        m.top_level.instructions.last().unwrap().op,
        Op::DONE,
        "top-level ends with done"
    );
    assert_eq!(
        m.procedures["::foo"].instructions.last().unwrap().op,
        Op::DONE,
        "the proc body ends with done"
    );
}

#[test]
fn module_dispatch_empty_source_is_just_top_level_done() {
    // The empty-module path: a `::top` with `done` and no procedures.
    let reg = registry();
    let ir = lower_to_ir("", &reg);
    let cfg = build_cfg(&ir, false);
    let m = codegen_module(&cfg, &ir, &reg);
    assert!(m.procedures.is_empty(), "no procedures for an empty module");
    assert_eq!(m.top_level.name, "::top");
    assert_eq!(m.top_level.instructions.last().unwrap().op, Op::DONE);
}

// ════════════════════════════════════════════════════════════════════════════
// PART C — var_escape walkers : the uncovered escape-detection arms
//
// Disjoint from var_escape_cfg.rs (which covers conditional upvar, loop /
// switch-arm escapes, the dynamic-set spill, the cfg_result surface, and the
// IR-walk if/for/switch/try arms). The arms below — the non-frameless
// command-substitution head in a value (record_fallback), the direct_callees
// population, the Return / ExprEval value scans, the standalone namespace-upvar
// handler, and the name-first append/lappend literal-vs-dynamic split — are not
// exercised there.
// ════════════════════════════════════════════════════════════════════════════

fn reg_static() -> &'static CommandRegistry {
    registry_for_dialect("tcl8.6")
}

/// CFG/SSA (codegen frame-analysis) summaries, interprocedural on.
fn escape_cu(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let cu = CompilationUnit::build_for(src, reg_static(), false);
    analyse_var_escape_cu(&cu, true)
}

/// IR-walk (inliner path) summaries, interprocedural on.
fn escape_ir(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let module = lower_to_ir(src, reg_static());
    analyse_var_escape(&module, true)
}

fn summary<'a>(m: &'a HashMap<String, ProcEscapeSummary>, q: &str) -> &'a ProcEscapeSummary {
    m.get(q).unwrap_or_else(|| {
        panic!(
            "summary {q} missing; keys = {:?}",
            m.keys().collect::<Vec<_>>()
        )
    })
}

// ── C.1  apply_value_scan: a non-frameless [cmd] head in a value → fallback ──

#[test]
fn cu_value_with_user_cmd_subst_head_records_fallback() {
    // A value `[myproc a]` embeds a command-substitution whose head (`myproc`)
    // is not on the frameless-runtime allow-list, so apply_value_scan's
    // extract_cmd_subst_heads loop records a fallback (the proc may dispatch to
    // the interpreter). The assigned name itself does not escape.
    let s = escape_cu("proc ::p {} { set x [myproc a] }");
    let p = summary(&s, "::p");
    assert!(
        p.has_fallback(),
        "a non-frameless cmd-subst head in a value sets the fallback flag"
    );
    assert_eq!(
        p.tag("x"),
        EscapeTag::Local,
        "the assigned name stays Local"
    );
    assert!(
        !p.dynamic_barrier(),
        "a fallback is not whole-proc pessimism"
    );
}

#[test]
fn cu_value_with_frameless_cmd_subst_head_no_fallback() {
    // The negative control for the head loop: a value whose only embedded subst
    // head is a frameless-runtime command (`string`) does NOT set the fallback.
    let s = escape_cu("proc ::p {} { set x \"len [string length foo]\" }");
    let p = summary(&s, "::p");
    assert!(
        !p.has_fallback(),
        "a frameless subst head must not trip the fallback: {p:?}"
    );
    assert_eq!(p.tag("x"), EscapeTag::Local);
}

#[test]
fn ir_value_with_user_cmd_subst_head_records_fallback() {
    // The IR-walk twin of the above: walker.rs's apply_value_scan head loop also
    // records the fallback for a non-frameless head.
    let s = escape_ir("proc ::p {} { set x [myproc a] }");
    let p = summary(&s, "::p");
    assert!(
        p.has_fallback(),
        "IR-walk records the non-frameless head fallback"
    );
    assert_eq!(p.tag("x"), EscapeTag::Local);
}

// ── C.2  direct_callees population on the CFG path (record_callee) ───────────

#[test]
fn cu_records_statically_resolvable_callees() {
    // handle_call records each bare / ::-qualified command word as a direct
    // callee (for interprocedural propagation). A user proc call and a builtin
    // both land in direct_callees; a dynamic `$cmd` head does not.
    let s = escape_cu("proc ::p {} { set x 1\n helper $x\n other::fn 2 }");
    let p = summary(&s, "::p");
    assert!(
        p.direct_callees.contains("helper"),
        "user call recorded as a direct callee: {:?}",
        p.direct_callees
    );
    assert!(
        p.direct_callees.contains("other::fn"),
        "qualified call recorded: {:?}",
        p.direct_callees
    );
    assert!(
        p.has_call_fallback(),
        "the user calls also set the call-fallback"
    );
}

#[test]
fn cu_dynamic_command_head_not_recorded_as_callee() {
    // A dynamic command head (`$cmd ...`) is NOT a statically-resolvable callee;
    // it sets the (eval-)fallback but contributes no direct_callees entry.
    let s = escape_cu("proc ::p {cmd} { $cmd arg }");
    let p = summary(&s, "::p");
    assert!(
        p.direct_callees.is_empty(),
        "a $cmd head is not a static callee: {:?}",
        p.direct_callees
    );
    assert!(
        p.has_fallback(),
        "but the dynamic head sets the eval fallback"
    );
}

// ── C.3  Return-value / ExprEval scans catch [info exists ...] (IR walk) ─────

#[test]
fn ir_return_value_info_exists_escapes_target() {
    // tclsh: `info exists` resolves its target against the runtime frame, so a
    // `[info exists rv]` in a `return` value names a frame-resident variable:
    //   proc f {} { set rv 1; return [info exists rv] }; f   -> 1
    // The walker.rs `Return { value }` arm runs apply_value_scan over the
    // returned value and finds the hazard.
    let s = escape_ir("proc ::p {} { set rv 1\n return [info exists rv] }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("rv"),
        "info-exists target in a return value is Frame: {p:?}"
    );
    assert!(
        !p.dynamic_barrier(),
        "a literal info-exists is not pessimistic"
    );
}

#[test]
fn ir_expr_eval_info_exists_escapes_target() {
    // A bare `expr {[info exists ev]}` statement (ExprEval) likewise has its
    // expression scanned — the walker.rs `ExprEval { expr }` arm.
    let s = escape_ir("proc ::p {} { expr {[info exists ev]} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("ev"),
        "info-exists in an expr statement is Frame: {p:?}"
    );
}

// ── C.4  standalone namespace upvar handler (handle_namespace_call) ──────────

#[test]
fn cu_standalone_namespace_upvar_escapes_alias_and_records_namespace_callee() {
    // tclsh: `namespace upvar` makes the alias read/write the namespace var:
    //   namespace eval ::ns {variable counter 5}
    //   proc readit {} { namespace upvar ::ns counter c; set c 7; return $c }
    //   readit; set ::ns::counter   -> 7
    // handle_namespace_call routes the `upvar` subcommand to the upvar handler,
    // tagging `c` Frame, and records `namespace` as a callee.
    let s = escape_cu("proc ::p {} { namespace upvar ::ns counter c\n set c 1 }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("c"), "namespace-upvar alias is Frame");
    assert!(p.frame_needed);
    assert!(
        !p.dynamic_barrier(),
        "a literal namespace upvar is not pessimistic"
    );
    assert!(
        p.direct_callees.contains("namespace"),
        "the namespace command is recorded as a callee: {:?}",
        p.direct_callees
    );
}

#[test]
fn ir_standalone_namespace_upvar_escapes_alias() {
    // The IR-walk twin: walker.rs routes `namespace upvar` through the same
    // handler.
    let s = escape_ir("proc ::p {} { namespace upvar ::ns counter c\n set c 1 }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("c"), "IR-walk namespace-upvar alias is Frame");
    assert!(p.frame_needed);
}

// ── C.5  name-first append/lappend: literal (no escape) vs dynamic (barrier) ─

#[test]
fn cu_append_literal_name_does_not_escape() {
    // `append a x` is a name-first command with a *literal* name; the
    // handle_dynamic_name_first literal arm does not escape `a` (a plain local
    // append stays in the local). This is the negative half of the name-first
    // split.
    let s = escape_cu("proc ::p {} { append a x }");
    let p = summary(&s, "::p");
    assert_eq!(
        p.tag("a"),
        EscapeTag::Local,
        "literal-name append stays Local"
    );
    assert!(!p.dynamic_barrier());
    assert!(
        p.direct_callees.contains("append"),
        "append is still recorded as a callee: {:?}",
        p.direct_callees
    );
}

#[test]
fn cu_lappend_dynamic_name_is_pessimistic() {
    // tclsh: `lappend $n v` appends to whichever variable `$n` names — an
    // optimiser must treat the whole frame as live:
    //   proc f {n} { set a {}; lappend $n 1; return $a }
    //   f a -> 1 ;  f other -> "" (a untouched)
    // The dynamic name defeats the name-first analysis, so the proc goes
    // pessimistic (the handle_dynamic_name_first dynamic arm).
    let s = escape_cu("proc ::p {n} { lappend $n 1 }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "a dynamic lappend name is whole-proc pessimistic"
    );
    assert!(p.frame_needed);
}

// ── C.6  uplevel #0 literal body via the CFG UpFrame path (not a barrier) ────

#[test]
fn cu_uplevel_zero_literal_body_needs_fallback_only() {
    // tclsh: `uplevel #0 {set glob 1}` runs at global scope where our proc-
    // locals are not visible, so a literal-body `uplevel #0` is NOT a whole-proc
    // barrier — it only needs the eval fallback. (handle_uplevel #0 arm, reached
    // through the CFG UpFrame / barrier path.)
    let s = escape_cu("proc ::p {} { uplevel #0 {set glob 1} }");
    let p = summary(&s, "::p");
    assert!(
        !p.dynamic_barrier(),
        "uplevel #0 with a literal body is not pessimistic"
    );
    assert!(p.has_fallback(), "but it needs the eval fallback");
}
