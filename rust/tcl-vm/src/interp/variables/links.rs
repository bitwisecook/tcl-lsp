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

//! Variable links: `upvar`, `global`, `variable` and `namespace upvar` links, `TclOO` instance links, and installing a link without losing observable state.

use super::super::namespaces::NativeNamespaceAddress;
use super::super::{VarBinding, VarTableOwner, VariableLinkTarget, Vm};
use super::elem_ref;
use crate::value::Value;
use crate::vars::{NameLink, VarState as Local};
use std::collections::HashSet;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{FrameLinkOrigin, NsId, ROOT_NS, VarId};

/// A semantic `upvar` link failure. The variable resolver decides which home
/// each side denotes; command adapters only render Tcl's operation-specific
/// message and error code.
#[derive(Clone, Copy, Debug)]
pub(crate) enum UpvarLinkError {
    /// The other-variable names an absent namespace.
    TargetNamespace,
    /// A namespace cell would point into a shorter-lived procedure frame.
    Inverted,
    /// The alias name has array-element shape.
    LocalElement,
    /// The alias names an absent namespace.
    LocalNamespace,
    /// The alias already names a defined scalar or array cell.
    Exists,
    /// The alias owns traces, which cannot be discarded by link installation.
    Traced,
    /// Installing the alias would make it address itself.
    SelfLink,
}

impl Vm {
    /// Bind an already-selected procedure formal key in its actual activation.
    #[cfg(test)]
    pub(crate) fn set_local_bytes(&mut self, name: &[u8], value: Value) {
        let binding = VarBinding {
            owner: VarTableOwner::Frame(self.current_level()),
            name: NameBytes::from(name),
        };
        let _ = self.bind_new_var(&binding, Local::Scalar(value));
    }

    /// Install a cross-frame link in the current frame (`upvar`/`global`).
    pub(crate) fn add_link(
        &mut self,
        local: &str,
        level: usize,
        target: &str,
    ) -> Result<(), UpvarLinkError> {
        self.add_link_with_origin(local, level, target, FrameLinkOrigin::Ordinary)
    }

    pub(crate) fn add_link_bytes(
        &mut self,
        local: &[u8],
        level: usize,
        target: &[u8],
    ) -> Result<(), UpvarLinkError> {
        if self.observed_names.is_some() {
            let _ =
                self.refuse_host_command("observed variable link operation is unavailable".into());
            return Err(UpvarLinkError::TargetNamespace);
        }
        let protocol = self
            .name_policy_protocol()
            .ok_or(UpvarLinkError::TargetNamespace)?
            .recipe();
        let local = protocol.variable_root_input(local);
        let binding = VarBinding {
            owner: VarTableOwner::Frame(self.current_level()),
            name: NameBytes::from(local.selected()),
        };
        if protocol.is_jim084() {
            let target = protocol.combined_variable_input(target);
            let target = NameLink {
                original_target: None,
                binding: self
                    .var_binding_from_bytes(target.root().selected(), level)
                    .ok_or(UpvarLinkError::TargetNamespace)?,
                element: target
                    .element()
                    .map(|part| NameBytes::from(part.selected())),
            };
            return self.bind_named_link(&binding, target);
        }
        let target = self
            .ensure_target_var_from_bytes(target, level)
            .ok_or(UpvarLinkError::TargetNamespace)?;
        self.bind_link_var_with_origin(&binding, target, FrameLinkOrigin::Ordinary)
    }

    /// Bind one actual installed `TclOO` compiler primary by its retained ordinal.
    pub(crate) fn add_tcloo_compiled_instance_link(
        &mut self,
        slot: usize,
        namespace: NsId,
        storage: &[u8],
    ) -> Result<(), UpvarLinkError> {
        let binding = self
            .compiled_local_binding(slot)
            .ok_or(UpvarLinkError::TargetNamespace)?;
        let target_binding = VarBinding {
            owner: VarTableOwner::Namespace(namespace),
            name: NameBytes::from(storage),
        };
        let target = self
            .raw_variable_binding(&target_binding)
            .or_else(|| self.bind_new_var(&target_binding, Local::Undefined))
            .ok_or(UpvarLinkError::TargetNamespace)?;
        self.bind_link_var_with_origin(&binding, target, FrameLinkOrigin::TclOoInstanceCompiled)
    }

