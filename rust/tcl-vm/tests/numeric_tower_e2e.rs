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
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Oracle-pinned coverage for the r3-numeric-tower lane (#1428, #1382,
//! #1432, #1581) on the bytecode VM.
//!
//! Every expectation was read verbatim out of `tclsh9.0` (9.0.4) — and, where
//! the releases agree, `tclsh8.6` (8.6.16) — with the sheet
//!
//! ```tcl
//! catch {expr {...}} m o; list $m [dict get $o -errorcode]
//! ```
//!
//! The matching pins for the WASM runtime live in
//! `runtime/rust/tests/numeric_tower_semantics.rs`; both files assert the
//! same rows, so the two engines cannot drift apart on them.
//!
//! Each expression is checked in **both** forms: braced (`expr {…}`, which
//! the compiler may const-fold through `tcl_expr_eval`) and dynamic
//! (`set e {…}; expr $e`, which always reaches `tcl_vm::expr` at run time),
//! so a fix in one evaluator cannot mask a gap in the other.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::{lower_to_ir, lower_to_ir_with_dialect};
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

struct CompilerSvc {
    registry: CommandRegistry,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(src, &self.registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }

    /// A VM pinned to a release compiles its run-time scripts for that
    /// release's profile too, so `run_at` can evaluate `expr $e` at 8.6.
    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error_with_config(src, config)
        {
            return Err(CompileError(msg));
        }
        let ir = tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect(
            src,
            registry,
            config,
            Some(profile),
        );
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry))
    }
}

#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Compile and run `src` under the VM's default (Tcl 9.0) release.
fn run(src: &str) -> String {
    run_at(src, tcl_dialect::TclVersion::V9_0)
}

/// Compile and run `src` with the VM pinned to `version`.
///
/// Compiled *for* the release and then run at it, exactly as `tclvm` does — a
/// dialect-blind compile bakes in the newest grammar and the VM refuses to run
/// it under a pinned older release.
fn run_at(src: &str, version: tcl_dialect::TclVersion) -> String {
    let dialect = version.dialect_name();
    let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let ir = lower_to_ir_with_dialect(
        src,
        registry,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        Some(profile),
    );
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, registry);
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_runtime_version(version);
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
    }));
    let completion = vm.run_module(&asm);
    assert!(
        completion.code.is_ok(),
        "harness script failed: {}",
        completion.result.to_str()
    );
    completion.result.to_str().to_string()
}

/// `catch {expr …} m o` around `body`, reported as the Tcl list
/// `ok VALUE` or `err MESSAGE ERRORCODE` — exactly what a script sees.
fn probe(body: &str) -> String {
    run(&format!(
        "if {{[catch {{expr {{{body}}}}} m o]}} {{ list err $m [dict get $o -errorcode] }} else {{ list ok $m }}"
    ))
}

/// The same probe over the *dynamic* `expr $e` form, which never const-folds
/// and therefore always runs `tcl_vm::expr`.
fn probe_dynamic(body: &str) -> String {
    run(&format!(
        "set e {{{body}}}\nif {{[catch {{expr $e}} m o]}} {{ list err $m [dict get $o -errorcode] }} else {{ list ok $m }}"
    ))
}

/// Assert both evaluation forms report exactly `want` (a Tcl list, as built
/// by [`probe`]).
#[track_caller]
fn both(body: &str, want: &str) {
    assert_eq!(probe(body), want, "expr {{{body}}} (braced/const-folded)");
    assert_eq!(probe_dynamic(body), want, "expr $e where e = {body}");
}

// ---------------------------------------------------------------------------
// #1428 — `0 ** -1` is C's domain error, not a division by zero; the two
// refusals `number_tower::int_pow` merges are told apart by the adopter, and
// both engines stamp the `-errorcode` C stamps.
// ---------------------------------------------------------------------------

#[test]
fn zero_to_a_negative_power_is_a_domain_error_not_divide_by_zero() {
    const WANT: &str = "err {exponentiation of zero by negative power} {ARITH DOMAIN {exponentiation of zero by negative power}}";
    for body in [
        "0 ** -1",
        "0 ** -2",
        "0 ** -100000000000000000000",
        "0.0 ** -1",
        "0 ** -1.0",
        "0.0 ** -1.0",
    ] {
        both(body, WANT);
    }
}

#[test]
fn divide_by_zero_keeps_its_own_class() {
    const WANT: &str = "err {divide by zero} {ARITH DIVZERO {divide by zero}}";
    for body in ["1 / 0", "1 % 0", "(2**70) / 0", "(2**70) % 0"] {
        both(body, WANT);
    }
}

