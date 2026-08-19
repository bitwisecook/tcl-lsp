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

/// The per-module facts every function emission shares: the registry, the
/// module source (for `errorInfo` surface text) and the release being compiled
/// for (its dialect name, and the numeral and backslash-escape grammars that
/// name resolves to). Bundled rather than threaded as parallel parameters —
/// the argument list is already at `clippy::too_many_arguments`'s ceiling, and
/// these always travel together.
#[derive(Clone, Copy)]
struct ModuleEmit<'a> {
    registry: &'a CommandRegistry,
    source: &'a str,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    numbers: tcl_dialect::NumberSyntax,
    escapes: tcl_dialect::EscapeSyntax,
    braced_var: tcl_dialect::BracedVarStyle,
    /// The unit's command-binding summary — see
    /// [`CodegenCtx::command_bindings`](crate::codegen::CodegenCtx::command_bindings).
    /// Scanned once per module, not once per function.
    command_bindings: &'a crate::command_binding::ModuleCommandMutations,
}

/// Like [`codegen_function_with_procs`] but threading the module source text so
/// each instruction carries its command's surface text for `errorInfo`, plus
/// the proc body's `base_line` (its `proc` definition line) so the
/// `(procedure … line N)` frame reports a proc-relative line.
#[must_use]
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
    ctx.braced_var = module.braced_var;
    ctx.dialect = module.dialect;
    ctx.command_bindings = Some(module.command_bindings);
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

/// Resolve the compile's dialect name to the profile [`ModuleEmit`] carries.
///
/// `map(by_name)`, deliberately — **not** `and_then(DialectProfile::find)`.
/// Every *named* dialect must stay `Some` here. `find` answers `None` for any
/// name without a catalogue entry of its own (`tk`, an additive command
/// surface, and every unrecognised string), while `by_name` sinks those to the
/// permissive plain-Tcl profile and stays `Some`.
///
/// The distinction is load-bearing because the readers branch on `is_some()`,
/// not on which profile came back: `parse_expr_for_profile` in
/// [`control_flow`](crate::codegen::control_flow) and
/// [`cmd_subst`](crate::codegen::cmd_subst) selects the *target* numeral
/// grammar when this is `Some` and the *thread-ambient* one when it is `None`.
/// Resolving with `find` therefore moves a `tk` (or unknown-dialect) compile's
/// re-parsed `expr` bodies onto whatever grammar the host thread happens to
/// have installed — a silent, host-dependent change of meaning for a literal
/// like `010`. `None` here must mean "this compile named no dialect", nothing
/// else. This reproduces the pre-refactor `parse_expr(&str)` boundary.
fn emit_profile(dialect: Option<&str>) -> Option<&'static tcl_dialect::DialectProfile> {
    dialect.map(tcl_dialect::DialectProfile::by_name)
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
    let dialect = ir_module.dialect.as_deref();
    let numbers = tcl_dialect::NumberSyntax::of_dialect_name(dialect);
    let escapes = tcl_dialect::EscapeSyntax::of_dialect_name(dialect);
    let braced_var = tcl_dialect::BracedVarStyle::of_dialect_name(dialect);
    // Which builtins this unit leaves alone (issue #1585). Scanned from the IR
    // — the top-level script plus every proc / method body — so a `rename` or
    // shadowing `proc` *anywhere* in the unit is seen before the first
    // instruction is emitted, whatever order the bodies are lowered in.
    let command_bindings =
        crate::command_binding::scan_module_command_mutations(ir_module, registry);
    let module = ModuleEmit {
        registry,
        source: src,
        dialect: emit_profile(dialect),
        numbers,
        escapes,
        braced_var,
        command_bindings: &command_bindings,
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
        profile: tcl_dialect::DialectProfile::by_opt_name(dialect),
        top_level: top,
        procedures: procs,
    }
}

#[cfg(test)]
mod tests {
    use super::emit_profile;

    /// Only an *unnamed* compile may reach the readers as `None`.
    ///
    /// `None` selects the thread-ambient numeral grammar over the compile's
    /// target grammar, so any named dialect answering `None` here silently
    /// re-reads literals like `010` under whatever the host thread installed.
    #[test]
    fn every_named_dialect_resolves_to_some_profile() {
        assert!(
            emit_profile(None).is_none(),
            "a compile that named no dialect stays `None`"
        );

        for name in [
            "tcl8.4",
            "tcl8.6",
            "tcl9.0",
            "f5-irules",
            "expect",
            // The regression cases: `tk` is an additive command surface with
            // no catalogue entry, and an unrecognised name is not a licence to
            // fall back to the ambient grammar either. `DialectProfile::find`
            // answers `None` for both.
            "tk",
            "not-a-real-dialect",
        ] {
            assert!(
                emit_profile(Some(name)).is_some(),
                "named dialect {name:?} must resolve to a profile, not to the \
                 thread-ambient grammar"
            );
        }
    }

    /// The two names `find` rejects still pick the compile's *target* numeral
    /// grammar, not the ambient one.
    ///
    /// Pinned through the same `numbers_for` decision the `expr` re-parse
    /// makes: with a non-9.x grammar installed on this thread, a `tk` compile
    /// must still read `010` under the plain-Tcl 9.x grammar `tk` sinks to,
    /// rather than the 8.4 grammar that would make it octal.
    #[test]
    fn an_uncatalogued_dialect_keeps_the_target_numeral_grammar() {
        use tcl_dialect::NumberSyntax;

        // Thread-local, but `cargo test` may run this thread again for another
        // test, so restore it the way `number.rs`'s own ambient tests do.
        let restore = tcl_syntax::number::runtime_syntax();
        tcl_syntax::number::set_runtime_syntax(NumberSyntax::Tcl84);
        assert_eq!(
            tcl_syntax::number::runtime_syntax(),
            NumberSyntax::Tcl84,
            "the ambient grammar must actually be installed for this to pin \
             anything"
        );

        let profile = emit_profile(Some("tk")).expect("`tk` resolves to a profile");
        let numerals = NumberSyntax::of_profile(Some(profile));
        tcl_syntax::number::set_runtime_syntax(restore);

        assert_eq!(
            numerals,
            NumberSyntax::Tcl90,
            "a `tk` compile reads numerals under its target grammar, not the \
             thread-ambient one"
        );
    }
}