    pub(crate) fn tcloo_storage_variable_is_defined(
        &self,
        namespace: NsId,
        storage: &[u8],
    ) -> bool {
        let binding = VarBinding {
            owner: VarTableOwner::Namespace(namespace),
            name: NameBytes::from(storage),
        };
        self.raw_variable_binding(&binding)
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(|cell| !matches!(cell.state(), Local::Undefined))
    }

    fn add_link_with_origin(
        &mut self,
        local: &str,
        level: usize,
        target: &str,
        origin: FrameLinkOrigin,
    ) -> Result<(), UpvarLinkError> {
        if self.observed_name_policy_selected() {
            let _ =
                self.refuse_host_command("observed variable link purpose is unavailable".into());
            return Err(UpvarLinkError::TargetNamespace);
        }
        if self.dialect_profile().variable_link_binding()
            == Some(tcl_dialect::VariableLinkBinding::SelectedFrameName)
            && origin == FrameLinkOrigin::Ordinary
        {
            let (base, element) =
                elem_ref(target).map_or((target, None), |(base, key)| (base, Some(key)));
            let target = NameLink {
                original_target: None,
                binding: self
                    .var_binding_from(base, level)
                    .ok_or(UpvarLinkError::TargetNamespace)?,
                element: element.map(tcl_core_types::NameBytes::from),
            };
            let binding = self
                .var_binding_from(local, self.current_level())
                .ok_or(UpvarLinkError::LocalNamespace)?;
            return self.bind_named_link(&binding, target);
        }
        let target = if matches!(
            origin,
            FrameLinkOrigin::TclOoInstance | FrameLinkOrigin::TclOoInstanceCompiled
        ) {
            self.ensure_tcloo_storage_var_from(target, level)
        } else {
            self.ensure_target_var_from(target, level)
        };
        let Some(target) = target else {
            return Err(UpvarLinkError::TargetNamespace);
        };
        let binding = VarBinding {
            owner: VarTableOwner::Frame(self.current_level()),
            name: (local.to_owned()).into(),
        };
        self.bind_link_var_with_origin(&binding, target, origin)
    }

    /// Materialise the direct object-namespace storage cell used by a `TclOO`
    /// projection without collapsing a link it contains. Reads through the
    /// method-local link still resolve transitively, while `info consts` can
    /// test whether the declared storage binding itself is constant.
    fn ensure_tcloo_storage_var_from(&mut self, name: &str, start: usize) -> Option<VarId> {
        if elem_ref(name).is_some() {
            return None;
        }
        let binding = self.var_binding_from(name, start)?;
        self.raw_variable_binding(&binding)
            .or_else(|| self.bind_new_var(&binding, Local::Undefined))
    }

    pub(crate) fn link_namespace_variable_bytes(
        &mut self,
        local: &[u8],
        written: &[u8],
    ) -> Result<(), UpvarLinkError> {
        self.link_namespace_variable_with_binding_bytes(local, written, None)
    }

