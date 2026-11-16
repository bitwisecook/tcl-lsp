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

//! Traces: variable and command trace registration, the native `env` and `tcl_precision` observers, command-trace firing and relocation, and firing variable traces.

use super::{
    CmdTraceEntry, NativeTraceInterpreterState, NativeTraceResultState, ResolvedVar, VarTrace,
    VarTraceCell, Vm, err, native_operation_currency, variable_trace_outcomes,
};
use crate::value::Value;
use crate::vars::VarState as Local;
use tcl_runtime_api::guard::GuardDomain;
use tcl_runtime_api::{Completion, VarId};

mod command_traces;
mod env_and_precision;
mod variable_firing;

impl VarTraceCell {
    pub(super) fn array_id(&self) -> Option<VarId> {
        self.array
    }
}

impl CmdTraceEntry {
    pub(super) fn new(ops: Vec<String>, callback: Vec<u8>, token: Option<u64>) -> Self {
        Self {
            ops,
            callback,
            token: std::cell::Cell::new(token),
            firing: std::cell::Cell::new(false),
            untraced: std::cell::Cell::new(false),
        }
    }

    /// Whether `trace remove` has unlinked this registration (see
    /// [`Self::untraced`]).
    pub(crate) fn untraced(&self) -> bool {
        self.untraced.get()
    }

    /// Whether this entry fires for `op`.
    pub(crate) fn has_op(&self, op: &str) -> bool {
        self.ops.iter().any(|o| o == op)
    }

    /// Whether this entry's callback is currently running (its firings are
    /// suppressed while it is).
    pub(crate) fn firing(&self) -> bool {
        self.firing.get()
    }
}

impl Vm {
    /// Save the authentic C trace-chain result role, independently of callback type.
    pub(crate) fn save_native_interp_trace_result(
        &mut self,
    ) -> Result<Option<NativeTraceResultState>, tcl_runtime_api::NativeExecutionError> {
        self.save_native_trace_result(
            tcl_registry::native_variable_trace::NativeTraceStatePurpose::VariableChain,
        )
    }

    pub(super) fn save_native_command_trace_result(
        &mut self,
    ) -> Result<Option<NativeTraceResultState>, tcl_runtime_api::NativeExecutionError> {
        use tcl_registry::native_variable_trace::{
            NativeTraceStatePurpose, NativeTraceStateRecipe,
        };
        match self
            .actual_native_invocation_dialect()
            .native_trace_state_recipe(NativeTraceStatePurpose::CommandChain)
        {
            Some(NativeTraceStateRecipe::None) => Ok(None),
            None => Err(self
                .retain_host_command_refusal("command trace state recipe is unavailable".into())),
            Some(_) => self.save_native_trace_result(NativeTraceStatePurpose::CommandChain),
        }
    }

    pub(crate) fn save_native_script_trace_result(
        &mut self,
    ) -> Result<Option<NativeTraceResultState>, tcl_runtime_api::NativeExecutionError> {
        self.save_native_trace_result(
            tcl_registry::native_variable_trace::NativeTraceStatePurpose::VariableScript,
        )
    }

    fn save_native_trace_result(
        &mut self,
        purpose: tcl_registry::native_variable_trace::NativeTraceStatePurpose,
    ) -> Result<Option<NativeTraceResultState>, tcl_runtime_api::NativeExecutionError> {
        use tcl_registry::native_variable_trace::NativeTraceStateRecipe;
        let scope = native_operation_currency::NativeOperationScope::enter_native(self)?;
        let physical_owner =
            variable_trace_outcomes::TracePhysicalOwner::capture(self, scope.currency().clone());
        let recipe = self
            .actual_native_invocation_dialect()
            .native_trace_state_recipe(purpose);
        if recipe.is_none() || recipe == Some(NativeTraceStateRecipe::None) {
            return Ok(None);
        }
        if recipe == Some(NativeTraceStateRecipe::MoveResult) {
            self.with_native_interp_result(|_| ())
                .map_err(|error| self.retain_variable_trace_value_error(&error))?;
            let result = self
                .native_c_result
                .replace(Some(Value::new_native_string_bytes(b"".as_slice())))
                .expect("C84 transfers its current result role");
            let saved = NativeTraceResultState {
                result,
                interpreter: None,
                recipe: NativeTraceStateRecipe::MoveResult,
                physical_owner,
            };
            scope.check_native(self)?;
            return Ok(Some(saved));
        }
        if recipe != Some(NativeTraceStateRecipe::Interpreter) {
            return Err(self.retain_host_command_refusal(
                "original variable trace state recipe is unavailable".into(),
            ));
        }
        let interpreter = NativeTraceInterpreterState {
            error: self.save_native_error_trace_state().ok_or_else(|| {
                self.retain_host_command_refusal(
                    "original trace interpreter error state is unavailable".into(),
                )
            })?,
            return_state: self.native_errors.native_c_return_state,
            stack: self
                .actual_native_string_protocol()
                .and_then(tcl_syntax::native_string::NativeStringProtocol::tcl_version)
                .is_some_and(tcl_dialect::TclVersion::has_error_stack)
                .then(|| self.native_errors.error_stack.clone()),
        };
        let saved = self
            .with_native_interp_result(|result| {
                Some(NativeTraceResultState {
                    result: result.clone(),
                    interpreter: Some(interpreter),
                    recipe: NativeTraceStateRecipe::Interpreter,
                    physical_owner,
                })
            })
            .map_err(|error| self.retain_variable_trace_value_error(&error))?;
        scope.check_native(self)?;
        Ok(saved)
    }