#[test]
fn the_exponent_ceiling_refuses_instead_of_allocating() {
    for body in ["2 ** 268435456", "3 ** 268435456", "(2**70) ** 268435456"] {
        both(body, "err {exponent too large} NONE");
    }
}

#[test]
fn the_collapsing_bases_still_answer_at_any_exponent() {
    both("0 ** 268435456", "ok 0");
    both("1 ** 268435456", "ok 1");
    both("-1 ** 268435456", "ok 1");
    both("(-1) ** 268435457", "ok -1");
    both("(-1) ** 100000000000000000001", "ok -1");
    both("(-1) ** 100000000000000000000", "ok 1");
    both("2 ** -100000000000000000000", "ok 0");
    both("7 ** -3", "ok 0");
    both("0 ** 0", "ok 1");
    both("0.0 ** 0", "ok 1.0");
}

#[test]
fn floor_division_and_modulus_match_the_oracle() {
    both("-7 / 2", "ok -4");
    both("-7 % 2", "ok 1");
    both("7 % -2", "ok -1");
    both("(2**64) % 7", "ok 2");
    both("(2**64) / -3", "ok -6148914691236517206");
    both("(-9223372036854775807 - 1) / -1", "ok 9223372036854775808");
}

#[test]
fn shift_edges_match_the_oracle() {
    both("2 >> 100000000000000000000", "ok 0");
    both("-2 >> 100000000000000000000", "ok -1");
    both("-1 >> 100000000000000000000", "ok -1");
    both("0 >> 100000000000000000000", "ok 0");
    both("-8 >> 1", "ok -4");
    both("1 << 100", "ok 1267650600228229401496703205376");
    both("0 << 100000000000000000000", "ok 0");
    both("0 << 2147483648", "ok 0");
    both("1 << -1", "err {negative shift argument} NONE");
    both("1 >> -1", "err {negative shift argument} NONE");
    both(
        "1 << 100000000000000000000",
        "err {integer value too large to represent} NONE",
    );
    both(
        "1 << 2147483648",
        "err {integer value too large to represent} NONE",
    );
}

// ---------------------------------------------------------------------------
// #1382 — `entier`/`int`/`wide`/`round` on a float outside the wide range,
// and `int()`'s release axis.
// ---------------------------------------------------------------------------

/// Assert `expr {body}` evaluates to `want` at `version`, in both the braced
/// (const-foldable) and the dynamic form.
#[track_caller]
fn both_at(body: &str, want: &str, version: tcl_dialect::TclVersion) {
    let want = format!("ok {want}");
    let probe = format!(
        "if {{[catch {{expr {{{body}}}}} m o]}} {{ list err $m [dict get $o -errorcode] }} else {{ list ok $m }}"
    );
    assert_eq!(
        run_at(&probe, version),
        want,
        "expr {{{body}}} at {version:?}"
    );
    let dynamic = format!(
        "set e {{{body}}}\nif {{[catch {{expr $e}} m o]}} {{ list err $m [dict get $o -errorcode] }} else {{ list ok $m }}"
    );
    assert_eq!(
        run_at(&dynamic, version),
        want,
        "expr $e ({body}) at {version:?}"
    );
}

/// `round()` is half away from zero on the *exact* operand — not
/// `floor(d + 0.5)`, which rounds twice (tclsh 8.6.16/9.0.4:
/// `round(0.49999999999999994)` is `0` while `floor(0.49999999999999994+0.5)`
/// is `1.0`). `both` runs the braced form too, so these rows also pin the
/// const-folder's answer.
#[test]
fn entier_and_round_of_a_beyond_wide_float_are_exact() {
    both("entier(1e300)", &format!("ok {E1E300}"));
    both("entier(1e20)", "ok 100000000000000000000");
    both("entier(-1e20)", "ok -100000000000000000000");
    both("entier(1.9)", "ok 1");
    both("entier(-1.9)", "ok -1");
    both("entier(2**64+1)", "ok 18446744073709551617");
    both("round(1e300)", &format!("ok {E1E300}"));
    both("round(1e20)", "ok 100000000000000000000");
    both("round(0.5)", "ok 1");
    both("round(1.5)", "ok 2");
    both("round(2.5)", "ok 3");
    both("round(-2.5)", "ok -3");
    both("round(0.49999999999999994)", "ok 0");
    both("round(-0.49999999999999994)", "ok 0");
    both("round(4503599627370497.0)", "ok 4503599627370497");
    both("round(-4503599627370497.0)", "ok -4503599627370497");
    both("round(4503599627370497.5)", "ok 4503599627370498");
    both("round(2251799813685248.5)", "ok 2251799813685249");
}