    pub(in super::super) fn link_namespace_variable_with_binding_bytes(
        &mut self,
        local: &[u8],
        written: &[u8],
        compiled_binding: Option<&VarBinding>,
    ) -> Result<(), UpvarLinkError> {
        let protocol = self
            .name_policy_protocol()
            .ok_or(UpvarLinkError::TargetNamespace)?
            .recipe();
        let target = self
            .namespace_var_binding_from_token_bytes(self.current_ns_id(), written)
            .ok_or(UpvarLinkError::TargetNamespace)?;
        let local = protocol.variable_root_input(local);
        let local_binding = if let Some(binding) = compiled_binding {
            binding.clone()
        } else if protocol.is_jim084() {
            self.var_binding_from_bytes(local.selected(), self.current_level())
                .ok_or(UpvarLinkError::LocalNamespace)?
        } else {
            VarBinding {
                owner: VarTableOwner::Frame(self.current_level()),
                name: NameBytes::from(local.selected()),
            }
        };
        if target == local_binding
            || (self.current_level() == 0
                && target.owner == VarTableOwner::Namespace(ROOT_NS)
                && target.name == local_binding.name)
        {
            if !protocol.is_jim084() {
                let id = self
                    .ensure_target_var_at_binding(&target, None)
                    .ok_or(UpvarLinkError::TargetNamespace)?;
                self.var_arena.mark_namespace_declared(id);
            }
            return Ok(());
        }
        if protocol.is_jim084() {
            return self.bind_named_link(
                &local_binding,
                NameLink {
                    original_target: None,
                    binding: target,
                    element: None,
                },
            );
        }
        let target_id = self
            .bound_var_id(&target)
            .or_else(|| self.bind_new_var(&target, Local::Undefined))
            .ok_or(UpvarLinkError::TargetNamespace)?;
        self.var_arena.mark_namespace_declared(target_id);
        self.bind_link_var(&local_binding, target_id)
    }

    pub(crate) fn link_upvar_bytes(
        &mut self,
        target_level: usize,
        other: &[u8],
        local: &[u8],
    ) -> Result<(), UpvarLinkError> {
        self.link_upvar_with_binding_bytes(target_level, other, local, None)
    }

    pub(in super::super) fn link_upvar_with_binding_bytes(
        &mut self,
        target_level: usize,
        other: &[u8],
        local: &[u8],
        compiled_binding: Option<&VarBinding>,
    ) -> Result<(), UpvarLinkError> {
        if self.observed_names.is_some() {
            let _ =
                self.refuse_host_command("observed variable link operation is unavailable".into());
            return Err(UpvarLinkError::TargetNamespace);
        }
        let protocol = self
            .name_policy_protocol()
            .ok_or(UpvarLinkError::TargetNamespace)?
            .recipe();
        let parts = protocol.combined_variable_input(other);
        let binding = self
            .var_binding_from_bytes(parts.root().selected(), target_level)
            .ok_or(UpvarLinkError::TargetNamespace)?;
        let owner_is_proc = matches!(binding.owner, VarTableOwner::Frame(level) | VarTableOwner::CompiledLocal { level, .. } if self.frame_owns_local_variables(level));
        let target = if protocol.is_jim084() {
            VariableLinkTarget::Name(NameLink {
                original_target: None,
                binding,
                element: parts.element().map(|part| NameBytes::from(part.selected())),
            })
        } else {
            let id = self
                .ensure_target_var_at_binding(
                    &binding,
                    parts
                        .element()
                        .map(tcl_syntax::naming::NativeNameProjection::selected),
                )
                .ok_or(UpvarLinkError::TargetNamespace)?;
            if parts.element().is_none() && self.native_c_variable_name_protocol().is_some() {
                self.var_arena.record_native_entry(id, binding.clone());
            }
            VariableLinkTarget::Cell(id)
        };
        match compiled_binding {
            Some(binding) => self.bind_variable_link(binding, target),
            None => self.install_upvar_link_bytes(target, owner_is_proc, local),
        }
    }

