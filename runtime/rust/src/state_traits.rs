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

//! The runtime's impls of the `tcl-runtime-api` interp-state role traits.
//!
//! The runtime satisfies the shared state-mutation contract over its
//! `*mut TclObj` value model, so a consumer of the `tcl-runtime-api` role
//! traits can reach into this runtime's state with the *same* contract the
//! bytecode VM (`tcl-vm`) satisfies over `Rc<Obj>`. All seven role traits —
//! `VarStore`, `Introspect`, `Procs`, `Commands`, `Traces`, `Frames`,
//! `Namespaces` — are implemented here. `Namespaces::find_command` mints a
//! `CommandId` that `Commands::dispatch_id` invokes (the resolve-then-invoke
//! pairing).

use tcl_runtime_api::{
    ArrayElementRead, ArrayTarget, CommandId, Commands, Completion, FrameId, Frames, Introspect,
    Namespaces, NsId, ProcInfo, ProcParam, Procs, Traces, VarStore, VarUnsetError,
};

use crate::interp::{Interp, new_string};
use crate::obj::{self, TclObj};

/// The Family-B variable store, honouring `FrameId` (the absolute frame level,
/// `GLOBAL_FRAME` = 0). Like the bytecode VM's impl, a `FrameId` naming the
/// *active* frame delegates to the by-name accessors verbatim (their namespace
/// resolution + trace firing), and any other frame uses the frame-addressed
/// resolver (`vars::*_at`, resolving as if that frame were active, following
/// links). Removal uses the shared physical callback owner in both cases;
/// quiet bootstrap clearing is a separate internal operation. The VM's
/// noncurrent storage-only ports retain their own independent contract.
/// The refcount contract mirrors the runtime's internal accessors:
/// [`get`](VarStore::get) returns a **borrowed** pointer (the variable table
/// keeps its reference — the caller must not release it), and
/// [`set`](VarStore::set) has the table take its own `+1` on the value.
impl VarStore for Interp {
    type Value = *mut TclObj;