#[test]
fn wide_windows_the_exact_truncation() {
    both("wide(1e20)", "ok 7766279631452241920");
    both("wide(-1e20)", "ok -7766279631452241920");
    both("wide(1e19)", "ok -8446744073709551616");
    both("wide(1e300)", "ok 0");
    both("wide(2**64+1)", "ok 1");
}

#[test]
fn isqrt_of_a_beyond_wide_float_is_exact() {
    both("isqrt(1e300)", &format!("ok {ISQRT_1E300}"));
}

/// tclsh9.0.4 binds `int` to the unbounded `ExprIntFunc`; tclsh8.6.16 keeps
/// its signed-64-bit window. Both engines select through the shared
/// `IntWidth` axis, so they cannot disagree about which release windows.
#[test]
fn int_follows_the_vms_release() {
    use tcl_dialect::TclVersion::{V8_6, V9_0};
    both_at("int(1e20)", "100000000000000000000", V9_0);
    both_at("int(-1e20)", "-100000000000000000000", V9_0);
    both_at("int(2**64+1)", "18446744073709551617", V9_0);
    both_at("int(1e300)", E1E300, V9_0);

    both_at("int(1e20)", "7766279631452241920", V8_6);
    both_at("int(-1e20)", "-7766279631452241920", V8_6);
    both_at("int(2**64+1)", "1", V8_6);
    both_at("int(1e300)", "0", V8_6);

    // `wide` and `entier` are release-invariant.
    for v in [V8_6, V9_0] {
        both_at("wide(1e20)", "7766279631452241920", v);
        both_at("entier(1e20)", "100000000000000000000", v);
    }
}

/// The exact integer value of the double `1e300` (301 digits) — tclsh
/// `expr {entier(1e300)}`.
const E1E300: &str = "1000000000000000052504760255204420248704468581108159154915854115511802457988908195786371375080447864043704443832883878176942523235360430575644792184786706982848387200926575803737830233794788090059368953234970799945081119038967640880074652742780142494579258788820056842838115669472196386865459400540160";

/// tclsh `expr {isqrt(1e300)}` (151 digits).
const ISQRT_1E300: &str = "1000000000000000026252380127602209779758503108492371458359424883684651414333812736380124287612629691547944630047071980611862607399628869272326975124240";

// ---------------------------------------------------------------------------
// #1432 — `rand`/`srand` through the shared generator.
// ---------------------------------------------------------------------------

/// The VM used to divide by `RAND_IM` where C multiplies by `1.0/RAND_IM`,
/// giving `0.0019644186841158285` for `srand(251)` where tclsh (and the WASM
/// runtime) say `0.001964418684115828`. Both engines now call the one shared
/// generator.
#[test]
fn srand_reproduces_cs_reciprocal_multiply_scaling() {
    both("srand(251)", "ok 0.001964418684115828");
    both("srand(1)", "ok 7.826369259425611e-6");
    both("srand(0)", "ok 0.24257829889775176");
    both("srand(2147483647)", "ok 0.7574217011022483");
    both("srand(-1)", "ok 0.7574217011022483");
    both("srand(2**64+7)", "ok 5.4784584815979276e-5");
}

/// The 145th draw of `srand(1)`'s stream — the first index at which the two
/// scalings disagreed. tclsh 8.6.16/9.0.4: `0.9833050970841688`.
#[test]
fn the_145th_draw_after_srand_1_matches_the_oracle() {
    let out = run(
        "expr {srand(1)}\nset v {}\nfor {set i 2} {$i <= 145} {incr i} { set v [expr {rand()}] }\nset v",
    );
    assert_eq!(out, "0.9833050970841688");
}

/// C refuses a double `srand` operand rather than truncating it; the VM used
/// to truncate (`srand(1.5)` seeded 1) while the runtime errored.
#[test]
fn srand_refuses_a_non_integer_operand() {
    both(
        "srand(1.5)",
        "err {expected integer but got \"1.5\"} {TCL VALUE INTEGER}",
    );
    both(
        "srand(\"abc\")",
        "err {expected integer but got \"abc\"} {TCL VALUE NUMBER}",
    );
}

/// Like [`both_at`] but taking the complete probe result (`ok …` / `err …`),
/// for rows that assert an error rather than a value.
#[track_caller]
fn both_at_raw(body: &str, want: &str, version: tcl_dialect::TclVersion) {
    let probe = format!(
        "if {{[catch {{expr {{{body}}}}} m o]}} {{ list err $m [dict get $o -errorcode] }} else {{ list ok $m }}"
    );
    assert_eq!(
        run_at(&probe, version),
        want,
        "expr {{{body}}} at {version:?}"
    );
    let dynamic = format!(
        "set e {{{body}}}\nif {{[catch {{expr $e}} m o]}} {{ list err $m [dict get $o -errorcode] }} else {{ list ok $m }}"
    );
    assert_eq!(
        run_at(&dynamic, version),
        want,
        "expr $e ({body}) at {version:?}"
    );
}

