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

//! Namespace identity: interning a path to its token, namespace operands from native bytes, activating and deleting by operand, lookup errors and reporting names.

use super::super::commands::NativeCommandLookupUnavailable;
use super::super::namespace_context::{
    NamespaceExecutionContext, OriginalJimNamespaceContext, ProcedureNamespace,
};
use super::super::{NamespacePath, Vm, native_operation_currency};
use super::{NativeNamespaceAddress, NativeNamespaceAddressError};
use crate::frame::CallFrame;
use crate::value::Value;
use std::rc::Rc;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{Completion, NsId, ROOT_NS};

impl Vm {
    /// The current namespace (canonical, no leading `::`; `""` = global).
    pub(crate) fn current_ns(&self) -> &str {
        self.resolution_stacks
            .ns_stack
            .last()
            .and_then(Option::as_ref)
            .expect("operation requires a materialized namespace report")
            .try_utf8()
            .expect("Unicode namespace compatibility view requires checked native bytes")
    }

    /// Intern a canonical namespace path to its stable `NsId`, minting one on
    /// first sight (`ROOT_NS` = 0 = the empty path).
    pub(in super::super) fn intern_ns_path(&mut self, path: NamespacePath) -> NsId {
        if let Some(id) = self.name_world.borrow().ns_intern.get(&path).copied() {
            return id;
        }
        let parent = path.parent().map(|parent| self.intern_ns_path(parent));
        let world_handle = Rc::clone(&self.name_world);
        let mut world = world_handle.borrow_mut();
        let id = NsId(u32::try_from(world.ns_arena.len()).expect("namespace count fits u32"));
        world.ns_arena.push(path.clone());
        world.ns_parents.push(parent);
        world.ns_intern.insert(path, id);
        drop(world);
        self.ns_vars.entry(id).or_default();
        id
    }

    /// Intern a Tcl-written absolute namespace name. Internal callers with an
    /// already-structured path use [`Self::intern_ns_path`] so rendered names
    /// are never parsed back into identity.
    pub fn intern_ns(&mut self, name: &str) -> NsId {
        self.intern_ns_path(NamespacePath::from_segments(
            tcl_syntax::naming::qualifier_segments(name.as_bytes()),
        ))
    }

    fn namespace_path_from_written(&self, context: NsId, written: &str) -> NamespacePath {
        let mut path = if tcl_syntax::naming::written_command_parts(written.as_bytes()).absolute {
            NamespacePath::root()
        } else {
            self.ns_path(context)
        };
        for segment in tcl_syntax::naming::qualifier_segments(written.as_bytes()) {
            path.push(segment);
        }
        path
    }

