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

//! Jim procedure statics: installing the static slots of a procedure from their original declaration.

use super::super::{
    ResolvedVar, Vm, err, native_name_consumers, native_numeric_ops, variable_cleanup,
};
use crate::value::Value;
use crate::vars::{VarState as Local, VarTable};
use std::rc::Rc;
use tcl_runtime_api::{Completion, VarId};

impl Vm {
    /// A Jim static reference retains the value after its original name is removed.
    pub(super) fn detach_static_reference(&mut self, resolved: &ResolvedVar) -> Option<bool> {
        let original = variable_cleanup::OriginalVariableCleanup::enter(self);
        original.detach_static(self, resolved)
    }

    /// Install native Jim static slots from their original member objects.
    pub(crate) fn prepare_procedure_statics_value(
        &mut self,
        original: &Value,
    ) -> Result<Rc<crate::vars::StaticVariables>, Completion<Value>> {
        let scope = native_name_consumers::OriginalNameConsumerScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = self.prepare_procedure_statics_value_in_original_name(original, &scope);
        self.original_name_finish(scope, reached)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?
    }

    fn populate_procedure_statics(
        &mut self,
        outer: Vec<Value>,
        strings: tcl_syntax::native_string::NativeStringProtocol,
        dialect: tcl_native_model::InvocationDialect,
        table: &mut VarTable,
        fresh: &mut Vec<VarId>,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<(), Completion<Value>> {
        use tcl_registry::native_procedure::{
            StaticVariableValueError as Error, StaticVariableValueInitialiser as Initialiser,
        };
        for specifier in outer {
            let reached = native_numeric_ops::OriginalNumericOps::in_original_scope(
                self,
                original_name_scope.operation_scope(),
            )
            .and_then(|ops| {
                ops.list(&specifier, strings)
                    .map(|items| items.as_ref().clone())
            });
            let fields = self
                .original_name_outcome(original_name_scope, reached)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            if !matches!(fields.len(), 1 | 2) {
                let bytes = self
                    .native_name_operand_bytes_checked(&specifier)
                    .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
                return Err(err(Error::Fields.message(&bytes)));
            }
            let bytes = self
                .native_name_operand_bytes_checked(&fields[0])
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let selected = tcl_registry::native_procedure::static_variable_value_specifier(
                dialect,
                fields.len(),
                Some(&bytes),
            )
            .expect("selected native Jim static name");
            let declaration = selected.map_err(|error| {
                let name = if bytes.first() == Some(&b'&') {
                    &bytes[1..]
                } else {
                    &bytes[..]
                };
                err(error.message(name))
            })?;
            let creates_cell = declaration.initialiser != Initialiser::CaptureCurrentCell;
            let id = match declaration.initialiser {
                Initialiser::LiteralValue => self.var_arena.alloc(Local::Scalar(fields[1].clone())),
                Initialiser::CopyCurrent => self
                    .resolve_var_from_bytes(&declaration.name, self.current_level())
                    .and_then(|resolved| resolved.id)
                    .and_then(|id| self.var_arena.copy_cell(id)),
                Initialiser::CaptureCurrentCell => self
                    .var_binding_from_bytes(&declaration.name, self.current_level())
                    .and_then(|binding| self.var_table(binding.owner)?.get(&binding.name).copied()),
            };
            if creates_cell && let Some(id) = id {
                fresh.push(id);
            }
            original_name_scope
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let id = id.ok_or_else(|| err(Error::MissingCurrent.message(&declaration.name)))?;
            if table.contains_key(&declaration.name) {
                return Err(err(Error::Duplicate.message(&declaration.name)));
            }
            table.insert(declaration.name, id);
        }
        Ok(())
    }

    pub(crate) fn prepare_procedure_statics_value_in_original_name(
        &mut self,
        original: &Value,
        original_name_scope: &native_name_consumers::OriginalNameConsumerScope,
    ) -> Result<Rc<crate::vars::StaticVariables>, Completion<Value>> {
        original_name_scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;

        let dialect = self.actual_native_invocation_dialect();
        if !dialect
            .native_scalar_getter_protocol()
            .is_some_and(tcl_syntax::scalar_getter::NativeScalarGetterProtocol::is_jim084)
        {
            return Err(
                self.refuse_host_command("native procedure statics protocol is unavailable".into())
            );
        }
        let strings = dialect.native_string_protocol().ok_or_else(|| {
            crate::command::completion_from_cmd_error(
                self,
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native procedure statics List",
                )
                .into(),
            )
        })?;
        let reached = native_numeric_ops::OriginalNumericOps::in_original_scope(
            self,
            original_name_scope.operation_scope(),
        )
        .and_then(|ops| {
            ops.list(original, strings)
                .map(|items| items.as_ref().clone())
        });
        let outer = self
            .original_name_outcome(original_name_scope, reached)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let mut table = VarTable::new();
        let mut fresh = Vec::new();
        let result = self.populate_procedure_statics(
            outer,
            strings,
            dialect,
            &mut table,
            &mut fresh,
            original_name_scope,
        );
        let result = match original_name_scope.check(self) {
            Ok(()) => result,
            Err(error) => Err(crate::command::completion_from_cmd_error(self, error)),
        };
        if let Err(error) = result {
            for id in fresh {
                self.var_arena.discard_unbound(id);
            }
            original_name_scope
                .check(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            return Err(error);
        }
        original_name_scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let retained = self.var_arena.retain_statics(table);
        original_name_scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        Ok(retained)
    }
}