// ---------------------------------------------------------------------------
// #1581 — the expr/mathfunc error taxonomy on the VM.
// ---------------------------------------------------------------------------

#[test]
fn an_infinity_in_an_integer_conversion_is_ioverflow() {
    const WANT: &str = "err {integer value too large to represent} {ARITH IOVERFLOW {integer value too large to represent}}";
    for body in [
        "entier(Inf)",
        "int(Inf)",
        "wide(Inf)",
        "round(Inf)",
        "entier(-Inf)",
        "isqrt(Inf)",
    ] {
        both(body, WANT);
    }
    both("abs(-Inf)", "ok Inf");
    both("double(Inf)", "ok Inf");
    both("floor(Inf)", "ok Inf");
}

#[test]
fn a_nan_operand_carries_the_double_nan_code() {
    const WANT: &str = "err {floating point value is Not a Number} {TCL VALUE DOUBLE NAN}";
    for body in [
        "entier(NaN)",
        "int(NaN)",
        "round(NaN)",
        "abs(NaN)",
        "double(NaN)",
        "ceil(NaN)",
        "isqrt(NaN)",
    ] {
        both(body, WANT);
    }
}

#[test]
fn domain_errors_keep_their_own_class() {
    both(
        "sqrt(-1)",
        "err {domain error: argument not in valid range} {ARITH DOMAIN {domain error: argument not in valid range}}",
    );
    both(
        "isqrt(-1)",
        "err {square root of negative argument} {ARITH DOMAIN {domain error: argument not in valid range}}",
    );
}

#[test]
fn boolean_context_errors_carry_their_codes() {
    for body in [
        "\"abc\" ? 1 : 2",
        "\"abc\" && 1",
        "1 && \"abc\"",
        "\"abc\" || 0",
    ] {
        both(
            body,
            "err {expected boolean value but got \"abc\"} {TCL VALUE NUMBER}",
        );
    }
    both(
        "\"NaN\" ? 1 : 2",
        "err {floating point value is Not a Number} {TCL VALUE DOUBLE NAN}",
    );
}

/// The `IllegalExprOperandType` release axis on the VM: 9.0 names the value
/// and the side and has a list branch; 8.6 names neither and has no list
/// branch. The `-errorcode` is invariant. Every row measured on tclsh9.0.4
/// and tclsh8.6.16.
#[test]
fn operand_type_errors_follow_the_release_wording() {
    use tcl_dialect::TclVersion::{V8_6, V9_0};
    let rows: &[(&str, &str, &str, &str)] = &[
        (
            "!\"abc\"",
            "cannot use non-numeric string \"abc\" as operand of \"!\"",
            "can't use non-numeric string as operand of \"!\"",
            "{non-numeric string}",
        ),
        (
            "~1.5",
            "cannot use floating-point value \"1.5\" as operand of \"~\"",
            "can't use floating-point value as operand of \"~\"",
            "{floating-point value}",
        ),
        (
            "\"abc\" + 1",
            "cannot use non-numeric string \"abc\" as left operand of \"+\"",
            "can't use non-numeric string as operand of \"+\"",
            "{non-numeric string}",
        ),
        (
            "1 + \"abc\"",
            "cannot use non-numeric string \"abc\" as right operand of \"+\"",
            "can't use non-numeric string as operand of \"+\"",
            "{non-numeric string}",
        ),
        (
            "NaN + 1",
            "cannot use non-numeric floating-point value \"NaN\" as left operand of \"+\"",
            "can't use non-numeric floating-point value as operand of \"+\"",
            "{non-numeric floating-point value}",
        ),
    ];
    for (body, want90, want86, code) in rows {
        both_at_raw(
            body,
            &format!("err {{{want90}}} {{ARITH DOMAIN {code}}}"),
            V9_0,
        );
        both_at_raw(
            body,
            &format!("err {{{want86}}} {{ARITH DOMAIN {code}}}"),
            V8_6,
        );
    }
    // The list branch exists only at 9.0; 8.6 reports a non-numeric string.
    both_at_raw(
        "\"a b\" + 1",
        "err {cannot use a list as left operand of \"+\"} {ARITH DOMAIN list}",
        V9_0,
    );
    both_at_raw(
        "\"a b\" + 1",
        "err {can't use non-numeric string as operand of \"+\"} {ARITH DOMAIN {non-numeric string}}",
        V8_6,
    );
}
