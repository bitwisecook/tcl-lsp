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

//! Main emitter loop and public API.
//!
//! Split across multiple files by responsibility:
//!
//! - [`ordering`]  — CFG linearisation and loop body detection
//! - [`terminator`] — CFG terminator emission (goto/branch/return)
//! - [`proc_defs`] — interleaved proc definition emission
//! - [`loop_blocks`] — per-block handlers for foreach/while/for
//! - [`try_blocks`] — try/finally CFG pattern detection
//! - [`generate`] — top-level dispatcher
//! - [`bytecoded`] — registry-backed codegen hook dispatch

#![allow(dead_code)]

pub mod bytecoded;
pub mod generate;
pub mod loop_blocks;
pub mod ordering;
pub mod proc_defs;
pub mod terminator;
pub mod try_blocks;

use std::collections::HashMap;

use tcl_registry::CommandRegistry;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::ir::{Module as IrModule, Procedure as IrProcedure};

use super::{CodegenCtx, FunctionAsm, ModuleAsm};

/// Generate bytecode assembly for a single CFG function.
///
/// When `is_proc` is true, variables are accessed via the LVT
/// (`loadScalar1`/`storeScalar1`). When `false` (top-level scripts),
/// variables are accessed via the stack (`loadStk`/`storeStk`).
/// `registry` is consulted for codegen-hook resolution; pass the
/// same instance the lowering pass used so dialect-loaded specs
/// are visible.
#[must_use]
pub fn codegen_function(
    cfg: &CfgFunction,
    params: &[&str],
    is_proc: bool,
    registry: &CommandRegistry,
) -> FunctionAsm {
    codegen_function_with_procs(cfg, params, is_proc, &[], registry)
}

/// Generate bytecode assembly for a CFG function, with pending proc defs.
///
/// Used by `codegen_module` to interleave proc definitions at their
/// source positions within the top-level script.
#[must_use]
pub fn codegen_function_with_procs(
    cfg: &CfgFunction,
    params: &[&str],
    is_proc: bool,
    proc_defs: &[IrProcedure],
    registry: &CommandRegistry,
) -> FunctionAsm {
    let mut ctx = CodegenCtx::new(is_proc, params, registry);
    generate::generate(&mut ctx, cfg, proc_defs)
}

/// Like [`codegen_function_with_procs`] but threading the module source text so
/// each instruction carries its command's surface text for `errorInfo`, plus
/// the proc body's `base_line` (its `proc` definition line) so the
/// `(procedure … line N)` frame reports a proc-relative line.
#[must_use]
/// The per-module facts every function emission shares: the registry, the
/// module source (for `errorInfo` surface text) and the release being compiled
/// for (its numeral and backslash-escape grammars). Bundled rather than
/// threaded as parallel parameters — the argument list is already at
/// `clippy::too_many_arguments`'s ceiling, and these always travel together.
#[derive(Clone, Copy)]
struct ModuleEmit<'a> {
    registry: &'a CommandRegistry,
    source: &'a str,
    numbers: tcl_dialect::NumberSyntax,
    escapes: tcl_dialect::EscapeSyntax,
}

fn codegen_function_src(
    cfg: &CfgFunction,
    params: &[&str],
    is_proc: bool,
    proc_defs: &[IrProcedure],
    module: ModuleEmit<'_>,
    base_line: u32,
) -> FunctionAsm {
    let mut ctx = CodegenCtx::new(is_proc, params, module.registry);
    ctx.numbers = module.numbers;
    ctx.escapes = module.escapes;
    ctx.set_source(module.source);
    let mut asm = generate::generate(&mut ctx, cfg, proc_defs);
    asm.body_base_line = base_line;
    asm
}

/// The 1-based line of byte offset `off` within `source`.
fn line_of(source: &str, off: u32) -> u32 {
    let end = (off as usize).min(source.len());
    1 + u32::try_from(
        source
            .get(..end)
            .unwrap_or("")
            .bytes()
            .filter(|&b| b == b'\n')
            .count(),
    )
    .unwrap_or(0)
}

/// Generate bytecode assembly for an entire module.
#[must_use]
pub fn codegen_module(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    registry: &CommandRegistry,
) -> ModuleAsm {
    let src = &ir_module.source;
    // The compile's target release: a named dialect's own numeric grammar, else
    // the permissive 9.x default.
    let numbers = tcl_dialect::NumberSyntax::of_dialect_name(ir_module.dialect.as_deref());
    let escapes = tcl_dialect::EscapeSyntax::of_dialect_name(ir_module.dialect.as_deref());
    let module = ModuleEmit {
        registry,
        source: src,
        numbers,
        escapes,
    };
    let top = codegen_function_src(&cfg_module.top_level, &[], false, &[], module, 0);
    let mut procs: HashMap<String, FunctionAsm> = HashMap::new();
    for (qname, cfg_func) in &cfg_module.procedures {
        let ir_proc = ir_module.procedures.get(qname);
        // Skip procs defined inside namespace eval — tclsh compiles
        // them lazily at runtime, not at compile time.
        if let Some(p) = ir_proc
            && p.namespace_scoped
        {
            continue;
        }
        let params: Vec<&str> = ir_proc
            .map(|p| p.params.iter().map(String::as_str).collect())
            .unwrap_or_default();
        // The proc's definition line drives proc-relative `errorInfo` lines.
        let base_line = ir_proc.map_or(0, |p| line_of(src, p.span.start()));
        procs.insert(
            qname.clone(),
            codegen_function_src(cfg_func, &params, true, &[], module, base_line),
        );
    }
    ModuleAsm {
        top_level: top,
        procedures: procs,
    }
}
