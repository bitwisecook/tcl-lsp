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

//! The agnostic code-generation [`Backend`] trait.
//!
//! The shared frontend (lexer → CST → IR → CFG → SSA → optimise) is
//! target-independent; a backend only owns lowering CFG/IR to its artifact.
//! The bytecode ("TCLVM") backend implements this trait by delegating to the
//! bytecode emitter entry points. WASM uses the separate structured [`Emit`]
//! seam documented in `docs/design/compiler/wasm-codegen.md`.

use tcl_registry::CommandRegistry;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::ir::{Module as IrModule, Procedure as IrProcedure};

use super::emitter::{codegen_function_with_procs, codegen_module};
use super::{FunctionAsm, ModuleAsm};

/// A code-generation backend: lowers the shared CFG/IR to a backend artifact.
///
/// The frontend is shared; an implementor only lowers CFG/IR → its artifact
/// types. Generic over the artifact so backends with different value and
/// instruction models (bytecode vs WASM vs …) share one driving interface.
pub trait Backend {
    /// What this backend produces for a single function (a proc body or the
    /// top-level script).
    type FuncArtifact;
    /// What this backend produces for a whole module.
    type ModuleArtifact;

    /// Lower a single CFG function.
    ///
    /// `is_proc` selects LVT-based vs stack-based variable access; `proc_defs`
    /// are pending proc definitions to interleave at their source positions
    /// (as `codegen_module` does for the top-level script); `registry` is
    /// consulted for codegen-hook resolution — pass the same instance the
    /// lowering pass used.
    fn lower_function(
        &mut self,
        cfg: &CfgFunction,
        params: &[&str],
        is_proc: bool,
        proc_defs: &[IrProcedure],
        registry: &CommandRegistry,
    ) -> Self::FuncArtifact;

    /// Lower an entire module (top-level script + procedures).
    fn lower_module(
        &mut self,
        cfg: &CfgModule,
        ir: &IrModule,
        registry: &CommandRegistry,
    ) -> Self::ModuleArtifact;
}

/// The Tcl 9 bytecode ("TCLVM") backend.
///
/// A zero-sized handle; it implements [`Backend`] by delegating to the existing
/// `emitter` free functions, which keeps the hot, churn-sensitive emitter body
/// untouched.
#[derive(Debug, Clone, Copy, Default)]
pub struct BytecodeBackend;

impl Backend for BytecodeBackend {
    type FuncArtifact = FunctionAsm;
    type ModuleArtifact = ModuleAsm;

    fn lower_function(
        &mut self,
        cfg: &CfgFunction,
        params: &[&str],
        is_proc: bool,
        proc_defs: &[IrProcedure],
        registry: &CommandRegistry,
    ) -> FunctionAsm {
        codegen_function_with_procs(cfg, params, is_proc, proc_defs, registry)
    }

    fn lower_module(
        &mut self,
        cfg: &CfgModule,
        ir: &IrModule,
        registry: &CommandRegistry,
    ) -> ModuleAsm {
        codegen_module(cfg, ir, registry)
    }
}
