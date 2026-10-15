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
use std::sync::Arc;

use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{ArtefactIdentityManifest, FatalTail};

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
    /// The parse error to raise once this unit's commands have run, for a body
    /// whose *later* commands do not parse.
    ///
    /// C compiles a procedure body when the procedure is **called**, so a
    /// malformed body neither refuses the definition nor runs with the lenient
    /// lowering's invented meaning: the clean prefix runs on entry and this is
    /// raised after it (#1829).  `None` for every body that parses whole.
    pub(crate) fatal_tail: Option<FatalTail>,
    /// What the module this unit came from says about the world it was
    /// compiled for, so the rungs a disagreeing field rests on are refused
    /// again whenever the unit is re-checked. `None` for a unit no compiler
    /// stated a manifest for: plain-dispatch children, scanner activations.
    pub(crate) manifest: Option<Arc<ArtefactIdentityManifest>>,
    /// The objects this unit has pushed for its literal words, kept while it
    /// lives as C's bytecode keeps its literals.
    pub(crate) literal_values: Rc<LiteralValues>,
}

/// A unit's literal objects by literal index: its strings, which every unit
/// shares through [`crate::value::LiteralTable`], and the lists the compiler
/// built from literal words, which C keeps with the one unit.
#[derive(Default)]
pub(crate) struct LiteralValues {
    pub(crate) strings: std::cell::RefCell<Vec<Option<crate::value::Value>>>,
    pub(crate) lists: std::cell::RefCell<Vec<Option<crate::value::Value>>>,
}

impl LiteralValues {
    /// The object in `slots` at `index`, made by `make` the first time.
    pub(crate) fn get_or(
        slots: &std::cell::RefCell<Vec<Option<crate::value::Value>>>,
        index: usize,
        make: impl FnOnce() -> crate::value::Value,
    ) -> crate::value::Value {
        if let Some(Some(value)) = slots.borrow().get(index) {
            return value.clone();
        }
        let value = make();
        let mut slots = slots.borrow_mut();
        if slots.len() <= index {
            slots.resize(index + 1, None);
        }
        slots[index] = Some(value.clone());
        value
    }
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
            fatal_tail: None,
            manifest: None,
            literal_values: Rc::default(),
        }
    }

    /// Carry the manifest of the module this unit came from.
    pub(crate) fn with_manifest(mut self, manifest: Option<Arc<ArtefactIdentityManifest>>) -> Self {
        self.manifest = manifest;
        self
    }

    /// Carry the parse error this unit raises once its clean prefix has run.
    pub(crate) fn with_fatal_tail(mut self, fatal_tail: Option<FatalTail>) -> Self {
        self.fatal_tail = fatal_tail;
        self
    }
}