    pub(crate) fn restore_native_interp_trace_result(
        &mut self,
        state: NativeTraceResultState,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        self.restore_original_trace_state(state)
    }

    /// Whether a traced invocation currently owns an interpreter-wide step
    /// trace. Registration alone does not disable native inline compilation;
    /// C installs this trace after entry callbacks and retires it at settlement.
    pub(crate) fn step_trace_active(&self) -> bool {
        self.exec_step_scopes
            .iter()
            .any(|scope| !scope.entry.untraced())
    }

    pub(crate) fn publish_native_inline_trace_transition(&self, was_disabled: bool) {
        if was_disabled != self.step_trace_active() {
            self.bump_trace_deopt_epoch();
            self.note_native_compiler_cache_mutation(
                tcl_registry::native_procedure::NativeCompilerCacheMutation::InlineTracing,
            );
        }
    }
}

/// Whether a `trace add execution` op set includes `enterstep`/`leavestep` —
/// the C-Tcl-significant "forbids inline compilation" test
/// (`TCL_TRACE_ENTER_DURING_EXEC | TCL_TRACE_LEAVE_DURING_EXEC`). A plain
/// `enter`/`leave` (without step) does not: the traced command's own
/// dispatch already fires it (procs are never opcode-inlined), so it needs
/// no deoptimisation.
fn is_step_capable(ops: &[String]) -> bool {
    ops.iter().any(|o| o == "enterstep" || o == "leavestep")
}

impl Vm {
    // Variable traces (`trace add|remove|info variable`)

    pub(super) fn trace_cell_bytes(&self, name: &[u8]) -> Option<VarTraceCell> {
        let resolved = self.resolve_var_from_bytes(name, self.current_level())?;
        self.trace_cell_from_resolved(&resolved)
    }

    pub(super) fn registered_trace_cell_bytes(&self, original: &[u8]) -> Option<VarTraceCell> {
        let projection = self
            .name_policy_protocol()?
            .recipe()
            .trace_query_input(original)
            .ok()?;
        self.trace_cell_bytes(projection.selected())
    }

    pub(super) fn trace_cell_from_resolved(&self, resolved: &ResolvedVar) -> Option<VarTraceCell> {
        if let Some(elem) = &resolved.elem {
            resolved.base_id?;
            return Some(VarTraceCell {
                id: resolved.id,
                array: resolved.base_id,
                elem: Some(elem.clone()),
            });
        }
        let id = resolved.id?;
        let parent = self
            .var_arena
            .element_parent(id)
            .map(|(array, key)| (array, key.to_owned()));
        Some(VarTraceCell {
            id: Some(id),
            array: parent.as_ref().map(|(array, _)| *array),
            elem: parent.map(|(_, key)| key),
        })
    }

    /// Whether any script or native observer is registered on this cell.
    pub(crate) fn variable_cell_observed(&self, id: VarId) -> bool {
        let observers = &self.variable_observers;
        observers.script_traces.contains_key(&id)
            || observers.native_error_cells.contains_key(&id)
            || observers.precision_cell == Some(id)
            || observers.env_cell == Some(id)
    }

    pub(super) fn variable_observers_active(&self) -> bool {
        !self.variable_observers.native_error_cells.is_empty()
            || !self.variable_observers.script_traces.is_empty()
            || self.variable_observers.precision_cell.is_some()
            || self.variable_observers.env_cell.is_some()
    }

    /// Register a `trace add variable` callback. `old_style` marks the
    /// deprecated `trace variable` spelling (see [`VarTrace::old_style`]).
    #[cfg(test)]
    pub(crate) fn add_var_trace(
        &mut self,
        name: &str,
        ops: Vec<String>,
        command: String,
        old_style: bool,
    ) {
        self.add_var_trace_bytes(name.as_bytes(), ops, &Value::string(command), old_style);
    }

    #[cfg(test)]
    pub(crate) fn add_var_trace_bytes(
        &mut self,
        name: &[u8],
        ops: Vec<String>,
        command: &Value,
        old_style: bool,
    ) {
        let _ = self.add_var_trace_bytes_checked(name, ops, command, old_style);
    }