    fn variable_diagnostic_at(
        &self,
        operation: tcl_syntax::naming::NativeVariableDiagnosticOperation,
        reason: tcl_syntax::naming::NativeVariableDiagnosticReason,
        site: tcl_syntax::naming::NativeVariableFailureSite,
        input: tcl_syntax::naming::NativeVariableInputForm<'_>,
    ) -> Result<tcl_syntax::naming::NativeVariableDiagnosticProjection, tcl_syntax::value::ValueError>
    {
        let policy = self
            .execution_name_policy()
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "variable diagnostic",
            ))?;
        tcl_syntax::naming::report_native_variable_diagnostic_at(
            policy.recipe(),
            operation,
            reason,
            site,
            input,
        )
        .map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("variable diagnostic input")
        })
    }

    fn get_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError> {
        let value = self.var_get_at(name, frame.0);
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(value)
    }

    fn set_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
        value: Self::Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let _ = self.var_set_at(name, value, frame.0);
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(())
    }

    fn unset_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let removed = self.var_unset_at(name, frame.0);
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(removed)
    }

    fn unset_command_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<Result<bool, VarUnsetError>, tcl_syntax::value::ValueError> {
        if self.is_constant_at(name, frame.0) {
            return Ok(Err(VarUnsetError::IsConstant));
        }
        self.unset_bytes(frame, name).map(Ok)
    }

    fn exists_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        let value = self.var_exists_at(name, frame.0);
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(value)
    }

    fn get_elem_bytes(
        &self,
        frame: FrameId,
        root: &[u8],
        element: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError> {
        let value = self.var_get_elem_at(root, element, frame.0);
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(value)
    }

    fn set_elem_bytes(
        &mut self,
        frame: FrameId,
        root: &[u8],
        element: &[u8],
        value: Self::Value,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        let _ = self.var_set_elem_at(root, element, value, frame.0);
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(())
    }

    fn array_target_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<ArrayTarget, tcl_syntax::value::ValueError> {
        if self.observed_name_policy_selected() {
            return self.observed_array_target(name, frame.0);
        }
        self.require_variable_name_protocol().map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("variable naming")
        })?;
        Ok(
            crate::vars::array_target_at(&self.frames.borrow(), &self.namespaces(), name, frame.0)
                .map_or_else(
                    || ArrayTarget::named_bytes(frame, name),
                    |target| ArrayTarget::cell_bytes(frame, name, target.id()),
                ),
        )
    }

    fn array_default_state_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<tcl_runtime_api::ArrayDefaultState<Self::Value>, tcl_syntax::value::ValueError>
    {
        if self.observed_name_policy_selected() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        use tcl_syntax::value::ValueError;
        self.native_invocation_dialect()
            .native_array_default_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "array default storage",
            ))?;
        let Some(record) = self.array_operation_target(target) else {
            return Ok(tcl_runtime_api::ArrayDefaultState::Undefined);
        };
        Ok(crate::vars::array_default_state_at_target(
            &self.frames.borrow(),
            &self.namespaces(),
            &record,
        ))
    }

    fn unset_array_default_at(
        &mut self,
        target: &ArrayTarget,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        if self.observed_name_policy_selected() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        use tcl_syntax::value::ValueError;
        self.native_invocation_dialect()
            .native_array_default_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "array default storage",
            ))?;
        if let Some(record) = self.array_operation_target(target) {
            crate::vars::unset_array_default_at_target(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces_mut(),
                &record,
            );
        }
        Ok(())
    }

    fn set_array_default_bytes(
        &mut self,
        frame: FrameId,
        name: &[u8],
        value: Self::Value,
    ) -> Result<Result<(), tcl_runtime_api::ArrayDefaultSetFailure>, tcl_syntax::value::ValueError>
    {
        if self.observed_name_policy_selected() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        use tcl_runtime_api::ArrayDefaultSetFailure;
        use tcl_syntax::value::ValueError;
        self.native_invocation_dialect()
            .native_array_default_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "array default storage",
            ))?;
        if frame != Frames::current(self) {
            return Err(ValueError::CommandProtocolUnavailable(
                "array default setter context",
            ));
        }
        self.require_variable_name_protocol()
            .map_err(|_| ValueError::CommandProtocolUnavailable("array default variable name"))?;
        let outcome = crate::vars::set_array_default_classified(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces_mut(),
            self.current_ns(),
            name,
            value,
        );
        match outcome {
            Ok(outcome) => Ok(outcome),
            Err(crate::frame::VarError::NameProtocolUnavailable) => Err(
                ValueError::CommandProtocolUnavailable("array default variable name"),
            ),
            Err(error) => {
                use tcl_syntax::naming::{
                    NativeVariableDiagnosticOperation as Op,
                    NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
                    NativeVariableInputForm as Input,
                };
                let reason = match error {
                    crate::frame::VarError::NoSuchNamespace => Reason::MissingParentNamespace,
                    crate::frame::VarError::DeletedNamespace => Reason::RetiredNamespace,
                    crate::frame::VarError::DeletedArray => Reason::DetachedElement,
                    crate::frame::VarError::IsConstant => Reason::Constant,
                    crate::frame::VarError::IsArray => Reason::IsArray,
                    _ => {
                        return Err(ValueError::CommandProtocolUnavailable(
                            "array default lookup failure",
                        ));
                    }
                };
                Ok(Err(ArrayDefaultSetFailure::Lookup(
                    self.variable_diagnostic_at(
                        Op::Write,
                        reason,
                        Site::NameLookup,
                        Input::Combined(name),
                    )?,
                )))
            }
        }
    }

    fn get(&self, frame: FrameId, name: &str) -> Option<*mut TclObj> {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_get(name.as_bytes())
        } else {
            self.var_get_at(name.as_bytes(), frame.0)
        }
    }

    fn set(&mut self, frame: FrameId, name: &str, value: *mut TclObj) {
        // The variable table takes a +1; a write-trace error is irrelevant to
        // the storage contract, so the `Result` is intentionally discarded
        // (the VM's impl likewise drops it).
        if frame.0 == self.frames.borrow().current_level() {
            let _ = self.var_set(name.as_bytes(), value);
        } else {
            let _ = self.var_set_at(name.as_bytes(), value, frame.0);
        }
    }

    fn unset(&mut self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_unset(name.as_bytes())
        } else {
            self.var_unset_at(name.as_bytes(), frame.0)
        }
    }

    fn unset_command(&mut self, frame: FrameId, name: &str) -> Result<bool, VarUnsetError> {
        if self.is_constant_at(name.as_bytes(), frame.0) {
            return Err(VarUnsetError::IsConstant);
        }
        Ok(self.unset(frame, name))
    }

    fn unset_confined(&self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.frames.borrow().current_level() {
            self.store_escapes(name.as_bytes())
        } else {
            self.store_escapes_at(name.as_bytes(), frame.0)
        }
    }

    fn unset_confined_bytes(
        &self,
        frame: FrameId,
        name: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        Ok(if frame.0 == self.frames.borrow().current_level() {
            self.store_escapes(name)
        } else {
            self.store_escapes_at(name, frame.0)
        })
    }

    fn exists(&self, frame: FrameId, name: &str) -> bool {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_exists(name.as_bytes())
        } else {
            self.var_exists_at(name.as_bytes(), frame.0)
        }
    }

    // Element access carries the base and key independently through both the
    // current and frame-addressed resolver paths.

    fn get_elem(&self, frame: FrameId, name: &str, key: &str) -> Option<*mut TclObj> {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_get_elem(name.as_bytes(), key.as_bytes())
        } else {
            self.var_get_elem_at(name.as_bytes(), key.as_bytes(), frame.0)
        }
    }

    fn set_elem(&mut self, frame: FrameId, name: &str, key: &str, value: *mut TclObj) {
        if frame.0 == self.frames.borrow().current_level() {
            let _ = self.var_set_elem(name.as_bytes(), key.as_bytes(), value);
        } else {
            let _ = self.var_set_elem_at(name.as_bytes(), key.as_bytes(), value, frame.0);
        }
    }

    fn unset_elem(&mut self, frame: FrameId, name: &str, key: &str) -> bool {
        if frame.0 == self.frames.borrow().current_level() {
            self.var_unset_elem(name.as_bytes(), key.as_bytes())
        } else {
            self.var_unset_elem_at(name.as_bytes(), key.as_bytes(), frame.0)
        }
    }

    fn exists_elem(&self, frame: FrameId, name: &str, key: &str) -> bool {
        self.get_elem(frame, name, key).is_some()
    }

    fn variable_container_model(&self) -> tcl_dialect::VariableContainerModel {
        self.native_invocation_dialect()
            .variable_container_model
            .unwrap_or_default()
    }

    fn array_keys_checked_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<String>>, tcl_syntax::value::ValueError> {
        self.array_key_bytes_checked_at(target)?
            .map(|keys| {
                keys.into_iter()
                    .map(|key| {
                        tcl_syntax::raw_string::RawString::from_bytes(key)
                            .unicode()
                            .map(|key| key.to_string())
                            .map_err(Into::into)
                    })
                    .collect()
            })
            .transpose()
    }

    fn array_key_bytes_checked_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        if self.observed_name_policy_selected() {
            return self.observed_array_keys(target);
        }
        if self.variable_container_model() == tcl_dialect::VariableContainerModel::DictionaryValue {
            return native_dictionary_array_pairs(self, target)?
                .map(|pairs| {
                    let protocol = self
                        .native_invocation_dialect()
                        .native_string_protocol()
                        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "variable container",
                        ))?;
                    pairs
                        .into_iter()
                        .map(|(key, _)| crate::dict::native_object_bytes(key, protocol))
                        .collect()
                })
                .transpose();
        }
        Ok(if target.cell_id().is_some() {
            self.array_keys_at_target(target)
        } else {
            self.array_names(target.name_bytes())
        })
    }

    fn array_search_key_bytes_at(
        &self,
        target: &ArrayTarget,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        if self.observed_name_policy_selected() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        self.array_search_keys_at_target(target)
    }

    fn array_elem_exists_bytes_at(
        &self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        if self.observed_name_policy_selected() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        self.array_search_element_exists_at_target(target, key)
    }

    fn array_read_elem_bytes_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<ArrayElementRead<*mut TclObj>, tcl_syntax::value::ValueError> {
        if self.observed_name_policy_selected() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        if self.variable_container_model() == tcl_dialect::VariableContainerModel::DictionaryValue {
            let Some(pairs) = native_dictionary_array_pairs(self, target)? else {
                return Ok(ArrayElementRead::ArrayInvalidated(
                    tcl_runtime_api::ArrayInvalidation::Unset,
                ));
            };
            let protocol = self
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "variable container",
                ))?;
            for (name, value) in pairs {
                if crate::dict::native_object_bytes(name, protocol)? == key {
                    tcl_syntax::value::ValueOps::pin_value(self, &value);
                    return Ok(ArrayElementRead::Value(value));
                }
            }
            return Ok(ArrayElementRead::Missing(
                tcl_runtime_api::ArrayReadMiss::missing(),
            ));
        }
        let result = self.array_read_elem_at_target(target, key);
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(result)
    }

    fn unset_elem_bytes_at(
        &mut self,
        target: &ArrayTarget,
        key: &[u8],
    ) -> Result<bool, tcl_syntax::value::ValueError> {
        if self.observed_name_policy_selected() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "unmeasured observed variable operation",
            ));
        }
        let removed = if self.variable_container_model()
            == tcl_dialect::VariableContainerModel::DictionaryValue
        {
            self.var_unset_elem_at(target.name_bytes(), key, target.frame().0)
        } else {
            use tcl_runtime_api::variable_destruction::NativeArrayUnsetMemberLookup;
            match self.native_invocation_dialect().array_unset_member_lookup() {
                Some(NativeArrayUnsetMemberLookup::OriginalName) => {
                    self.var_unset_elem_at(target.name_bytes(), key, target.frame().0)
                }
                Some(NativeArrayUnsetMemberLookup::SelectedArray) => {
                    self.array_unset_elem_at_target(target, key)
                }
                None => {
                    return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "array member unset lookup",
                    ));
                }
            }
        };
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(removed)
    }

    fn array_keys(&self, _frame: FrameId, name: &str) -> Option<Vec<String>> {
        // `array_names` is `None` for a non-array, `Some(keys)` (possibly empty)
        // for an array — exactly the contract. Resolves against the active frame.
        self.array_names(name.as_bytes()).map(|keys| {
            keys.iter()
                .map(|k| String::from_utf8_lossy(k).into_owned())
                .collect()
        })
    }

    fn array_target(&self, frame: FrameId, name: &str) -> ArrayTarget {
        crate::vars::array_target_at(
            &self.frames.borrow(),
            &self.namespaces(),
            name.as_bytes(),
            frame.0,
        )
        .map_or_else(
            || ArrayTarget::named(frame, name),
            |target| ArrayTarget::cell(frame, name, target.id()),
        )
    }

    fn array_keys_at(&self, target: &ArrayTarget) -> Option<Vec<String>> {
        if target.cell_id().is_none() {
            return self.array_keys(target.frame(), target.name());
        }
        self.array_keys_at_target(target).map(|keys| {
            keys.into_iter()
                .map(|key| String::from_utf8_lossy(&key).into_owned())
                .collect()
        })
    }

    fn array_read_elem_at(
        &mut self,
        target: &ArrayTarget,
        key: &str,
    ) -> ArrayElementRead<*mut TclObj> {
        self.array_read_elem_at_target(target, key.as_bytes())
    }
}

