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
use tcl_compiler::lowering::lower_to_ir;
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

/// Compile and run `src`; return its result string.
fn run(src: &str) -> String {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
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