    pub(crate) fn add_var_trace_bytes_checked(
        &mut self,
        name: &[u8],
        ops: Vec<String>,
        command: &Value,
        old_style: bool,
    ) -> Result<(), Completion<Value>> {
        let original_names =
            crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(self)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let reached = (|| {
            if self.observed_names.is_some() {
                return Err(self.refuse_host_command(
                    "observed variable trace registration is unavailable".into(),
                ));
            }
            let Some(protocol) = self
                .actual_native_invocation_dialect()
                .native_variable_trace_protocol()
            else {
                return Err(self
                    .refuse_host_command("variable trace prefix protocol is unavailable".into()));
            };
            let bytes = original_names
                .read(self, command)
                .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
            let copied = Value::new_native_string_bytes(protocol.variable_prefix_storage(&bytes));
            if self.registered_trace_cell_bytes(name).is_none() {
                self.ensure_trace_variable_bytes(name)?;
            }
            let Some(key) = self
                .registered_trace_cell_bytes(name)
                .and_then(|cell| cell.id)
            else {
                return Ok(());
            };
            self.next_var_trace_id += 1;
            let id = self.next_var_trace_id;
            if !self.variable_observers.script_traces.contains_key(&key) {
                self.var_arena.pin(key);
            }
            self.variable_observers
                .script_traces
                .entry(key)
                .or_default()
                .push(VarTrace {
                    id,
                    ops,
                    command: Some(copied),
                    native: None,
                    old_style,
                });
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
            Ok(())
        })();
        original_names
            .finish(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(crate) fn ensure_trace_variable_bytes(
        &mut self,
        original: &[u8],
    ) -> Result<(), Completion<Value>> {
        if self.observed_names.is_some() {
            return Err(
                self.refuse_host_command("observed variable trace operation is unavailable".into())
            );
        }
        let Some(policy) = self.name_policy_protocol() else {
            return Err(
                self.refuse_host_command("native variable trace name policy is unavailable".into())
            );
        };
        let projection = policy
            .recipe()
            .trace_registration_input(original)
            .map_err(|error| {
                self.refuse_host_command(format!(
                    "variable trace registration is unavailable: {error:?}"
                ))
            })?;
        if self
            .ensure_target_var_from_bytes(projection.selected(), self.current_level())
            .is_some()
        {
            return Ok(());
        }
        let scalar_root = self
            .resolve_var_from_bytes(projection.selected(), self.current_level())
            .is_some_and(|resolved| {
                resolved.elem.is_some()
                    && matches!(
                        resolved
                            .base_id
                            .and_then(|id| self.var_arena.get(id))
                            .map(crate::vars::VarCell::state),
                        Some(Local::Scalar(_))
                    )
            });
        let mut message = b"can't trace \"".to_vec();
        message.extend_from_slice(original);
        message.extend_from_slice(if scalar_root {
            b"\": variable isn't array".as_slice()
        } else {
            b"\": parent namespace doesn't exist"
        });
        Err(err(message))
    }

    /// Remove one `trace remove variable` callback matching `ops` + `command`.
    /// C walks the trace list head→tail and breaks at the first match, and its
    /// head is the newest registration — so among duplicates the **newest**
    /// goes, which is observable in both the surviving firing order and
    /// `trace info`. Our Vec is oldest-first, hence `rposition`.
    /// The old-style flag is deliberately not part of the match (C masks
    /// `TCL_TRACE_OLD_STYLE` out here), so either spelling removes the other's.
    #[cfg(test)]
    pub(crate) fn remove_var_trace(&mut self, name: &str, ops: &[String], command: &str) {
        self.remove_var_trace_bytes(name.as_bytes(), ops, command.as_bytes());
    }

    pub(crate) fn remove_var_trace_bytes(&mut self, name: &[u8], ops: &[String], command: &[u8]) {
        let Some(protocol) = self
            .actual_native_invocation_dialect()
            .native_variable_trace_protocol()
        else {
            let _ = self.refuse_host_command("variable trace prefix removal is unavailable".into());
            return;
        };
        let Some(key) = self
            .registered_trace_cell_bytes(name)
            .and_then(|cell| cell.id)
        else {
            return;
        };
        let mut removed = false;
        let mut emptied = false;
        if let Some(list) = self.variable_observers.script_traces.get_mut(&key) {
            if let Some(index) = list.iter().rposition(|t| {
                t.ops == ops
                    && t.command.as_ref().is_some_and(|prefix| {
                        protocol.variable_prefix_matches(&prefix.string_bytes(), command)
                    })
            }) {
                list.remove(index);
                removed = true;
            }
            if list.is_empty() {
                self.variable_observers.script_traces.remove(&key);
                emptied = true;
            }
        }
        if emptied {
            self.var_arena.unpin(key);
            if let Some((parent, element)) = self
                .var_arena
                .element_parent(key)
                .map(|(parent, element)| (parent, element.to_owned()))
            {
                self.discard_undefined_array_shell(parent, &element, key);
            }
        }
        if removed {
            self.invalidate_guard_domain(GuardDomain::VariableTrace);
        }
    }
}
