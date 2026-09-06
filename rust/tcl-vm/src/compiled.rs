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

use std::rc::Rc;

use tcl_bytecode::FunctionAsm;

/// How bytecode entered this VM's compilation domain.
///
/// A public [`tcl_bytecode::ModuleAsm`] is an embedder-owned artifact. Admitting
/// it for one execution must not claim that the VM's current compile service
/// produced it; reusable source-bearing children are therefore recompiled on
/// first use when a service is installed. A source-less VM may execute a still-
/// valid admitted child as supplied without changing this marker. VM compilation
/// paths record the service generation that really produced their assembly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompilerProvenance {
    CurrentService(u64),
    AdmittedForeign(u64),
}

impl CompilerProvenance {
    pub(crate) fn generation(self) -> u64 {
        match self {
            Self::CurrentService(generation) | Self::AdmittedForeign(generation) => generation,
        }
    }

    pub(crate) fn is_current_service(self, generation: u64) -> bool {
        self == Self::CurrentService(generation)
    }

    pub(crate) fn is_current_foreign_admission(self, generation: u64) -> bool {
        self == Self::AdmittedForeign(generation)
    }
}

/// Bytecode assembly and the complete VM-local provenance that validated it.
#[derive(Clone)]
pub(crate) struct CompiledUnit {
    pub(crate) asm: Rc<FunctionAsm>,
    pub(crate) source_namespace: String,
    pub(crate) profile_generation: u64,
    pub(crate) command_epoch: u64,
    pub(crate) compiler: CompilerProvenance,
}

impl CompiledUnit {
    pub(crate) fn new(
        asm: Rc<FunctionAsm>,
        source_namespace: String,
        profile_generation: u64,
        command_epoch: u64,
        compiler: CompilerProvenance,
    ) -> Self {
        Self {
            asm,
            source_namespace,
            profile_generation,
            command_epoch,
            compiler,
        }
    }
}