    /// Select the namespace canonicalised once by Jim's upvar helper.
    pub(crate) fn jim_namespace_upvar_canonical_bytes(
        &self,
        namespace: &[u8],
    ) -> Result<NameBytes, tcl_syntax::value::ValueError> {
        let selected = self
            .namespace_address_from_native_bytes(self.current_ns_id(), namespace)
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("namespace upvar context")
            })?;
        let NativeNamespaceAddress::Jim { object, .. } = selected else {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim namespace upvar context",
            ));
        };
        Ok(object)
    }

    pub(in super::super) fn link_namespace_upvar_with_binding_bytes(
        &mut self,
        namespace: NsId,
        other: &[u8],
        local: &[u8],
        compiled_binding: Option<&VarBinding>,
    ) -> Result<(), UpvarLinkError> {
        if self.observed_names.is_some() {
            let _ =
                self.refuse_host_command("observed variable link operation is unavailable".into());
            return Err(UpvarLinkError::TargetNamespace);
        }
        let protocol = self
            .name_policy_protocol()
            .ok_or(UpvarLinkError::TargetNamespace)?
            .recipe();
        let parts = protocol.combined_variable_input(other);
        let binding = self
            .namespace_var_binding_from_token_bytes(namespace, parts.root().selected())
            .ok_or(UpvarLinkError::TargetNamespace)?;
        let target = if protocol.is_jim084() {
            VariableLinkTarget::Name(NameLink {
                original_target: None,
                binding,
                element: parts.element().map(|part| NameBytes::from(part.selected())),
            })
        } else {
            let id = self
                .ensure_target_var_at_binding(
                    &binding,
                    parts
                        .element()
                        .map(tcl_syntax::naming::NativeNameProjection::selected),
                )
                .ok_or(UpvarLinkError::TargetNamespace)?;
            VariableLinkTarget::Cell(id)
        };
        match compiled_binding {
            Some(binding) => self.bind_variable_link(binding, target),
            None => self.install_upvar_link_bytes(target, false, local),
        }
    }

    fn install_upvar_link_bytes(
        &mut self,
        target: VariableLinkTarget,
        owner_is_proc: bool,
        local: &[u8],
    ) -> Result<(), UpvarLinkError> {
        if self.observed_names.is_some() {
            let _ =
                self.refuse_host_command("observed variable link operation is unavailable".into());
            return Err(UpvarLinkError::TargetNamespace);
        }
        let protocol = self
            .name_policy_protocol()
            .ok_or(UpvarLinkError::LocalNamespace)?
            .recipe();
        let parts = protocol.combined_variable_input(local);
        let selected = parts.root().selected();
        let qualified = parts.root().qualification()
            != tcl_syntax::naming::NativeNameQualification::Unqualified;
        let alias_is_namespace =
            qualified || !self.frame_owns_local_variables(self.current_level());
        let jim_binding = if protocol.is_jim084() {
            // Jim checks the original local value before frame inversion or
            // cycle traversal; a defined self-target is an existing value.
            if parts.element().is_some() {
                return Err(UpvarLinkError::LocalElement);
            }
            let binding = self
                .var_binding_from_bytes(local, self.current_level())
                .ok_or(UpvarLinkError::LocalNamespace)?;
            self.validate_link_binding(&binding)?;
            Some(binding)
        } else {
            None
        };
        if owner_is_proc && alias_is_namespace {
            return Err(UpvarLinkError::Inverted);
        }
        if parts.element().is_some() {
            return Err(UpvarLinkError::LocalElement);
        }
        let binding = if let Some(binding) = jim_binding {
            binding
        } else if alias_is_namespace {
            self.namespace_var_binding_from_token_bytes(self.current_ns_id(), selected)
                .ok_or(UpvarLinkError::LocalNamespace)?
        } else {
            VarBinding {
                owner: VarTableOwner::Frame(self.current_level()),
                name: NameBytes::from(selected),
            }
        };
        self.bind_variable_link(&binding, target)
    }

    /// Install a link without discarding observable local state. Tcl reuses an
    /// untraced undefined shell, and it may retarget an existing link, but a
    /// defined scalar/array or a trace-only shell is an `upvar` error.
    fn bind_link_var(&mut self, binding: &VarBinding, target: VarId) -> Result<(), UpvarLinkError> {
        self.bind_link_var_with_origin(binding, target, FrameLinkOrigin::Ordinary)
    }

    pub(in super::super) fn bind_variable_link(
        &mut self,
        binding: &VarBinding,
        target: VariableLinkTarget,
    ) -> Result<(), UpvarLinkError> {
        match target {
            VariableLinkTarget::Cell(target) => self.bind_link_var(binding, target),
            VariableLinkTarget::Name(target) => self.bind_named_link(binding, target),
        }
    }

    fn bind_named_link(
        &mut self,
        binding: &VarBinding,
        mut target: NameLink,
    ) -> Result<(), UpvarLinkError> {
        if self.uses_native_jim_lookup() && target.original_target.is_none() {
            let mut name = target.binding.name.as_bytes().to_vec();
            if let Some(element) = &target.element {
                name.push(b'(');
                name.extend_from_slice(element.as_bytes());
                name.push(b')');
            }
            let level = match target.binding.owner {
                VarTableOwner::Frame(level)
                | VarTableOwner::CompiledLocal { level, .. }
                | VarTableOwner::ProcedureStatics(level) => level,
                VarTableOwner::Namespace(_) => 0,
            };
            let frame = self
                .frames
                .get(level)
                .ok_or(UpvarLinkError::TargetNamespace)?;
            target.original_target = Some(crate::vars::JimNameLinkOriginal {
                name: Value::new_native_string_bytes(name),
                frame: frame.jim_storage.reference(),
            });
        }
        self.validate_link_binding(binding)?;
        let mut current = target.binding.clone();
        let mut visited = HashSet::new();
        while visited.insert(current.clone()) {
            if current == *binding {
                return Err(UpvarLinkError::SelfLink);
            }
            let raw = self.raw_variable_binding(&current);
            match raw
                .and_then(|raw| self.var_arena.get(raw))
                .map(crate::vars::VarCell::state)
            {
                Some(Local::NameLink(target)) => current = target.binding.clone(),
                _ => break,
            }
        }
        self.validate_link_binding(binding)?;
        if let Some(raw) = self.raw_variable_binding(binding) {
            let _ = self.var_arena.replace_state(raw, Local::NameLink(target));
            return Ok(());
        }
        self.bind_new_var(binding, Local::NameLink(target))
            .map(|_| ())
            .ok_or(UpvarLinkError::TargetNamespace)
    }

    fn bind_link_var_with_origin(
        &mut self,
        binding: &VarBinding,
        target: VarId,
        origin: FrameLinkOrigin,
    ) -> Result<(), UpvarLinkError> {
        if self.raw_variable_binding(binding) == Some(target) {
            return Err(UpvarLinkError::SelfLink);
        }
        self.validate_link_binding(binding)?;
        if self
            .raw_variable_binding(binding)
            .and_then(|id| self.var_arena.get(id))
            .is_some_and(
                |cell| matches!(cell.state(), Local::Link(previous) if *previous == target),
            )
        {
            return Ok(());
        }
        self.bind_new_link(binding, target, origin)
            .map(|_| ())
            .ok_or(UpvarLinkError::TargetNamespace)
    }

    fn validate_link_binding(&self, binding: &VarBinding) -> Result<(), UpvarLinkError> {
        if let Some(raw) = self.raw_variable_binding(binding) {
            match self.var_arena.get(raw).map(crate::vars::VarCell::state) {
                Some(Local::Link(_) | Local::NameLink(_)) => {}
                Some(Local::Undefined) => {
                    if self
                        .var_arena
                        .resolve(raw)
                        .is_some_and(|id| self.variable_observers.script_traces.contains_key(&id))
                    {
                        return Err(UpvarLinkError::Traced);
                    }
                }
                Some(Local::Scalar(_) | Local::Array(_)) => {
                    let traced = self
                        .var_arena
                        .resolve(raw)
                        .is_some_and(|id| self.variable_observers.script_traces.contains_key(&id));
                    return Err(if traced {
                        UpvarLinkError::Traced
                    } else {
                        UpvarLinkError::Exists
                    });
                }
                None => return Err(UpvarLinkError::Exists),
            }
        }
        Ok(())
    }
}
