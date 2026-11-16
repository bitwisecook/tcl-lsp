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

//! The `tcl_runtime_api` traits over `Vm`: `VarStore`, `Introspect`, `Procs`, `Commands`, `Traces`, `Frames`, `Aliases`, `Namespaces` and the array-search backend.

use super::commands::CommandSidecarKey;
use super::{VarBinding, VarTableOwner, Vm, err};
use crate::value::Value;
use crate::vars::VarState as Local;
use tcl_runtime_api::{
    CommandId, Commands, Completion, FrameId, FrameLinkOrigin, Frames, Introspect, NsId, ProcInfo,
    Procs, Traces,
};

mod namespace_queries;
mod variable_store;

/// Runtime introspection backing the `info` family (`info level`/`info level N`).
///
/// Handle-free — the reconciliation finding is that `Introspect` fits *both*
/// runtime models as-drafted (no `FrameId`/`NsId` reshape needed), so it is the
/// first Family-B role trait both the VM and `runtime/rust` satisfy with shared
/// semantics: [`level`](Introspect::level) is the current stack depth and
/// [`level_argv`](Introspect::level_argv) the retained invoking words at an
/// absolute level, `None` for a level with no call (the global frame).
impl Introspect for Vm {
    type Value = Value;

    fn level(&self) -> usize {
        self.current_level()
    }

    fn level_argv(
        &self,
        level: usize,
    ) -> Result<Option<Value>, tcl_runtime_api::NativeExecutionError> {
        Ok(self
            .frame_argv(level)
            .filter(|av| !av.is_empty())
            .map(Value::list))
    }
}

/// Procedure introspection over the retained original declaration. C formal
/// names use the declaration's compiled records; Jim `info args` returns its
/// retained original formal-list object, including default and reference specs.
impl Procs for Vm {
    type OperationError = tcl_cmd_core::CmdError;

    fn formal_introspection_name_bytes(
        &self,
        name: &[u8],
    ) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
        let protocol = self.name_policy_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("formal introspection"),
        )?;
        Ok(protocol
            .recipe()
            .formal_enumeration_name_input(name)
            .selected()
            .to_vec())
    }

    fn proc_body_bytes(&mut self, name: &[u8]) -> Result<Option<Vec<u8>>, Self::OperationError> {
        self.original_proc_body_query(name)
    }

    fn proc_body_original_value(
        &mut self,
        original: &Self::Value,
    ) -> Result<
        Option<tcl_runtime_api::ProcBodyValueQuery<Self::Value, Self::OperationError, Self>>,
        Self::OperationError,
    > {
        if self
            .actual_native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_none()
        {
            return Ok(None);
        }
        self.original_proc_body_value_query(original).map(Some)
    }

    fn proc_original_formal_list_value(
        &self,
        name: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError> {
        let protocol = self.name_policy_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("formal introspection"),
        )?;
        if !protocol.recipe().is_jim084() {
            return Ok(None);
        }
        let Some(definition) = self.proc_def_bytes(name) else {
            return Ok(None);
        };
        if definition.native_reference_count() == 0 {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "retired native procedure declaration",
            ));
        }
        let original = definition.native_parameters.as_ref().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim formal introspection list",
            ),
        )?;
        original.check_native_header()?;
        Ok(Some(original.clone()))
    }

    fn proc_formal_names_bytes(
        &mut self,
        name: &[u8],
    ) -> Result<Option<Vec<Vec<u8>>>, Self::OperationError> {
        Ok(self.proc_def_bytes(name).map(|definition| {
            definition
                .params
                .iter()
                .map(|parameter| parameter.name.as_bytes().to_vec())
                .collect()
        }))
    }

    fn proc_default_value_bytes(
        &mut self,
        name: &[u8],
        arg: &[u8],
    ) -> Result<tcl_runtime_api::ProcDefaultValue<Self::Value>, Self::OperationError> {
        use tcl_runtime_api::ProcDefaultValue;
        let selected = self.formal_introspection_name_bytes(arg)?;
        let Some(definition) = self.proc_def_bytes(name) else {
            return Ok(ProcDefaultValue::MissingProcedure);
        };
        for parameter in &definition.params {
            if self.formal_introspection_name_bytes(parameter.name.as_bytes())? == selected {
                return Ok(ProcDefaultValue::Declared(parameter.default.clone()));
            }
        }
        Ok(ProcDefaultValue::MissingParameter)
    }

    fn proc_info_bytes(&mut self, name: &[u8]) -> Result<Option<ProcInfo>, Self::OperationError> {
        self.original_proc_info_query(name)
    }
}