/// The selected Jim array command reads its dictionary variable through the
/// original frame/name owner. C array-cell snapshots do not supply its members.
fn native_dictionary_array_pairs(
    interp: &Interp,
    target: &ArrayTarget,
) -> Result<Option<Vec<(*mut TclObj, *mut TclObj)>>, tcl_syntax::value::ValueError> {
    // naming.array.jim-original-dictionary-runtime-enumeration
    // docs/design/analysis/name-resolution-proofs/array-jim-original-dictionary-runtime-enumeration.md
    let Some(root) = VarStore::get_bytes(interp, target.frame(), target.name_bytes())? else {
        return Ok(None);
    };
    let protocol = interp
        .native_invocation_dialect()
        .native_string_protocol()
        .filter(|protocol| protocol.is_jim084())
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "Jim dictionary array",
        ))?;
    crate::native_source::bind_context(root, &interp.native_jim_object_context()?)?;
    crate::dict::native_dict_pairs(root, protocol).map(Some)
}

/// Runtime introspection backing the `info` family (`info level`/`info level N`).
///
/// The handle-free role trait that fits *both* runtime models as drafted, so
/// it is the first beyond `VarStore` both runtimes share. `level` is the
/// current proc-nesting depth; `level_argv` builds a
/// **fresh** list of the retained invoking words at an absolute level (`None`
/// for a level with no call — the global frame). Unlike [`VarStore::get`]'s
/// borrowed pointer, the returned `*mut TclObj` is freshly constructed (rc-0)
/// and the caller adopts it (store via `set_result`, or `drop_fresh`), exactly
/// as `info level N` does inline.
impl Introspect for Interp {
    type Value = *mut TclObj;

    fn level(&self) -> usize {
        self.frames.borrow().current_level()
    }

    fn level_argv(&self, level: usize) -> Option<*mut TclObj> {
        let original = self.frames.borrow().original_error_stack_argv_at(level);
        if let Some(original) = original {
            return Some(self.new_list_object(&original));
        }
        let words = self.level_words(level)?;
        let objs: Vec<*mut TclObj> = words.iter().map(|w| new_string(w)).collect();
        Some(self.new_list_object(&objs))
    }
}

/// Proc introspection (`info body`/`args`/`default`) over the runtime's retained
/// [`ProcDef`](crate::interp::ProcDef). `proc_def` already follows `namespace
/// import` redirects to the underlying proc, so an imported proc introspects as
/// its source (info-1.7/2.4). The body/params are cloned into owned bytes — the
/// contract is value-agnostic. Jim `info args` instead returns the original
/// retained formal list, including defaults, reference names and rest labels.
impl Procs for Interp {
    fn proc_info(&self, name: &str) -> Option<ProcInfo> {
        self.proc_info_bytes(name.as_bytes()).ok().flatten()
    }

    fn formal_introspection_name_bytes(
        &self,
        name: &[u8],
    ) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
        let protocol = self
            .native_invocation_dialect()
            .native_name_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "formal introspection",
            ))?;
        Ok(protocol
            .formal_enumeration_name_input(name)
            .selected()
            .to_vec())
    }

    fn proc_body_bytes(
        &self,
        name: &[u8],
    ) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
        let Some(definition) = self.proc_def(name) else {
            return Ok(None);
        };
        definition.check_native_liveness()?;
        let mut context = self.clone();
        tcl_syntax::value::ValueOps::native_string_bytes(
            &mut context,
            &definition.body.checked_ptr()?,
        )
        .map(|bytes| Some(bytes.to_vec()))
    }

    fn proc_original_formal_list_value(
        &self,
        name: &[u8],
    ) -> Result<Option<Self::Value>, tcl_syntax::value::ValueError> {
        let protocol = self
            .native_invocation_dialect()
            .native_name_protocol()
            .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "formal introspection",
            ))?;
        if !protocol.is_jim084() {
            return Ok(None);
        }
        let Some(definition) = self.proc_def(name) else {
            return Ok(None);
        };
        definition.check_native_liveness()?;
        let original = definition.jim_parameters.as_ref().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim formal introspection list",
            ),
        )?;
        original.checked_ptr().map(Some)
    }

    fn proc_formal_names_bytes(
        &self,
        name: &[u8],
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        let Some(definition) = self.proc_def(name) else {
            return Ok(None);
        };
        definition.check_native_liveness()?;
        Ok(Some(
            definition
                .params
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect(),
        ))
    }

    fn proc_default_value_bytes(
        &mut self,
        name: &[u8],
        arg: &[u8],
    ) -> Result<tcl_runtime_api::ProcDefaultValue<Self::Value>, tcl_syntax::value::ValueError> {
        use tcl_runtime_api::ProcDefaultValue;
        let selected = self.formal_introspection_name_bytes(arg)?;
        let Some(definition) = self.proc_def(name) else {
            return Ok(ProcDefaultValue::MissingProcedure);
        };
        definition.check_native_liveness()?;
        for parameter in &definition.params {
            if self.formal_introspection_name_bytes(&parameter.name)? == selected {
                return Ok(ProcDefaultValue::Declared(
                    parameter
                        .default
                        .as_ref()
                        .map(crate::obj::ProcedureObject::checked_ptr)
                        .transpose()?,
                ));
            }
        }
        Ok(ProcDefaultValue::MissingParameter)
    }

    fn proc_info_bytes(
        &self,
        name: &[u8],
    ) -> Result<Option<ProcInfo>, tcl_syntax::value::ValueError> {
        let Some(def) = self.proc_def(name) else {
            return Ok(None);
        };
        def.check_native_liveness()?;
        let mut context = self.clone();
        let params = def
            .params
            .iter()
            .map(|parameter| {
                let default = parameter
                    .default
                    .as_ref()
                    .map(|value| {
                        tcl_syntax::value::ValueOps::native_string_bytes(
                            &mut context,
                            &value.checked_ptr()?,
                        )
                        .map(|bytes| bytes.to_vec())
                    })
                    .transpose()?;
                Ok(ProcParam {
                    name: parameter.name.clone(),
                    default,
                })
            })
            .collect::<Result<Vec<_>, tcl_syntax::value::ValueError>>()?;
        Ok(Some(ProcInfo {
            body: tcl_syntax::value::ValueOps::native_string_bytes(
                &mut context,
                &def.body.checked_ptr()?,
            )?
            .to_vec(),
            params,
        }))
    }
}

