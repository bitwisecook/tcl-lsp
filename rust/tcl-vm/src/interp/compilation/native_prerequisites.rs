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

//! The prerequisites a native compiler selection recorded: the fixed math table, command, procedure-header and ensemble compiler registrations, checked against the live interpreter.

use super::super::Vm;
use super::super::commands::CommandSidecarKey;
use super::CommandSemantics;
use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::NsId;

impl Vm {
    pub(in super::super) fn native_math_function_table(
        &self,
    ) -> Option<tcl_runtime_api::native_compilation::NativeMathFunctionTable> {
        use tcl_runtime_api::native_compilation::{
            NativeMathFunctionBinding, NativeMathFunctionTable,
        };
        let surface = tcl_registry::expr_surface::RuntimeExprSurface::for_profile(
            self.actual_native_execution_profile(),
        );
        if surface.has_math_function_command_table() {
            return None;
        }
        let mut functions = self
            .fixed_math
            .handlers
            .keys()
            .filter_map(|key| {
                let name = tcl_registry::mathfunc::global_command_bare_name(key)?;
                let spec = surface.builtin_math_function(name)?;
                let (token, implementation_generation, identity) =
                    self.fixed_math.registrations.get(key)?;
                let arity = (spec.arity.max == spec.arity.min).then(|| usize::from(spec.arity.min));
                Some(NativeMathFunctionBinding {
                    name: name.into(),
                    token: *token,
                    implementation_generation: *implementation_generation,
                    registry_identity: Some(identity.clone()),
                    arity,
                })
            })
            .collect::<Vec<_>>();
        functions.sort_by(|left, right| left.name.cmp(&right.name));
        Some(NativeMathFunctionTable {
            closed: true,
            generation: self.fixed_math.generation,
            functions,
        })
    }

    pub(crate) fn native_math_table_prerequisite_matches(&self, asm: &FunctionAsm) -> bool {
        asm.native_math_table_prerequisite
            .as_ref()
            .is_none_or(|required| self.native_fixed_math_prerequisite().as_ref() == Some(required))
    }

    /// Capture the complete actual fixed table and its owning interpreter.
    pub(crate) fn native_fixed_math_prerequisite(
        &self,
    ) -> Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite> {
        Some(
            tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite {
                interpreter: self.native_interpreter_identity(),
                table: self.native_math_function_table()?,
            },
        )
    }

    pub(crate) fn native_compiler_prerequisites_match(&self, asm: &FunctionAsm) -> bool {
        asm.native_compiler_prerequisites
            .iter()
            .all(|required| self.native_compiler_selection_prerequisite_matches(required))
    }

    pub(crate) fn native_compiler_selection_prerequisite_matches(
        &self,
        required: &tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite,
    ) -> bool {
        match required {
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::ProcedureHeader(required) => {
                self.native_procedure_header_prerequisite_matches(required)
            }
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Ensemble(required) => {
                self.native_ensemble_compiler_prerequisite_matches(required)
            }
            tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::Command(required) => {
                self.native_command_compiler_prerequisite_matches(required)
            }
        }
    }

    /// [`Self::native_compiler_selection_prerequisite_matches`], reusing a
    /// match recorded at the current command-binding epoch. Once bootstrap
    /// ends every table, path and trace change advances that epoch. Matches
    /// are keyed by the retained prerequisite allocation itself.
    pub(crate) fn native_compiler_selection_prerequisite_matches_at(
        &self,
        required: &tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite,
    ) -> bool {
        use tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite as Required;
        const MAX_MATCHED: usize = 256;
        let tracking =
            self.compilation_epochs.command_semantics.get() == CommandSemantics::Tracking;
        let epoch = self.trace_deopt_epoch();
        let registration = match required {
            Required::Command(registration) | Required::Ensemble(registration) => {
                Some(registration)
            }
            Required::ProcedureHeader(_) => None,
        };
        if tracking && let Some(registration) = registration {
            let memo = self.matched_prerequisites.borrow();
            if memo.0 == epoch
                && memo
                    .1
                    .iter()
                    .any(|matched| std::sync::Arc::ptr_eq(matched, registration))
            {
                return true;
            }
        }
        let current = self.native_compiler_selection_prerequisite_matches(required);
        if current
            && tracking
            && let Some(registration) = registration
        {
            let mut memo = self.matched_prerequisites.borrow_mut();
            if memo.0 != epoch || memo.1.len() >= MAX_MATCHED {
                *memo = (epoch, Vec::new());
            }
            memo.1.push(std::sync::Arc::clone(registration));
        }
        current
    }

    pub(crate) fn native_command_compiler_prerequisite_matches(
        &self,
        required: &tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
    ) -> bool {
        required.compiler.ensemble.is_none()
            && required.selected_worker.is_none()
            && self.native_command_compiler_registration_matches(required)
    }

    /// Validate an actual procedure-header token without assigning it stock
    /// command semantics or resolving a lossy display name.
    pub(crate) fn native_procedure_header_prerequisite_matches(
        &self,
        required: &tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite,
    ) -> bool {
        if required.interpreter.owner != self.owner_nonce
            || required.interpreter.interpreter
                != u64::try_from(self.cur.0).expect("interpreter slot fits u64")
            || required.header != tcl_dialect::NativeProcedureHeaderCompilation::NoOp
        {
            return false;
        }
        self.native_compiler_binding_at_lookup(
            required.lookup_namespace_token,
            &required.invocation_word,
        )
        .is_some_and(|binding| {
            binding.slot == required.slot
                && binding.namespace_token == required.namespace_token
                && binding.token == required.token
                && binding.implementation_generation == required.implementation_generation
                && binding.procedure_header == Some(required.header)
                && !binding.has_execution_trace
        })
    }

    pub(crate) fn native_ensemble_compiler_prerequisite_matches(
        &self,
        required: &tcl_runtime_api::native_compilation::NativeEnsembleCompilerPrerequisite,
    ) -> bool {
        required.interpreter == self.native_interpreter_identity()
            && required.compiler.ensemble.is_some()
            && self.native_command_compiler_registration_matches(required)
    }

    fn native_command_compiler_registration_matches(
        &self,
        required: &tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite,
    ) -> bool {
        required.interpreter == self.native_interpreter_identity()
            && required
                .matches_registration_with::<core::convert::Infallible>(|namespace, word| {
                    Ok(self.native_compiler_binding_at_lookup(namespace, word))
                })
                .expect("infallible native registration query")
    }

    pub(in super::super) fn native_compiler_binding_at_lookup(
        &self,
        lookup_namespace_token: u64,
        invocation_word: &tcl_core_types::NameBytes,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCompilationBinding> {
        let Ok(lookup_namespace) = u32::try_from(lookup_namespace_token) else {
            return None;
        };
        let lookup_namespace = NsId(lookup_namespace);
        if !self
            .name_world
            .borrow()
            .ns_intern
            .values()
            .chain(
                self.name_world
                    .borrow()
                    .ns_deferral
                    .retained
                    .values()
                    .flat_map(|record| record.subtree.values()),
            )
            .any(|token| *token == lookup_namespace)
        {
            return None;
        }
        let key = self.resolve_command_from_token_bytes(
            lookup_namespace,
            invocation_word.as_bytes(),
            true,
        )?;
        if self.command_has_execution_trace(&CommandSidecarKey::visible(key.clone())) {
            return None;
        }
        self.visible_command_at_key(&key).and_then(|command| {
            self.native_compilation_binding(&key, &command, self.native_interpreter_identity())
        })
    }
}