    /// Select the actual engine's namespace operand before token lookup.
    pub(in super::super) fn namespace_address_from_native_bytes(
        &self,
        context: NsId,
        original: &[u8],
    ) -> Result<NativeNamespaceAddress, NativeNamespaceAddressError> {
        use tcl_syntax::naming::{NativeNameContext, NativeNameQualification};
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeNamespaceAddressError::ProtocolUnavailable)?
            .recipe();
        let root_object = if protocol.is_jim084() && context == ROOT_NS {
            Some(
                self.native_jim_root_namespace_bytes()
                    .map_err(|_| NativeNamespaceAddressError::ContextUnavailable)?,
            )
        } else {
            None
        };
        let world = self.name_world.borrow();
        let context_path = world
            .ns_arena
            .get(context.0 as usize)
            .ok_or(NativeNamespaceAddressError::ContextUnavailable)?;
        let native_context = if protocol.is_jim084() {
            let object = world
                .jim_namespace_objects
                .get(&context)
                .ok_or(NativeNamespaceAddressError::ContextUnavailable)?;
            NativeNameContext::with_jim_namespace(
                context_path,
                root_object.as_deref().unwrap_or_else(|| object.as_bytes()),
            )
        } else {
            NativeNameContext::new(context_path)
        };
        let selected = protocol
            .namespace_address_input(native_context, original)
            .map_err(NativeNamespaceAddressError::Projection)?;
        if protocol.is_jim084() {
            return Ok(NativeNamespaceAddress::Jim {
                object: NameBytes::from(selected.selected()),
                original: NameBytes::from(original),
            });
        }
        let absolute = selected.qualification() == NativeNameQualification::Absolute;
        let path = protocol
            .namespace_address_path(native_context, original)
            .map_err(NativeNamespaceAddressError::Projection)?;
        Ok(NativeNamespaceAddress::C { path, absolute })
    }

    pub(in super::super) fn namespace_id_from_written(
        &self,
        context: NsId,
        written: &str,
    ) -> Option<NsId> {
        self.name_world
            .borrow()
            .ns_intern
            .get(&self.namespace_path_from_written(context, written))
            .copied()
    }

    #[cfg(test)]
    pub(crate) fn namespace_token_for_written(&self, written: &str) -> Option<NsId> {
        let path = self.namespace_path_from_written(self.current_ns_id(), written);
        self.namespace_token_at_path(
            &path,
            tcl_syntax::naming::written_command_parts(written.as_bytes()).absolute,
        )
    }

    pub(crate) fn activate_namespace_written(&mut self, written: &str) -> NsId {
        let path = self.namespace_path_from_written(self.current_ns_id(), written);
        self.declare_namespace_path_with_origin(
            path.clone(),
            tcl_syntax::naming::written_command_parts(written.as_bytes()).absolute,
        );
        let existing = self.name_world.borrow().ns_intern.get(&path).copied();
        existing.unwrap_or_else(|| self.intern_ns_path(path))
    }

    /// Resolve an original namespace operand in the actual current token context.
    /// Present original argument words using the selected native usage protocol.
    pub(crate) fn native_argument_usage_header(
        &mut self,
        words: &[Value],
    ) -> Result<Vec<u8>, Completion<Value>> {
        let scope = native_operation_currency::NativeOperationScope::enter(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let protocol = self.native_invocation_dialect().usage_protocol(Some(
            tcl_registry::native_usage::LogicalUsageProvider::Tcl84CoreSimulation,
        ));
        let bytes = words
            .iter()
            .map(|word| self.native_index_usage_bytes_checked(word))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| crate::command::completion_from_cmd_error(self, error.into()))?;
        scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        let operands = bytes
            .iter()
            .map(|(bytes, index)| {
                if *index {
                    tcl_registry::native_usage::NativeUsageWord::CanonicalIndex(bytes)
                } else {
                    tcl_registry::native_usage::NativeUsageWord::Original(bytes)
                }
            })
            .collect::<Vec<_>>();
        let reached = protocol
            .and_then(|protocol| protocol.render_header(&operands))
            .ok_or_else(|| {
                crate::command::completion_from_cmd_error(
                    self,
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native argument usage protocol",
                    )
                    .into(),
                )
            });
        scope
            .check(self)
            .map_err(|error| crate::command::completion_from_cmd_error(self, error))?;
        reached
    }

    pub(crate) fn namespace_object_bytes(&self, namespace: NsId) -> NameBytes {
        self.ns_name_bytes(namespace)
    }

    /// Enter a namespace frame using the retained token selected by native lookup.
    pub(crate) fn push_ns_eval_token_frame(&mut self, namespace: NsId, call_argv: Vec<Value>) {
        let level = self.frames.len();
        let mut frame = CallFrame::new(level, namespace, None, call_argv);
        if self.uses_native_jim_lookup() {
            frame.jim_storage = self.jim_storage.acquire();
        }
        frame.ns_eval = Some(self.ns_name_bytes(namespace));
        if self.uses_native_jim_lookup() {
            *frame.jim_namespace.get_mut() = Some((
                Some(namespace),
                self.new_jim_declaration_namespace(namespace),
            ));
        }
        self.frames.push(frame);
        self.recursion_depth += 1;
        self.push_ns_token(self.ns_name_bytes(namespace), namespace);
    }

    pub(crate) fn delete_namespace_operand(
        &mut self,
        original: &[u8],
    ) -> Result<bool, Completion<Value>> {
        let address = match self.namespace_address_from_native_bytes(self.current_ns_id(), original)
        {
            Ok(address) => address,
            Err(NativeNamespaceAddressError::Projection(
                tcl_syntax::naming::NameProjectionUnavailable::EmptyNamespaceOutsideRoot,
            )) => return Ok(false),
            Err(error) => return Err(self.refuse_host_command(format!("{error:?}"))),
        };
        match address {
            NativeNamespaceAddress::C { path, .. } => Ok(self.delete_namespace_path(&path)),
            NativeNamespaceAddress::Jim { object, .. } => {
                // The script helper retires matching flat commands and globals;
                // it does not destroy a namespace object or reject an absent name.
                if original.is_empty() || original == b"::" {
                    return Ok(true);
                }
                let mut pattern = object.as_bytes().to_vec();
                pattern.extend_from_slice(b"::*");
                let keys = self.commands.keys().cloned().collect::<Vec<_>>();
                for key in keys {
                    let Some(slot) = self.command_slot(&key) else {
                        continue;
                    };
                    let matched = self
                        .native_namespace_match(
                            tcl_syntax::native_glob::NativeNameGlobPurpose::InfoCommandsSearch,
                            &pattern,
                            slot.simple.as_bytes(),
                        )
                        .map_err(|error| self.refuse_host_command(format!("{error:?}")))?;
                    if matched {
                        self.remove_registered_command(&key);
                    }
                }
                self.delete_jim_namespace_globals_pattern(object.as_bytes())?;
                Ok(true)
            }
        }
    }

    pub(crate) fn activate_namespace_operand(
        &mut self,
        original: &[u8],
    ) -> Result<NsId, Completion<Value>> {
        let address = match self.namespace_address_from_native_bytes(self.current_ns_id(), original)
        {
            Ok(address) => address,
            Err(NativeNamespaceAddressError::Projection(
                reason @ tcl_syntax::naming::NameProjectionUnavailable::EmptyNamespaceOutsideRoot,
            )) => {
                let Some(policy) = self.name_policy_protocol() else {
                    return Err(self.refuse_host_command(
                        "native namespace creation protocol is unavailable".into(),
                    ));
                };
                let report = tcl_syntax::naming::report_native_namespace_creation_error(
                    policy.recipe(),
                    reason,
                )
                .map_err(|error| self.refuse_host_command(format!("{error:?}")))?;
                return Err(self.namespace_operation_error(report));
            }
            Err(error) => return Err(self.refuse_host_command(format!("{error:?}"))),
        };
        match address {
            NativeNamespaceAddress::Jim { object, .. } => {
                Ok(self.intern_jim_namespace_object(object.as_bytes()))
            }
            NativeNamespaceAddress::C { path, absolute } => {
                let dying = self
                    .namespace_token_at_path(&path, absolute)
                    .filter(|token| self.name_world.borrow().dying_namespaces.contains(token));
                if let Some(namespace) = dying {
                    let Some(policy) = self.name_policy_protocol() else {
                        return Err(self.refuse_host_command(
                            "native namespace creation protocol is unavailable".into(),
                        ));
                    };
                    let reuse_parent = policy
                        .recipe()
                        .namespace_creation_reuses_parent(original, false)
                        .map_err(|_| {
                            self.refuse_host_command(
                                "native namespace creation parent route unavailable".into(),
                            )
                        })?;
                    if reuse_parent {
                        return Ok(namespace);
                    }
                    let report = tcl_syntax::naming::report_native_namespace_creation_conflict(
                        policy.recipe(),
                        original,
                    )
                    .map_err(|_| {
                        self.refuse_host_command(
                            "native namespace creation conflict report unavailable".into(),
                        )
                    })?;
                    return Err(self.namespace_operation_error(report));
                }
                if !absolute && let Some(token) = self.retained_token_at_path(&path) {
                    return Ok(token);
                }
                let current = self.current_ns_id();
                let anchor = if absolute { ROOT_NS } else { current };
                let prefix = if absolute {
                    0
                } else {
                    self.ns_path(current).as_segments().len()
                };
                let components = path.as_segments().get(prefix..).ok_or_else(|| {
                    self.refuse_host_command(
                        "native namespace creation anchor geometry unavailable".into(),
                    )
                })?;
                let (publication, created) = self
                    .name_world
                    .borrow_mut()
                    .ensure_namespace_at_components(anchor, components);
                for child in created {
                    self.ns_vars.entry(child).or_default();
                    self.note_namespace_creation_environment();
                }
                publication.map_err(|error| {
                    self.refuse_host_command(format!("native namespace creation: {error}"))
                })
            }
        }
    }

    pub(crate) fn namespace_object_lookup(
        &mut self,
        original: &Value,
    ) -> Result<Option<NsId>, tcl_cmd_core::CmdError> {
        self.native_namespace_object_lookup(original)
    }

    pub(crate) fn namespace_lookup_error_bytes(&mut self, original: &[u8]) -> Completion<Value> {
        self.namespace_lookup_error_at(
            original,
            tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
        )
    }

    pub(crate) fn namespace_lookup_error_at(
        &mut self,
        original: &[u8],
        operation: tcl_syntax::naming::NativeNamespaceLookupOperation,
    ) -> Completion<Value> {
        let Some(policy) = self.name_policy_protocol() else {
            return self
                .refuse_host_command("native namespace reporting protocol is unavailable".into());
        };
        let current = tcl_runtime_api::Namespaces::name_bytes(self, self.current_ns_id());
        let report = match tcl_syntax::naming::report_native_namespace_operation_error(
            policy.recipe(),
            operation,
            original,
            &current,
        ) {
            Ok(report) => report,
            Err(error) => return self.refuse_host_command(format!("{error:?}")),
        };
        self.namespace_operation_error(report)
    }

    pub(crate) fn namespace_operation_error(
        &mut self,
        report: tcl_syntax::naming::NativeNamespaceOperationError,
    ) -> Completion<Value> {
        let string_protocol = report.string_protocol;
        let message = report.message.clone();
        let error = match report.error_code {
            Some(code) => tcl_cmd_core::CmdError::with_error_code_bytes(report.message, code),
            None => tcl_cmd_core::CmdError::new_bytes(report.message),
        };
        let mut completion = crate::command::completion_from_cmd_error(self, error);
        let dialect = self.native_invocation_dialect();
        if dialect.native_string_protocol() == Some(string_protocol) {
            completion.result = match Value::from_native_string_cache(
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
                    protocol: string_protocol,
                    num_chars: None,
                    unicode: None,
                },
                dialect,
                Some((
                    message.into(),
                    tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
                )),
            ) {
                Ok(value) => value,
                Err(error) => return self.refuse_host_command(error.to_string()),
            };
        }
        completion
    }

    pub(crate) fn namespace_path_for_token(&self, token: NsId) -> NamespacePath {
        self.ns_path(token)
    }

    pub(crate) fn native_namespace_context_for_path(
        &self,
        path: &NamespacePath,
    ) -> Result<NsId, NativeCommandLookupUnavailable> {
        if let Some(current) = self.current_namespace_execution_context().resolved_token()
            && self.native_namespace_token_has_owner(current)
            && path == &self.ns_path(current)
        {
            return Ok(current);
        }
        if path.is_empty() {
            return Ok(ROOT_NS);
        }
        let protocol = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?
            .recipe();
        if protocol.is_jim084() {
            let [object] = path.as_segments() else {
                return Err(NativeCommandLookupUnavailable::NamespaceContextUnavailable);
            };
            return self
                .name_world
                .borrow()
                .jim_namespace_objects
                .iter()
                .find_map(|(token, candidate)| (candidate == object).then_some(*token))
                .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable);
        }
        self.namespace_token_at_path(path, true)
            .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)
    }

    pub(crate) fn intern_jim_namespace_object(&mut self, object: &[u8]) -> NsId {
        if let Some(token) = self
            .name_world
            .borrow()
            .jim_namespace_objects
            .iter()
            .find_map(|(token, stored)| (stored.as_bytes() == object).then_some(*token))
        {
            return token;
        }
        let path = NamespacePath::from_segments([object]);
        let id = {
            let mut world = self.name_world.borrow_mut();
            let id = NsId(u32::try_from(world.ns_arena.len()).expect("namespace count fits u32"));
            world.ns_arena.push(path);
            world.ns_parents.push(Some(ROOT_NS));
            world
                .jim_namespace_objects
                .insert(id, NameBytes::from(object));
            id
        };
        self.ns_vars.entry(id).or_default();
        id
    }

    /// Checked analytical namespace display adapter.
    pub(in super::super) fn ns_name(&self, id: NsId) -> String {
        self.ns_name_checked_utf8(id)
            .expect("analytical namespace display requires checked Unicode components")
    }

    pub(in super::super) fn namespace_path_reporting_bytes(path: &NamespacePath) -> NameBytes {
        NameBytes::from(tcl_syntax::naming::native_namespace_key_bytes(path))
    }

    /// Exact reporting bytes do not reconstruct namespace identity.
    pub(crate) fn ns_name_bytes(&self, id: NsId) -> NameBytes {
        if id == ROOT_NS
            && self
                .name_policy_protocol()
                .is_some_and(|policy| policy.recipe().is_jim084())
        {
            return NameBytes::from(
                self.native_jim_root_namespace_bytes()
                    .expect("authenticated Jim top namespace retains its original object")
                    .as_ref(),
            );
        }
        if self
            .name_policy_protocol()
            .is_some_and(|policy| policy.recipe().is_jim084())
            && let Some(object) = self.name_world.borrow().jim_namespace_objects.get(&id)
        {
            return object.clone();
        }
        Self::namespace_path_reporting_bytes(&self.ns_path(id))
    }

    pub(in super::super) fn ns_name_checked_utf8(&self, id: NsId) -> Option<String> {
        self.ns_name_bytes(id).try_utf8().ok().map(str::to_owned)
    }

    pub(crate) fn current_ns_id(&self) -> NsId {
        self.resolution_stacks
            .ns_id_stack
            .last()
            .and_then(NamespaceExecutionContext::resolved_token)
            .expect("operation requires a materialized current namespace")
    }

    pub(crate) fn current_namespace_execution_context(&self) -> NamespaceExecutionContext {
        self.resolution_stacks
            .ns_id_stack
            .last()
            .expect("current namespace execution context")
            .clone()
    }

    pub(in super::super) fn note_original_jim_namespace_materialized(
        &mut self,
        original: &OriginalJimNamespaceContext,
        token: NsId,
    ) {
        let name = self
            .name_world
            .borrow()
            .jim_namespace_objects
            .get(&token)
            .cloned()
            .expect("materialized Jim namespace report belongs to its token");
        let stacks = &mut self.resolution_stacks;
        for (context, report) in stacks.ns_id_stack.iter().zip(&mut stacks.ns_stack) {
            if matches!(context, NamespaceExecutionContext::OriginalJim(selected) if std::ptr::eq(selected.as_ref(), original))
            {
                *report = Some(name.clone());
            }
        }
        for frame in &mut self.frames {
            if matches!(&frame.ns, NamespaceExecutionContext::OriginalJim(selected) if std::ptr::eq(selected.as_ref(), original))
                && let Some((selected, _)) = frame.jim_namespace.get_mut()
            {
                *selected = Some(token);
            }
        }
    }

    pub(crate) fn push_procedure_namespace_context(&mut self, namespace: ProcedureNamespace) {
        let context = namespace.context();
        self.frames.last_mut().expect("entered procedure frame").ns = context.clone();
        match namespace {
            ProcedureNamespace::Resolved { path, token } => self.push_proc_ns(&path, token),
            ProcedureNamespace::OriginalJim(_) => {
                let report = context.resolved_token().and_then(|token| {
                    self.name_world
                        .borrow()
                        .jim_namespace_objects
                        .get(&token)
                        .cloned()
                });
                self.resolution_stacks
                    .push_original_activation(report, context);
            }
        }
    }

    pub(super) fn retained_token_at_path(&self, path: &NamespacePath) -> Option<NsId> {
        self.retained_current_record()?.subtree.get(path).copied()
    }

    /// Resolve a structured namespace path without allowing an old retained
    /// token to alias a same-spelled live recreation.
    pub(in super::super) fn namespace_token_at_path(
        &self,
        path: &NamespacePath,
        absolute: bool,
    ) -> Option<NsId> {
        if !absolute && let Some(id) = self.retained_token_at_path(path) {
            return Some(id);
        }
        self.name_world.borrow().ns_intern.get(path).copied()
    }

    pub(in super::super) fn definition_namespace_token_at_path(
        &mut self,
        path: &NamespacePath,
        absolute: bool,
    ) -> NsId {
        if !absolute && let Some(id) = self.retained_token_at_path(path) {
            return id;
        }
        let existing = self.name_world.borrow().ns_intern.get(path).copied();
        existing.unwrap_or_else(|| self.intern_ns_path(path.clone()))
    }
}