/// Bridge the runtime's own [`crate::interp::Code`] to the contract's
/// [`tcl_runtime_api::Code`]. The two enums are structurally identical (the
/// runtime predates the shared `tcl-core-types`); the explicit match keeps them
/// honestly decoupled and breaks loudly if either drifts.
fn api_code(code: crate::interp::Code) -> tcl_runtime_api::Code {
    use crate::interp::Code as Rt;
    use tcl_runtime_api::Code as Api;
    match code {
        Rt::Ok => Api::Ok,
        Rt::Error => Api::Error,
        Rt::Return => Api::Return,
        Rt::Break => Api::Break,
        Rt::Continue => Api::Continue,
        Rt::Other(n) => Api::Other(n),
    }
}

/// Snapshot one runtime completion as the shared [`Completion`] contract.
///
/// Both returned object handles own one reference. The caller must release
/// each exactly once, or transfer it to a runtime slot that takes ownership.
/// The return-options dict is built by the same live error/return-state helper
/// used by `catch` and `try`; it is not a placeholder.
pub(crate) fn capture_completion(
    interp: &mut Interp,
    code: crate::interp::Code,
) -> Completion<*mut TclObj> {
    if interp.host_refusal_pending() {
        // Transport only: callers inspect the retained host channel before publication.
        return Completion::new(api_code(code), core::ptr::null_mut(), core::ptr::null_mut());
    }
    let result = interp.result_obj();
    let options = crate::cmd_error::completion_options(interp, code);
    // SAFETY: `result` is interp-owned and `options` is fresh. Give the
    // completion one owned reference to each; the caller adopts both.
    unsafe {
        obj::incr_ref_count(result);
        obj::incr_ref_count(options);
    }
    Completion::new(api_code(code), result, options)
}

/// Run a full, prebuilt command argv through the inherent dispatcher, then
/// snapshot the resulting [`Completion`].
///
/// `argv[0]` is the already-evaluated command head. This deliberately performs
/// resolution and dispatch only: it does not parse source or repeat word
/// substitution. Callers retain argv elements for the call duration; this
/// helper borrows them and neither adopts nor releases them.
pub(crate) fn dispatch_prebuilt_argv(
    interp: &mut Interp,
    argv: &[*mut TclObj],
) -> Completion<*mut TclObj> {
    let code = Interp::dispatch(interp, argv);
    capture_completion(interp, code)
}

/// Run `name` + `argv` through the inherent dispatch and snapshot a [`Completion`]
/// — the shared body of [`Commands::dispatch`]/[`Commands::dispatch_id`].
///
/// The runtime is natively a `Code`-plus-`set_result` machine, so this bridges
/// to the `Completion` ABI: it builds the name-included argv the inherent
/// `dispatch` expects (taking an owning `+1` on every word for the call, mirroring
/// `dispatch_list_obj`), runs it, then snapshots `(code, result, options)`.
///
/// **Refcount contract.** Unlike [`VarStore::get`]'s borrowed pointer, the
/// returned completion *owns* a `+1` on both `result` and `options`; the caller
/// adopts them and must release each (`decr_ref_count` / `drop_fresh`) when
/// done — the `*mut TclObj` analogue of the VM completion's owned `Rc`s.
fn dispatch_named(
    interp: &mut Interp,
    name: &[u8],
    argv: &[*mut TclObj],
) -> Completion<*mut TclObj> {
    let name_obj = new_string(name);
    let mut full: Vec<*mut TclObj> = Vec::with_capacity(argv.len() + 1);
    full.push(name_obj);
    full.extend_from_slice(argv);
    // An owning +1 on each word for the call's duration, released after (the
    // `dispatch_list_obj` discipline). The fresh `name_obj` (rc 0) is freed by its
    // release unless dispatch retained it; the caller's `argv` (rc >= 1) are
    // returned to their prior count untouched.
    for &w in &full {
        unsafe { obj::incr_ref_count(w) };
    }
    let code = Interp::dispatch(interp, &full); // the inherent dispatch, not the trait method
    for &w in &full {
        unsafe { obj::decr_ref_count(w) };
    }
    capture_completion(interp, code)
}

/// The error completion for a fabricated/stale `CommandId` (rc-balanced like a
/// real one: `result`/`options` each `+1`).
fn invalid_command_id(interp: &mut Interp) -> Completion<*mut TclObj> {
    let code = interp.set_error(b"invalid command id");
    capture_completion(interp, code)
}

/// Command dispatch by name ([`dispatch`](Commands::dispatch)) or by a resolved
/// [`CommandId`] ([`dispatch_id`](Commands::dispatch_id), which invokes the
/// exact retained generation) — the resolve-then-invoke pairing with
/// [`Namespaces::find_command`].
impl Commands for Interp {
    type Value = *mut TclObj;

    fn dispatch(&mut self, name: &str, argv: &[*mut TclObj]) -> Completion<*mut TclObj> {
        dispatch_named(self, name.as_bytes(), argv)
    }

    fn dispatch_id(&mut self, cmd: CommandId, argv: &[*mut TclObj]) -> Completion<*mut TclObj> {
        match self.dispatch_command_id(cmd.0, argv) {
            Some(code) => capture_completion(self, code),
            None => invalid_command_id(self),
        }
    }
}

/// Variable traces: fire `var`'s `op` (`read`/`write`/`unset`) traces, aborting
/// the access if a callback errors.
///
/// Delegates to [`Interp::fire_var_traces_for`], which keeps the trace internals
/// (firing guard, result preservation, per-op `can't read/set "var"` wrapping)
/// in `interp.rs`. Only `read`/`write` callback errors abort (matching C and the
/// VM); `unset`/`array` errors are swallowed, so those fire as `Ok`. The error
/// value is **freshly built** (rc-0) — the caller adopts it (store via
/// `set_result`, or `drop_fresh`), as for [`Introspect::level_argv`].
impl Traces for Interp {
    type Value = *mut TclObj;

    fn fire_bytes(
        &mut self,
        var: &[u8],
        op: &str,
    ) -> Result<Result<(), Self::Value>, tcl_syntax::value::ValueError> {
        if self.host_refusal_pending() {
            return Err(self
                .native_access_refusal()
                .unwrap_or(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "retained native trace activation",
                    ),
                )
                .into());
        }
        let failure = self.fire_var_traces_for(var, op.as_bytes());
        if self.host_refusal_pending() {
            return Err(self
                .native_access_refusal()
                .unwrap_or(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "retained native trace activation",
                    ),
                )
                .into());
        }
        Ok(failure.map_or(Ok(()), |message| Err(new_string(&message))))
    }

    fn fire(&mut self, var: &str, op: &str) -> Result<(), *mut TclObj> {
        match self.fire_var_traces_for(var.as_bytes(), op.as_bytes()) {
            Some(msg) => Err(new_string(&msg)),
            None => Ok(()),
        }
    }
}

