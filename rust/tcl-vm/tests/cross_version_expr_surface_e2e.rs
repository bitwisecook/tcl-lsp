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

//! **The `expr` operator set follows the emulated release, however the
//! expression reaches the engine.**
//!
//! `expr` has two entry points and they used to apply opposite dialect
//! discipline (issue #1435). A braced body codegen could specialise compiled to
//! an opcode — `Op::from_binop` is total over `BinOp`, so `**`, `in`/`ni` and
//! the TIP-461 string-ordering operators always got one — and the emulating VM
//! executed it whatever release it was pretending to be. The same source
//! reaching the interpreted `exprStk` path was validated against the release's
//! own operator table first and rejected.
//!
//! That split is invisible until the two forms are compared: both return a
//! well-typed value, one of them from an operator the release cannot parse. The
//! vectors below therefore assert two things — the per-release answer, and that
//! the compiled and interpreted spellings of the same expression agree.
//!
//! Introduction releases, verified against the C sources in `tmp/`:
//!
//! | operator | first release | evidence |
//! |---|---|---|
//! | `eq`, `ne` | 8.4 | `tcl8.4.20/generic/tclCompExpr.c:133-134` (`operatorTable`) |
//! | `**` | 8.5 | `tcl8.5.19/generic/tclCompExpr.c:260,416` (`EXPON`, TIP 123); 8.4's table has no `**` and no `EXPON` |
//! | `in`, `ni` | 8.5 | `tcl8.5.19/generic/tclCompExpr.c:264-265,417-418,1896,1917` (`IN_LIST`/`NOT_IN_LIST`, TIP 201) |
//! | `lt`, `le`, `gt`, `ge` | 9.0 | `tcl9.0.4/generic/tclCompExpr.c:273,413,2053` (`STR_LT`, TIP 461); `STR_LT` appears nowhere in 8.6's |
//!
//! The registry's own tables say the same thing and are what the engines read:
//! `BinOp::expr_grammar_min_version` (`rust/tcl-syntax/src/expr/operators.rs`)
//! and `tcl_dialect::EXPR_WORD_OPERATORS`.
//!
//! A vector that expects a rejection prints a fixed word rather than the
//! interpreter's message: the claim under test is *which release* rejects the
//! operator, and the message wording is already pinned by
//! `tcl-vm/tests/expr_surface_e2e.rs` and the registry's own surface tests.

use std::cell::RefCell;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode_with_dialect as lower_to_ir;
use tcl_dialect::TclVersion;
use tcl_registry::CommandRegistry;
use tcl_vm::{CompileError, CompileService, Vm};

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct CompilerSvc {
    registry: CommandRegistry,
    /// The compile target for any script the VM compiles at run time (`eval`,
    /// a proc body): the same release it is emulating.
    dialect: &'static str,
}

impl CompileService for CompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
        if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
            return Err(CompileError(msg));
        }
        let ir = lower_to_ir(
            src,
            &self.registry,
            tcl_lexer::LexerConfig::default(),
            self.dialect,
        );
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.registry))
    }
}

/// Run `src` in the VM at `version`, returning its `puts` output.
///
/// Compiles *for* the release and then runs at it, exactly as `tclvm` does —
/// the operator gate is a codegen decision, so a dialect-blind compile bakes in
/// the newest grammar's answer and `set_runtime_version` cannot undo it.
fn vm_output(src: &str, version: TclVersion) -> String {
    let dialect = version.dialect_name();
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry, tcl_lexer::LexerConfig::default(), dialect);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);

    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_compiler(Box::new(CompilerSvc {
        registry: CommandRegistry::build_default(),
        dialect,
    }));
    vm.set_runtime_version(version);
    let _ = vm.run_module(&asm);
    String::from_utf8_lossy(&cap.0.borrow()).trim().to_string()
}

/// One operator and what each release must make of it.
struct Vector {
    name: &'static str,
    /// The expression body, as it appears inside `expr { … }`.
    expr: &'static str,
    /// Tcl 8.4: `eq`/`ne` only.
    want_84: &'static str,
    /// Tcl 8.5 / 8.6: adds `**` (TIP 123) and `in`/`ni` (TIP 201).
    want_8x: &'static str,
    /// Tcl 9.0 / 9.1: adds the string-ordering operators (TIP 461).
    want_90: &'static str,
}

/// What a release that lacks the operator must print.
const REJECTED: &str = "rejected";

const VECTORS: &[Vector] = &[
    Vector {
        name: "eq is in 8.4's operator table",
        expr: "\"a\" eq \"a\"",
        want_84: "1",
        want_8x: "1",
        want_90: "1",
    },
    Vector {
        name: "ne is in 8.4's operator table",
        expr: "\"a\" ne \"a\"",
        want_84: "0",
        want_8x: "0",
        want_90: "0",
    },
    Vector {
        name: "** arrives in 8.5 (TIP 123)",
        expr: "2 ** 3",
        want_84: REJECTED,
        want_8x: "8",
        want_90: "8",
    },
    Vector {
        name: "in arrives in 8.5 (TIP 201)",
        expr: "\"b\" in {a b c}",
        want_84: REJECTED,
        want_8x: "1",
        want_90: "1",
    },
    Vector {
        name: "ni arrives in 8.5 (TIP 201)",
        expr: "\"b\" ni {a b c}",
        want_84: REJECTED,
        want_8x: "0",
        want_90: "0",
    },
    Vector {
        name: "lt arrives in 9.0 (TIP 461)",
        expr: "\"a\" lt \"b\"",
        want_84: REJECTED,
        want_8x: REJECTED,
        want_90: "1",
    },
    Vector {
        name: "le arrives in 9.0 (TIP 461)",
        expr: "\"b\" le \"b\"",
        want_84: REJECTED,
        want_8x: REJECTED,
        want_90: "1",
    },
    Vector {
        name: "gt arrives in 9.0 (TIP 461)",
        expr: "\"a\" gt \"b\"",
        want_84: REJECTED,
        want_8x: REJECTED,
        want_90: "0",
    },
    Vector {
        name: "ge arrives in 9.0 (TIP 461)",
        expr: "\"a\" ge \"b\"",
        want_84: REJECTED,
        want_8x: REJECTED,
        want_90: "0",
    },
];