/// Command dispatch: resolve `name` in the current namespace context and run it
/// with `argv` (name-stripped) to a [`Completion`]. Builtins run inline, procs
/// run to completion in a nested activation, aliases re-evaluate their target,
/// and an unknown name yields an error completion. The owned-`Value` model keeps
/// the refcount discipline implicit (`Rc` clones), unlike the runtime's
/// `*mut TclObj` impl.
impl Commands for Vm {
    type Value = Value;

    fn dispatch(
        &mut self,
        name: &str,
        argv: &[Value],
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        if let Some(first) = self.take_host_boundary_refusal() {
            return Err(first);
        }
        self.try_invoke_command(name, argv)
    }

    fn dispatch_id(
        &mut self,
        cmd: CommandId,
        argv: &[Value],
    ) -> Result<Completion<Value>, tcl_runtime_api::NativeExecutionError> {
        if let Some(first) = self.take_host_boundary_refusal() {
            return Err(first);
        }
        let Some(key) = self.command_fqn(cmd.0) else {
            return self.finish_host_execution(err("invalid command id"));
        };
        let Some(command) = self.commands.get(&key).cloned().or_else(|| {
            self.name_world
                .borrow()
                .ns_deferral
                .retained
                .values()
                .find_map(|record| record.commands.get(&key).cloned())
        }) else {
            return self.finish_host_execution(err("invalid command id"));
        };
        let display = tcl_syntax::naming::root_unrooted_key(&self.command_display_key(&key));
        self.host_execution_depth += 1;
        let completion =
            self.invoke_resolved_command(&display, CommandSidecarKey::visible(key), command, argv);
        self.host_execution_depth -= 1;
        self.finish_host_execution(completion)
    }
}

/// Variable traces keep the callback's Guest error result inside an outer
/// original operational refusal. Both Unicode and byte names use the same
/// checked selected-name route; the full first cause is never reconstructed
/// from the empty completion used to unwind an activation.
impl Traces for Vm {
    type Value = Value;

    fn fire_bytes(
        &mut self,
        var: &[u8],
        op: &str,
    ) -> Result<Result<(), Value>, tcl_runtime_api::NativeExecutionError> {
        if let Some(first) = self.execution_refusal.clone() {
            return Err(first);
        }
        if self.name_policy_protocol().is_none() {
            let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "variable traces",
                ),
            );
            let _ = self
                .refuse_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
            return Err(first);
        }
        self.fire_var_traces_from_cell_bytes(var, op, None, None, None)
            .map(|guest| guest.map_err(|completion| completion.result))
    }
}

/// The call-frame stack. The VM tracks namespace context by both its display
/// name and stable `NsId`, so [`push`](Frames::push) preserves the exact handle
/// supplied by the caller (including a retained dead token) instead of
/// re-resolving its spelling. [`pop`](Frames::pop) unwinds both. [`link`] installs
/// an `upvar`-style alias in the current frame (the only frame `upvar` targets)
/// — the VM stores variables (globals included) in their frame's locals, so a
/// plain level-addressed link suffices.
impl Frames for Vm {
    fn push(&mut self, ns: NsId) -> FrameId {
        let name = self.ns_name_bytes(ns);
        let level = self.push_call_frame(None, Vec::new());
        self.resolution_stacks.push_activation(name, ns);
        FrameId(level)
    }

    fn pop(&mut self) {
        self.pop_ns();
        self.pop_call_frame();
    }

    fn current(&self) -> FrameId {
        FrameId(self.current_level())
    }

    fn link(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str) {
        debug_assert_eq!(
            here.0,
            self.current_level(),
            "upvar installs in the current frame"
        );
        let linked = self.add_link(local, target.0, target_name);
        debug_assert!(linked.is_ok(), "validated frame link must install");
    }

    fn in_proc(&self) -> bool {
        // A proc activation carries its name; the global (0) and `namespace eval`
        // frames do not (the latter runs in the current frame, pushing no frame).
        self.frames.last().is_some_and(|f| f.proc_name.is_some())
    }

    fn var_names(&self, include_links: bool) -> Vec<String> {
        self.frame_var_names(include_links)
    }

    fn var_names_bytes(&self, include_links: bool) -> Vec<Vec<u8>> {
        self.frame_var_name_bytes(include_links)
    }