/// The call-frame stack. `NsId` is native here (each frame carries its
/// namespace), so [`push`](Frames::push) maps directly to a proc-call frame.
/// [`link`](Frames::link) installs an `upvar`/`global`-style alias in the
/// current frame — the only frame `upvar` ever targets — to `target`'s variable;
/// the target home follows the same `level → frame-or-namespace` rule as
/// variable resolution ([`crate::vars::home_at`]), so a link to `GLOBAL_FRAME`
/// correctly lands in the global namespace table.
impl Frames for Interp {
    fn push(&mut self, ns: NsId) -> FrameId {
        // The contract's `NsId` is a `u32` newtype; the runtime's is a `usize`
        // arena index (`ROOT_NS`/`GLOBAL` both 0).
        let ns = ns.0 as usize;
        self.enter_namespace_activation(ns);
        let frame = FrameId(self.frames.borrow_mut().push(ns));
        if let Err(error) = self.retain_native_jim_frame_namespace(ns) {
            self.report_cmd_error(error.into());
        }
        frame
    }

    fn pop(&mut self) {
        let popped = self.pop_native_call_frame();
        self.leave_namespace_activation(popped);
    }

    fn current(&self) -> FrameId {
        FrameId(self.frames.borrow().current_level())
    }

    fn link(&mut self, here: FrameId, target: FrameId, local: &str, target_name: &str) {
        debug_assert_eq!(
            here.0,
            self.frames.borrow().current_level(),
            "upvar installs in the current frame"
        );
        let (base, elem) = crate::frame::split_array_ref(target_name.as_bytes());
        let target = crate::vars::link_target_at(
            &self.frames.borrow(),
            &self.namespaces(),
            &base,
            elem,
            target.0,
        );
        if let Some(target) = target {
            self.make_upvar(target, local.as_bytes());
        }
    }

    fn in_proc(&self) -> bool {
        self.frames.borrow().in_proc()
    }

    fn var_names(&self, include_links: bool) -> Vec<String> {
        let frames = self.frames.borrow();
        let names = if include_links {
            frames.local_names()
        } else {
            frames.local_names_no_links()
        };
        names
            .iter()
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    }

    fn var_names_bytes(&self, include_links: bool) -> Vec<Vec<u8>> {
        let frames = self.frames.borrow();
        if include_links {
            frames.local_names()
        } else {
            frames.local_names_no_links()
        }
    }

    fn var_names_bytes_checked(
        &self,
        include_links: bool,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        self.selected_variable_table_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native variable table inventory",
            ),
        )?;
        if include_links {
            self.frames
                .borrow()
                .declared_tcloo_variable_names()
                .map_err(|_| {
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "TclOO declaring variable inventory",
                    )
                })?;
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
        self.var_names_bytes_checked(include_links)?;
        self.frames
            .borrow()
            .local_name_pattern_inputs(include_links)
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original frame variable pattern inputs",
                )
            })
    }

    fn const_names(&self) -> Vec<String> {
        crate::vars::const_names(&self.frames.borrow(), &self.namespaces())
            .iter()
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .collect()
    }

    fn const_names_bytes(&self) -> Vec<Vec<u8>> {
        crate::vars::const_names(&self.frames.borrow(), &self.namespaces())
    }
}

/// Namespace name resolution. `NsId` is native (the contract's `u32` newtype
/// bridges the runtime's `usize` arena id), so [`current`](Namespaces::current)
/// is a direct read. [`find_command`](Namespaces::find_command) resolves `name`
/// from `cxt` to an exact `(FQN, generation)` token and interns that as a stable
/// `CommandId`.
impl tcl_runtime_api::Aliases for Interp {
    fn alias_prefix_original_value(
        &mut self,
        original_name: &Self::Value,
    ) -> Result<tcl_runtime_api::AliasPrefixLookup<Self::Value>, tcl_syntax::value::ValueError>
    {
        self.original_alias_prefix_value(*original_name)
    }
}

impl Namespaces for Interp {
    fn namespace_import_binding(&self) -> Option<tcl_dialect::NamespaceImportBinding> {
        self.dialect_profile().namespace_import_binding()
    }

    fn command_alias_prefix_bytes(&self, cmd: CommandId) -> Option<Vec<Vec<u8>>> {
        self.command_alias_prefix_bytes_checked(cmd).ok().flatten()
    }

    fn command_alias_prefix_bytes_checked(
        &self,
        cmd: CommandId,
    ) -> Result<Option<Vec<Vec<u8>>>, tcl_syntax::value::ValueError> {
        self.command_alias_prefix_by_id_checked(cmd.0)
    }
    fn variable_lookup_policy(&self) -> Option<tcl_dialect::VariableLookupPolicy> {
        self.dialect_profile()
            .variable_lookup_policy()
            .or(Some(tcl_dialect::VariableLookupPolicy::Tcl))
    }

    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId> {
        // The contract's `NsId` is a `u32` newtype; the runtime's is a `usize`.
        self.find_command_id(cxt.0 as usize, name.as_bytes())
            .map(CommandId)
    }

    fn current(&self) -> NsId {
        NsId(self.current_ns() as u32)
    }

    fn name(&self, ns: NsId) -> String {
        String::from_utf8_lossy(&self.ns_qualified_name(ns.0 as usize)).into_owned()
    }

    fn command_name(&self, cmd: CommandId) -> Option<String> {
        self.command_fqn(cmd.0)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }

    // Namespace-tree navigation. The contract's `NsId` is a `u32` newtype; the
    // runtime's arena id is a `usize` (both `ROOT_NS`/`GLOBAL` = 0).
    fn find_namespace(&self, cxt: NsId, name: &str) -> Option<NsId> {
        self.namespaces()
            .find_namespace(cxt.0 as usize, name.as_bytes())
            .map(|id| NsId(id as u32))
    }

    fn namespace_is_live(&self, ns: NsId) -> bool {
        self.namespaces().namespace_is_live(ns.0 as usize)
    }

    fn parent(&self, ns: NsId) -> Option<NsId> {
        self.namespaces()
            .parent(ns.0 as usize)
            .map(|id| NsId(id as u32))
    }

    fn children(&self, ns: NsId) -> Vec<NsId> {
        self.namespaces()
            .children(ns.0 as usize)
            .into_iter()
            .map(|id| NsId(id as u32))
            .collect()
    }

    fn children_hash_order(&self, ns: NsId) -> Vec<NsId> {
        self.namespaces()
            .children_hash_order(ns.0 as usize)
            .into_iter()
            .map(|id| NsId(id as u32))
            .collect()
    }

    // Command enumeration consumes the runtime-selected command surface. The
    // runtime owns the registry query that hides an unavailable builtin while
    // retaining user-defined entries in the same namespace.
    fn commands_in(&self, ns: NsId) -> Vec<String> {
        self.visible_command_report_names_in(ns.0 as usize)
            .into_iter()
            .map(|s| String::from_utf8(s).expect("Unicode command reports require checked bytes"))
            .collect()
    }

