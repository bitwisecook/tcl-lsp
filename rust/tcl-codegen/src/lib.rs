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

//! The back end of the Tcl compiler pipeline: what `tcl-compiler`'s IR, CFG and
//! SSA become.
//!
//! - [`codegen`] — the bytecode emitter ([`codegen::codegen_module`]), WASM
//!   emission, peephole and layout passes, and disassembly formatting.
//! - [`inlining`] — reference-body inlining ahead of emission.
//! - [`compile_service`] — [`compile_service::BytecodeCompileService`], the
//!   bytecode compile entry points the VM and the CLI use.
//!
//! These modules were part of `tcl-compiler`. They live in their own crate so
//! that the analyser, and everything that depends only on it, builds without
//! waiting for the emitters.

#![recursion_limit = "256"]
#![deny(missing_docs)]

// Host diagnostics cannot observe the private transcript worker (as in
// `tcl-compiler`).
macro_rules! eprintln {
    ($($arg:tt)*) => {
        if ::tcl_runtime_api::diagnostic_output::host_diagnostics_enabled() {
            ::std::eprintln!($($arg)*);
        }
    };
}

#[cfg(test)]
#[allow(unused_macros)]
macro_rules! println {
    ($($arg:tt)*) => {
        if ::tcl_runtime_api::diagnostic_output::host_diagnostics_enabled() {
            ::std::println!($($arg)*);
        }
    };
}

pub mod codegen;
pub mod compile_service;
pub mod inlining;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod smoke {
    /// The pipeline end to end: lower, build the CFG, emit bytecode.
    #[test]
    fn smoke_emits_bytecode_for_a_script() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let config = tcl_lexer::LexerConfig::default();
        let ir = tcl_compiler::lowering::lower_script_module_for_bytecode(
            "set x 1\nputs $x\n",
            "",
            &registry,
            config,
            None,
            false,
        );
        let cfg = tcl_compiler::cfg_builder::build_cfg_codegen_with_registry_and_config(
            &ir, false, &registry, config,
        );
        let module = crate::codegen::codegen_module(&cfg, &ir, &registry).expect("emits");
        assert!(!module.top_level.instructions.is_empty());
    }
}