/// One of the code positions a braced `expr` body reaches codegen through,
/// each with its own re-parse and emission site.
struct Position {
    name: &'static str,
    /// The script, with `@expr@` standing in for the expression body. It
    /// prints the expression's value, or [`REJECTED`] when the release's
    /// `expr` will not take the operator.
    template: &'static str,
    /// Whether the script prints the expression's *truth value* rather than
    /// its value — `if {2 ** 3}` takes 8.5's answer `8` as true.
    boolean: bool,
}

const POSITIONS: &[Position] = &[
    // Value position — `codegen::cmd_subst`'s `[expr {…}]` hook.
    Position {
        name: "value",
        template: "if {[catch {expr {@expr@}} r]} { puts rejected } else { puts $r }\n",
        boolean: false,
    },
    // Catch-body position — `codegen::control_flow`'s catch-body `expr` hook.
    Position {
        name: "catch body",
        template: "set c [catch {expr {@expr@}} r]\nif {$c} { puts rejected } else { puts $r }\n",
        boolean: false,
    },
    // Statement position — the `expr` lowering hook, emitted by
    // `codegen::statements`.
    Position {
        name: "statement",
        template: "if {[catch {set v [expr {@expr@}]} r]} { puts rejected } else { puts $v }\n",
        boolean: false,
    },
    // Condition position — the node the *lowerer* parses, emitted by
    // `codegen::emitter::terminator`.
    Position {
        name: "condition",
        template: "if {[catch {if {@expr@} { set v 1 } else { set v 0 }} r]} { puts rejected } else { puts $v }\n",
        boolean: true,
    },
];

/// The interpreted spelling: the same expression as a *value*, which cannot be
/// specialised at compile time and so goes through `exprStk` and
/// `Vm::eval_expr`.
const INTERPRETED: &str =
    "set e {@expr@}\nif {[catch {expr $e} r]} { puts rejected } else { puts $r }\n";

fn script(template: &str, expr: &str) -> String {
    template.replace("@expr@", expr)
}

/// A value as a [`Position::boolean`] script prints it. Every vector's value is
/// already `0`/`1` or a positive integer, so this is exact rather than a
/// re-implementation of Tcl's truth rule.
fn as_boolean(value: &str) -> &str {
    match value {
        REJECTED | "0" => value,
        _ => "1",
    }
}

#[test]
fn expr_operator_set_follows_the_compiled_release() {
    for v in VECTORS {
        for position in POSITIONS {
            let src = script(position.template, v.expr);
            for (version, value) in [
                (TclVersion::V8_4, v.want_84),
                (TclVersion::V8_5, v.want_8x),
                (TclVersion::V8_6, v.want_8x),
                (TclVersion::V9_0, v.want_90),
                (TclVersion::V9_1, v.want_90),
            ] {
                let want = if position.boolean {
                    as_boolean(value)
                } else {
                    value
                };
                assert_eq!(
                    vm_output(&src, version),
                    want,
                    "[{version:?}] {} ({} position)",
                    v.name,
                    position.name
                );
            }
        }
    }
}

/// The core of issue #1435: a source-identical `expr` must not get two
/// different release disciplines depending only on whether codegen could inline
/// it. The interpreted path has always validated against the release's operator
/// table; this asserts the compiled one now reaches the same verdict.
#[test]
fn compiled_and_interpreted_expr_agree_on_every_release() {
    for v in VECTORS {
        let interpreted = script(INTERPRETED, v.expr);
        for position in POSITIONS {
            let compiled = script(position.template, v.expr);
            for version in [
                TclVersion::V8_4,
                TclVersion::V8_5,
                TclVersion::V8_6,
                TclVersion::V9_0,
                TclVersion::V9_1,
            ] {
                let evaluated = vm_output(&interpreted, version);
                let want = if position.boolean {
                    as_boolean(&evaluated)
                } else {
                    &evaluated
                };
                assert_eq!(
                    vm_output(&compiled, version),
                    want,
                    "[{version:?}] {} ({} position vs interpreted)",
                    v.name,
                    position.name
                );
            }
        }
    }
}

/// A constant-folded operator is the sharpest form of the bug: `expr {2 ** 3}`
/// compiled for 8.4 collapsed to a push of `8` before any opcode was chosen, so
/// the release rejected the expression while the compiled script still printed
/// an answer.
#[test]
fn a_pre_floor_operator_is_not_constant_folded() {
    let src = "if {[catch {expr {2 ** 3}} r]} { puts rejected } else { puts $r }\n";
    assert_eq!(vm_output(src, TclVersion::V8_4), REJECTED);
    assert_eq!(vm_output(src, TclVersion::V8_5), "8");
}