    fn var_names_bytes_checked(
        &self,
        include_links: bool,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        self.variable_table_recipe().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native variable table inventory",
            ),
        )?;
        if include_links {
            crate::cmd_oo::native_variables::reported_names(self, self.current_level()).map_err(
                |_| {
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "TclOO declaring variable inventory",
                    )
                },
            )?;
        }
        Ok(self.var_names_bytes(include_links))
    }

    fn var_name_pattern_inputs_bytes_checked(
        &self,
        include_links: bool,
    ) -> Result<
        Vec<(Vec<u8>, tcl_syntax::native_glob::NativeNameGlobPurpose)>,
        tcl_syntax::value::ValueError,
    > {
        use tcl_syntax::native_glob::NativeNameGlobPurpose;
        self.var_names_bytes_checked(include_links)?;
        let Some(frame) = self.frames.last() else {
            return Ok(Vec::new());
        };
        let visible = |id| {
            matches!(
                self.var_arena.get(id).map(crate::vars::VarCell::state),
                Some(Local::Scalar(_) | Local::Array(_))
            ) || (include_links
                && matches!(
                    self.var_arena.get(id).map(crate::vars::VarCell::state),
                    Some(Local::Link(_) | Local::NameLink(_))
                ))
        };
        let mut inputs = Vec::new();
        for (index, (name, id)) in frame.compiled_locals.iter().enumerate() {
            let named = frame
                .compiled_local_layout
                .as_ref()
                .is_none_or(|layout| layout.names.get(index).is_some_and(Option::is_some));
            if named && visible(*id) {
                inputs.push((
                    name.as_bytes().to_vec(),
                    NativeNameGlobPurpose::InfoVariablesScan,
                ));
            }
        }
        for (name, id) in frame.locals.iter() {
            if visible(*id) {
                inputs.push((
                    name.as_bytes().to_vec(),
                    NativeNameGlobPurpose::InfoVariablesSearch,
                ));
            }
        }
        if include_links {
            let declared =
                crate::cmd_oo::native_variables::reported_names(self, self.current_level())
                    .map_err(|_| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "TclOO declaring variable inventory",
                        )
                    })?;
            for name in declared {
                if !inputs
                    .iter()
                    .any(|(existing, _)| existing.as_slice() == name.as_bytes())
                {
                    inputs.push((
                        name.as_bytes().to_vec(),
                        NativeNameGlobPurpose::InfoVariablesScan,
                    ));
                }
            }
        }
        Ok(inputs)
    }

    fn const_names(&self) -> Vec<String> {
        self.const_names_bytes()
            .into_iter()
            .map(|name| {
                String::from_utf8(name).expect("byte-valued constants require byte enumeration")
            })
            .collect()
    }

    fn const_names_bytes(&self) -> Vec<Vec<u8>> {
        let mut names: Vec<Vec<u8>> = self.frames.last().map_or_else(Vec::new, |frame| {
            frame
                .local_bindings()
                .filter(|(_, raw)| {
                    let Some(cell) = self.var_arena.get(**raw) else {
                        return false;
                    };
                    match cell.state() {
                        Local::Link(target) => {
                            cell.link_origin() == FrameLinkOrigin::TclOoInstance
                                && self.const_vars.contains(target)
                        }
                        Local::NameLink(_) => false,
                        Local::Undefined | Local::Scalar(_) | Local::Array(_) => {
                            self.const_vars.contains(raw)
                        }
                    }
                })
                .map(|(name, _)| name.as_bytes().to_vec())
                .collect()
        });
        let seen: Vec<_> = self.frames.last().map_or_else(Vec::new, |frame| {
            frame
                .local_bindings()
                .filter(|(_, raw)| {
                    self.var_arena
                        .get(**raw)
                        .is_some_and(|cell| !matches!(cell.state(), Local::Undefined))
                })
                .map(|(name, _)| name.clone())
                .collect()
        });
        if let Ok(declared) =
            crate::cmd_oo::native_variables::reported_bindings(self, self.current_level())
        {
            for (namespace, name, storage) in declared {
                if seen.contains(&name) {
                    continue;
                }
                let binding = VarBinding {
                    owner: VarTableOwner::Namespace(namespace),
                    name: storage,
                };
                if self
                    .raw_variable_binding(&binding)
                    .is_some_and(|id| self.const_vars.contains(&id))
                    && !names
                        .iter()
                        .any(|existing| existing.as_slice() == name.as_bytes())
                {
                    names.push(name.as_bytes().to_vec());
                }
            }
        }
        names
    }
}