    fn procs_in(&self, ns: NsId) -> Vec<String> {
        self.namespaces()
            .proc_names(ns.0 as usize)
            .into_iter()
            .map(|s| String::from_utf8(s).expect("Unicode procedure reports require checked bytes"))
            .collect()
    }

    fn aliases_in_bytes_checked(
        &self,
        ns: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        self.visible_alias_report_names_in(ns.0 as usize)
    }

    fn commands_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.visible_command_report_names_in(ns.0 as usize)
    }

    fn procs_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.namespaces()
            .proc_names(ns.0 as usize)
            .iter()
            .map(|name| name.to_vec())
            .collect()
    }

    fn vars_in(&self, ns: NsId) -> Vec<String> {
        self.namespaces()
            .var_names(ns.0 as usize)
            .iter()
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect()
    }

    fn consts_in(&self, ns: NsId) -> Vec<String> {
        self.namespaces()
            .const_names(ns.0 as usize)
            .iter()
            .map(|name| String::from_utf8_lossy(name).into_owned())
            .collect()
    }

    // `Tcl_FindNamespaceVar`'s single probe: the namespace's own `varTable`,
    // including a `variable`-declared but as-yet-unset cell, and never a call
    // frame.
    fn namespace_var_exists(&self, ns: NsId, simple: &str) -> bool {
        self.namespaces()
            .var_table(ns.0 as usize)
            .cell(simple.as_bytes())
            .is_some()
    }

    // -- byte-valued spellings. This runtime keys every table by bytes, so
    // these are the lossless forms; the `&str` methods above are the lossy
    // convenience the UTF-8-keyed VM uses. Shared cores call these, so a name
    // like `[binary format c 255]` reaches the table verbatim.

    fn find_command_bytes(&self, cxt: NsId, name: &[u8]) -> Option<CommandId> {
        self.find_command_id(cxt.0 as usize, name).map(CommandId)
    }

    fn find_command_bytes_checked(
        &self,
        cxt: NsId,
        original: &[u8],
    ) -> Result<Option<CommandId>, tcl_syntax::value::ValueError> {
        use tcl_syntax::{naming::NativeNameContext, value::ValueError};
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable("command lookup"))?;
        if policy.recipe().is_jim084() {
            self.namespaces()
                .jim_namespace_bytes(cxt.0 as usize)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim command namespace object",
                ))?;
            return Ok(self
                .find_command_id(cxt.0 as usize, original)
                .map(CommandId));
        }
        let path = self
            .namespaces()
            .native_context_path(cxt.0 as usize)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "command lookup namespace context",
            ))?;
        let selected = policy
            .recipe()
            .command_lookup_input(NativeNameContext::new(&path), original)
            .map_err(|_| ValueError::CommandProtocolUnavailable("command lookup name purpose"))?;
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(self
            .find_command_id(cxt.0 as usize, selected.selected())
            .map(CommandId))
    }

    fn find_namespace_bytes(&self, cxt: NsId, name: &[u8]) -> Option<NsId> {
        self.namespaces()
            .find_namespace(cxt.0 as usize, name)
            .map(|id| NsId(id as u32))
    }

    fn root_command_context_checked(&self) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "root command lookup context",
            ))?;
        if !policy.recipe().is_jim084() {
            return self.find_namespace_bytes_checked(Namespaces::current(self), b"::");
        }
        let context = self.native_jim_object_context()?;
        let holder = self
            .namespaces()
            .jim_namespace_object(crate::namespace::GLOBAL)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim root command namespace object",
            ))?;
        if !context.is_live() || holder.as_ptr() != context.empty_object().as_ptr() {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim root command context owner",
            ));
        }
        Ok(Some(tcl_runtime_api::ROOT_NS))
    }

    fn find_namespace_bytes_checked(
        &self,
        cxt: NsId,
        original: &[u8],
    ) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        use tcl_syntax::{naming::NativeNameContext, value::ValueError};
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable("namespace lookup"))?;
        if policy.recipe().is_jim084() {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim flat namespace object storage",
            ));
        }
        let path = self
            .namespaces()
            .native_context_path(cxt.0 as usize)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "namespace lookup context",
            ))?;
        let selected = policy
            .recipe()
            .namespace_address_input(NativeNameContext::new(&path), original)
            .map_err(|_| ValueError::CommandProtocolUnavailable("namespace lookup name purpose"))?;
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(self
            .namespaces()
            .find_namespace(cxt.0 as usize, selected.selected())
            .map(|id| NsId(id as u32)))
    }

    fn find_namespace_child_bytes_checked(
        &self,
        parent: NsId,
        member: &[u8],
    ) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        self.namespaces()
            .child_token(parent.0 as usize, member)
            .map(|child| {
                u32::try_from(child).map(NsId).map_err(|_| {
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "namespace child-token width",
                    )
                })
            })
            .transpose()
    }

    fn namespace_variable_name_bytes_checked(
        &self,
        cxt: NsId,
        original: &[u8],
    ) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
        use tcl_syntax::value::ValueError;
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "namespace variable query issuer",
            ))?;
        let protocol = policy.recipe();
        if protocol.is_jim084() {
            let namespaces = self.namespaces();
            let namespace = namespaces.jim_namespace_bytes(cxt.0 as usize).ok_or(
                ValueError::CommandProtocolUnavailable("Jim retained namespace query object"),
            )?;
            let path = tcl_core_types::ByteNamespacePath::root();
            let selected = protocol
                .jim_namespace_canonical_input(
                    tcl_syntax::naming::NativeNameContext::with_jim_namespace(&path, namespace),
                    original,
                )
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable("Jim namespace variable query name")
                })?;
            let mut rooted = b"::".to_vec();
            rooted.extend_from_slice(selected.selected());
            return Ok(Some(rooted));
        }
        self.require_variable_name_protocol().map_err(|_| {
            ValueError::CommandProtocolUnavailable("namespace variable query issuer")
        })?;
        self.namespaces()
            .native_context_path(cxt.0 as usize)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "namespace variable query context",
            ))?;
        let selected = protocol.namespace_variable_query_input(original);
        let mut contexts = vec![cxt.0 as usize];
        if protocol
            .tcl_version()
            .is_some_and(|version| version <= tcl_dialect::TclVersion::V8_6)
            && cxt.0 != 0
            && !selected.selected().starts_with(b"::")
        {
            contexts.push(crate::namespace::GLOBAL);
        }
        for context in contexts {
            let ns = self.namespaces();
            let Some((owner, simple)) = ns.var_home(context, selected.selected()) else {
                continue;
            };
            if !ns.var_table(owner).has_native_namespace_cell(&simple) {
                continue;
            }
            let mut qualified = ns.qualified_name(owner);
            if owner != crate::namespace::GLOBAL {
                qualified.extend_from_slice(b"::");
            }
            qualified.extend_from_slice(&simple);
            return Ok(Some(qualified));
        }
        if let Some(refusal) = self.native_access_refusal() {
            return Err(refusal.into());
        }
        Ok(None)
    }

    fn namespace_var_exists_bytes(&self, ns: NsId, simple: &[u8]) -> bool {
        self.namespaces()
            .var_table(ns.0 as usize)
            .cell(simple)
            .is_some()
    }

    fn name_bytes(&self, ns: NsId) -> Vec<u8> {
        self.ns_qualified_name(ns.0 as usize)
    }

    fn command_name_bytes(&self, cmd: CommandId) -> Option<Vec<u8>> {
        self.command_fqn(cmd.0)
    }

    fn vars_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.namespaces().var_names(ns.0 as usize)
    }

    fn vars_in_bytes_checked(
        &self,
        ns: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        self.selected_variable_table_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native namespace variable table inventory",
            ),
        )?;
        Ok(self.vars_in_bytes(ns))
    }

    fn consts_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.namespaces().const_names(ns.0 as usize)
    }

    fn command_origin(&self, cmd: CommandId) -> Option<CommandId> {
        self.imported_source_id(cmd.0).map(CommandId)
    }
}

impl tcl_cmd_core::native_array_search::NativeArraySearchBackend for Interp {
    fn array_search_protocol(
        &self,
    ) -> Option<tcl_syntax::native_array_search::NativeArraySearchProtocol> {
        self.native_invocation_dialect()
            .native_array_search_protocol(
                tcl_runtime_api::native_hash_abi::supported_backend_array_search_abi()?,
            )
    }
    fn array_search_cache(
        &self,
        value: &*mut TclObj,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<Option<tcl_core_types::NativeArraySearchCache>, tcl_syntax::value::ValueError> {
        crate::obj::native_array_search_cache_in(*value, protocol)
    }
    fn install_array_search_cache(
        &self,
        value: &*mut TclObj,
        cache: tcl_core_types::NativeArraySearchCache,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<(), tcl_syntax::value::ValueError> {
        crate::obj::install_native_array_search_cache(*value, cache, protocol)
    }
    fn array_search_on_original(
        &mut self,
        target: &ArrayTarget,
        sub: &str,
        name: &[u8],
        operand: Option<
            &tcl_cmd_core::native_array_search::NativeArraySearchOperand<'_, *mut TclObj>,
        >,
        protocol: tcl_syntax::native_array_search::NativeArraySearchProtocol,
    ) -> Result<
        Result<*mut TclObj, tcl_syntax::native_array_search::NativeArraySearchFailure>,
        tcl_syntax::value::ValueError,
    > {
        let (handle, bytes, cache) = operand.map_or((None, None, None), |operand| {
            (Some(operand.original), Some(operand.bytes), operand.cache)
        });
        let record = self.array_operation_target(target).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original array search cell"),
        )?;
        let cell = record.original_array_cell().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original array search cell"),
        )?;
        let result =
            cell.native_array_search(sub, name, handle.copied(), bytes, cache, protocol)?;
        if sub == "startsearch" && protocol.start_handle_has_string_primary() {
            if let Ok(value) = result {
                let materialization = self
                    .native_invocation_dialect()
                    .native_string_materialization(None)
                    .filter(|issuer| issuer.protocol().tcl_version() == Some(protocol.version()))
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native array search String producer",
                    ))?;
                crate::obj::retain_native_string_representation(value, materialization)?;
            }
        }
        if sub == "anymore" && protocol.version() == tcl_dialect::TclVersion::V8_4 {
            if let Ok(value) = result {
                let scalar = self
                    .native_invocation_dialect()
                    .native_scalar_getter_protocol()
                    .ok_or(tcl_syntax::value::ValueError::ScalarNumericInputUnavailable)?;
                crate::obj::adopt_native_scalar_cache(
                    value,
                    tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(crate::obj::wide_of(
                        value,
                    )),
                    scalar,
                )?;
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counters;
    use crate::interp::{drop_fresh, new_string, obj_bytes};
    use tcl_runtime_api::{Code, GLOBAL_FRAME, ROOT_NS};

    /// Run `body` against a fresh interpreter and assert it leaks nothing (the
    /// `*mut TclObj` refcount discipline of the `VarStore` impl is correct).
    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    #[test]
    fn varstore_set_get_unset_exists() {
        leak_free(|i| {
            assert!(!i.exists(GLOBAL_FRAME, "x"));
            // A fresh (rc-0) object handed to `set`; the table takes its +1.
            i.set(GLOBAL_FRAME, "x", new_string(b"hi"));
            assert!(i.exists(GLOBAL_FRAME, "x"));
            // `get` is borrowed — read it without releasing.
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "x").unwrap()), b"hi");
            // Overwrite: the table releases the old value and takes the new.
            i.set(GLOBAL_FRAME, "x", new_string(b"bye"));
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "x").unwrap()), b"bye");
            assert!(i.unset(GLOBAL_FRAME, "x"));
            assert!(!i.exists(GLOBAL_FRAME, "x"));
            assert!(!i.unset(GLOBAL_FRAME, "x")); // already gone
        });
    }

    #[test]
    fn varstore_array_elements() {
        leak_free(|i| {
            assert!(!i.exists_elem(GLOBAL_FRAME, "a", "k"));
            i.set_elem(GLOBAL_FRAME, "a", "k", new_string(b"v"));
            assert!(i.exists_elem(GLOBAL_FRAME, "a", "k"));
            assert_eq!(obj_bytes(i.get_elem(GLOBAL_FRAME, "a", "k").unwrap()), b"v");
            assert!(!i.exists_elem(GLOBAL_FRAME, "a", "nope"));
            assert!(i.unset_elem(GLOBAL_FRAME, "a", "k"));
            assert!(!i.exists_elem(GLOBAL_FRAME, "a", "k"));
            i.unset(GLOBAL_FRAME, "a"); // drop the now-empty array
        });
    }

    #[test]
    fn introspect_level_and_argv() {
        leak_free(|i| {
            // Top level: depth 0, the global frame has no invoking call.
            assert_eq!(Introspect::level(i), 0);
            assert!(Introspect::level_argv(i, 0).is_none());
            // Push a proc-call frame and record its invoking words.
            i.frames.borrow_mut().push(crate::namespace::GLOBAL);
            i.frames
                .borrow_mut()
                .set_words(vec![b"p".to_vec(), b"x".to_vec()]);
            assert_eq!(Introspect::level(i), 1);
            // `level_argv` builds a fresh (rc-0) list; the result slot adopts it
            // via `set_result`, so the leak gate stays balanced.
            let argv = Introspect::level_argv(i, 1).expect("level 1 argv");
            i.set_result(argv);
            assert_eq!(i.result_bytes(), b"p x");
            i.frames.borrow_mut().pop();
            assert_eq!(Introspect::level(i), 0);
        });
    }

    #[test]
    fn varstore_honours_frame_id() {
        leak_free(|i| {
            // A global, written while the global frame is active.
            i.set(GLOBAL_FRAME, "g", new_string(b"global"));
            // Enter a proc-call frame (its own local table).
            let lvl = i.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let here = FrameId(lvl);
            assert_ne!(here, GLOBAL_FRAME);
            i.set(here, "loc", new_string(b"local"));
            // FrameId is honoured: the global is reachable via GLOBAL_FRAME but
            // is not a proc local; the local lives in the proc frame only.
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g").unwrap()), b"global");
            assert!(i.get(here, "g").is_none());
            assert!(i.exists(here, "loc"));
            assert!(!i.exists(GLOBAL_FRAME, "loc"));
            // Frame-addressed pair access must not rebuild `z(b(key)` and
            // reparse it as a different array reference.
            i.set_elem(GLOBAL_FRAME, "z(b", "key", new_string(b"pair"));
            assert_eq!(
                obj_bytes(i.get_elem(GLOBAL_FRAME, "z(b", "key").unwrap()),
                b"pair"
            );
            assert!(i.exists_elem(GLOBAL_FRAME, "z(b", "key"));
            assert!(i.unset_elem(GLOBAL_FRAME, "z(b", "key"));
            // Reach back into the global frame from the proc frame.
            i.set(GLOBAL_FRAME, "g2", new_string(b"two"));
            // Pop the proc frame (frees `loc`); the reached-back write is visible.
            i.frames.borrow_mut().pop();
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g2").unwrap()), b"two");
            assert!(i.unset(GLOBAL_FRAME, "g2"));
            assert!(i.unset(GLOBAL_FRAME, "g"));
            assert!(i.unset(GLOBAL_FRAME, "z(b"));
            assert!(!i.exists(GLOBAL_FRAME, "g"));
        });
    }

    #[test]
    fn commands_dispatch_builtin_and_unknown() {
        leak_free(|i| {
            // A builtin runs and yields its result. The caller owns its argv
            // (rc >= 1, the dispatch contract); the completion owns +1 on the
            // result + options, which the caller releases. UFCS reaches the trait
            // method — the inherent `Interp::dispatch` shadows it for method
            // syntax (a generic `T: Commands` consumer is unaffected).
            let lst = new_string(b"a b c");
            unsafe { obj::incr_ref_count(lst) };
            let c = Commands::dispatch(i, "llength", &[lst]);
            assert_eq!(c.code, Code::Ok);
            assert_eq!(obj_bytes(c.result), b"3");
            unsafe {
                obj::decr_ref_count(c.result);
                obj::decr_ref_count(c.options);
                obj::decr_ref_count(lst);
            }

            // An unknown command name is an error completion.
            let c = Commands::dispatch(i, "no_such_command", &[]);
            assert_eq!(c.code, Code::Error);
            assert!(obj_bytes(c.result).starts_with(b"invalid command name"));
            unsafe {
                obj::decr_ref_count(c.result);
                obj::decr_ref_count(c.options);
            }
        });
    }

    #[test]
    fn traces_fire_ok_and_error() {
        leak_free(|i| {
            // A read trace whose callback succeeds → the access proceeds (Ok).
            // (`;#` comments out the appended `name elem op` words.)
            let _ = i.eval_str(b"trace add variable x read {list ok;#}");
            assert!(Traces::fire(i, "x", "read").is_ok());

            // A read trace whose callback errors → the access aborts, with the
            // error wrapped to the user-facing `can't read "var"` form. The error
            // value is fresh (rc-0); adopt and release it.
            let _ = i.eval_str(b"trace add variable y read {error boom;#}");
            let e = Traces::fire(i, "y", "read").unwrap_err();
            assert_eq!(obj_bytes(e), b"can't read \"y\": boom");
            drop_fresh(e);

            // An `unset` trace error does *not* abort (matches C / the VM): Ok.
            let _ = i.eval_str(b"trace add variable z unset {error nope;#}");
            assert!(Traces::fire(i, "z", "unset").is_ok());
        });
    }

    #[test]
    fn frames_push_pop_current_link() {
        leak_free(|i| {
            // A global, set while the global frame is current.
            i.set(GLOBAL_FRAME, "g", new_string(b"orig"));
            let outer = Frames::current(i);
            assert_eq!(outer, GLOBAL_FRAME);
            // Push a proc-call frame in the global namespace; it becomes current.
            let inner = Frames::push(i, ROOT_NS);
            assert_ne!(inner, outer);
            assert_eq!(Frames::current(i), inner);
            // `upvar`: link `gg` (inner) to the outer frame's global `g`. The
            // link target resolves to the global namespace table (level 0), so
            // reads and writes through `gg` reach `g`.
            Frames::link(i, inner, outer, "gg", "g");
            assert_eq!(obj_bytes(i.get(inner, "gg").unwrap()), b"orig");
            i.set(inner, "gg", new_string(b"changed"));
            // Pop back to the global frame: the link is gone, the global updated.
            Frames::pop(i);
            assert_eq!(Frames::current(i), outer);
            assert_eq!(obj_bytes(i.get(GLOBAL_FRAME, "g").unwrap()), b"changed");
            assert!(i.unset(GLOBAL_FRAME, "g"));
        });
    }

    #[test]
    fn namespaces_current_and_find_command() {
        leak_free(|i| {
            // At the top level the current namespace is the global root.
            assert_eq!(Namespaces::current(i), ROOT_NS);
            // A builtin resolves from the global namespace to a stable CommandId.
            let a = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
            let b = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
            assert_eq!(a, b);
            // A different command resolves to a distinct id.
            let c = Namespaces::find_command(i, ROOT_NS, "llength").expect("llength resolves");
            assert_ne!(a, c);
            // An unknown command resolves to nothing.
            assert!(Namespaces::find_command(i, ROOT_NS, "no_such_command").is_none());
        });
    }

    #[test]
    fn commands_dispatch_id_composes() {
        leak_free(|i| {
            // Resolve a command to a handle, then invoke it *by that handle* —
            // the find_command -> dispatch_id composition.
            let id = Namespaces::find_command(i, ROOT_NS, "list").expect("list resolves");
            let arg = new_string(b"x");
            unsafe { obj::incr_ref_count(arg) };
            let c = Commands::dispatch_id(i, id, &[arg]);
            assert_eq!(c.code, Code::Ok);
            assert_eq!(obj_bytes(c.result), b"x");
            unsafe {
                obj::decr_ref_count(c.result);
                obj::decr_ref_count(c.options);
                obj::decr_ref_count(arg);
            }
            // A fabricated id yields an error completion (no such command).
            let c = Commands::dispatch_id(i, CommandId(9999), &[]);
            assert_eq!(c.code, Code::Error);
            assert_eq!(obj_bytes(c.result), b"invalid command id");
            unsafe {
                obj::decr_ref_count(c.result);
                obj::decr_ref_count(c.options);
            }
        });
    }

    #[test]
    fn command_ids_do_not_rebind_to_same_named_replacements() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"proc p {} {return OLD}"),
                crate::interp::Code::Ok
            );
            let old = Namespaces::find_command(i, ROOT_NS, "p").expect("old p resolves");
            assert_eq!(
                i.eval_str(b"proc p {} {return NEW}"),
                crate::interp::Code::Ok
            );
            let new = Namespaces::find_command(i, ROOT_NS, "p").expect("new p resolves");
            assert_ne!(old, new);

            let stale = Commands::dispatch_id(i, old, &[]);
            assert_eq!(stale.code, Code::Error);
            assert_eq!(obj_bytes(stale.result), b"invalid command id");
            unsafe {
                obj::decr_ref_count(stale.result);
                obj::decr_ref_count(stale.options);
            }

            let live = Commands::dispatch_id(i, new, &[]);
            assert_eq!(live.code, Code::Ok);
            assert_eq!(obj_bytes(live.result), b"NEW");
            unsafe {
                obj::decr_ref_count(live.result);
                obj::decr_ref_count(live.options);
            }
        });
    }
}
