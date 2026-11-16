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

//! Resolving a variable name: the table a name binds in (frame or namespace), following links to the cell, and creating the cell a write or link needs.

use super::super::namespace_context::NamespaceExecutionContext;
use super::super::{ResolvedVar, VarBinding, VarTableOwner, Vm, variable_cleanup};
use crate::vars::{VarState, VarState as Local, VarTable};
use std::collections::HashSet;
use tcl_core_types::NameBytes;
use tcl_runtime_api::{FrameLinkOrigin, Namespaces, NsId, ROOT_NS, VarId};

impl Vm {
    /// Tcl 8.x resolves an unqualified variable at **namespace scope** to the
    /// global variable when the namespace has none but the global namespace
    /// does — reads *and* writes reach the global; 9.0 removed the fallback
    /// (TIP 278, `TCL_NAMESPACE_ONLY` — 8.6 `tclVar.c:757` vs 9.0 `:935`).
    /// tclsh 8.6/9.0-pinned in `cross_version_vars_e2e.rs`.
    fn ns_var_global_fallback(&self) -> bool {
        self.runtime_version.namespace_var_global_fallback()
    }
}

impl Vm {
    /// The supported backend owns zero-seeded Jim tables; foreign tables must
    /// supply their retained seed through their separate carrier interface.
    pub(in super::super) fn variable_table_recipe(
        &self,
    ) -> Option<tcl_core_types::NativeHashRecipe> {
        let abi = tcl_runtime_api::native_hash_abi::supported_backend_hash_abi(Some(0))?;
        let policy = self.name_policy_protocol()?;
        let dialect = self.actual_native_invocation_dialect();
        let protocol = match policy.authority() {
            tcl_syntax::naming::NamePolicyAuthority::Native => {
                dialect.native_variable_table_protocol(abi)?
            }
            tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation => self
                .native_invocation_dialect()
                .authored_variable_table_protocol(policy, abi)?,
        };
        Some(protocol.recipe())
    }

    pub(in super::super) fn var_table(&self, owner: VarTableOwner) -> Option<&VarTable> {
        let recipe = self.variable_table_recipe();
        self.var_arena.select_hash_recipe(recipe);
        let table = match owner {
            VarTableOwner::CompiledLocal { .. } => None,
            VarTableOwner::Frame(level) => self.frames.get(level).map(|frame| &frame.locals),
            VarTableOwner::ProcedureStatics(level) => self
                .frames
                .get(level)?
                .statics
                .as_ref()
                .map(|statics| &statics.table),
            VarTableOwner::Namespace(ROOT_NS) => self.frames.first().map(|frame| &frame.locals),
            VarTableOwner::Namespace(id) => self.ns_vars.get(&id),
        }?;
        table.select_hash_recipe(recipe);
        Some(table)
    }

    pub(in super::super) fn var_table_mut(
        &mut self,
        owner: VarTableOwner,
    ) -> Option<&mut VarTable> {
        let recipe = self.variable_table_recipe();
        self.var_arena.select_hash_recipe(recipe);
        let table = match owner {
            VarTableOwner::ProcedureStatics(_) | VarTableOwner::CompiledLocal { .. } => None,
            VarTableOwner::Frame(level) => {
                self.frames.get_mut(level).map(|frame| &mut frame.locals)
            }
            VarTableOwner::Namespace(ROOT_NS) => {
                self.frames.first_mut().map(|frame| &mut frame.locals)
            }
            VarTableOwner::Namespace(id) => self.ns_vars.get_mut(&id),
        }?;
        table.select_hash_recipe(recipe);
        Some(table)
    }

    pub(in super::super) fn jim_variable_lookup(&self) -> bool {
        self.dialect_profile().variable_lookup_policy()
            == Some(tcl_dialect::VariableLookupPolicy::Jim)
    }

    pub(crate) fn variable_alias_frame(&self) -> tcl_registry::VariableAliasFrame {
        if self.current_level() == 0 {
            tcl_registry::VariableAliasFrame::Global
        } else if self
            .frames
            .get(self.current_level())
            .is_some_and(|frame| frame.proc_name.is_some())
        {
            tcl_registry::VariableAliasFrame::Procedure
        } else {
            tcl_registry::VariableAliasFrame::Namespace
        }
    }

    pub(crate) fn frame_owns_local_variables(&self, level: usize) -> bool {
        level > 0
            && (self.jim_variable_lookup()
                || self
                    .frames
                    .get(level)
                    .is_some_and(|frame| frame.proc_name.is_some()))
    }

    /// Resolve a written base name to the frame or namespace-token table that
    /// owns its binding. No cell is created here.
    pub(super) fn var_binding_from(&self, name: &str, start: usize) -> Option<VarBinding> {
        self.var_binding_from_bytes(name.as_bytes(), start)
    }

    fn jim_var_binding_from_bytes(
        &self,
        name: &[u8],
        start: usize,
        qualification: tcl_syntax::naming::NativeNameQualification,
    ) -> VarBinding {
        let global =
            qualification == tcl_syntax::naming::NativeNameQualification::Absolute || start == 0;
        let global_key =
            global.then(|| tcl_syntax::naming::jim_global_variable_key_bytes(b"::", name));
        let selected = global_key.as_deref().unwrap_or(name);
        let owner = if global {
            VarTableOwner::Namespace(ROOT_NS)
        } else if self.frames.get(start).is_some_and(|frame| {
            !frame.locals.contains_key(selected)
                && frame
                    .statics
                    .as_ref()
                    .is_some_and(|statics| statics.table.contains_key(selected))
        }) {
            VarTableOwner::ProcedureStatics(start)
        } else {
            VarTableOwner::Frame(start)
        };
        VarBinding {
            owner,
            name: NameBytes::from(selected),
        }
    }

    pub(in super::super) fn var_binding_from_bytes(
        &self,
        original: &[u8],
        start: usize,
    ) -> Option<VarBinding> {
        if self.observed_names.is_some() {
            return None;
        }

        let protocol = self.name_policy_protocol()?.recipe();
        let projection = protocol.variable_root_input(original);
        let name = projection.selected();
        if protocol.is_jim084() {
            return Some(self.jim_var_binding_from_bytes(name, start, projection.qualification()));
        }
        if let Some((namespace, storage)) =
            crate::cmd_oo::native_variables::runtime_target(self, start, protocol, name).ok()?
        {
            return Some(VarBinding {
                owner: VarTableOwner::Namespace(namespace),
                name: storage,
            });
        }
        if projection.qualification() != tcl_syntax::naming::NativeNameQualification::Unqualified {
            return self.qualified_var_binding_from_bytes(name, start, projection.qualification());
        }
        let owner = if start == 0 {
            VarTableOwner::Namespace(ROOT_NS)
        } else {
            let frame = self.frames.get(start)?;
            if frame.locals.contains_key(name) {
                self.compiled_local_dynamic_slot(start, name)
                    .map_or(VarTableOwner::Frame(start), |slot| {
                        VarTableOwner::CompiledLocal { level: start, slot }
                    })
            } else if frame.ns_eval.is_some() {
                let token = self
                    .resolution_stacks
                    .ns_id_stack
                    .get(start)
                    .and_then(NamespaceExecutionContext::resolved_token)?;
                let namespace = tcl_syntax::naming::variable_namespace_lookup_bases(
                    false,
                    token,
                    self.ns_var_global_fallback(),
                )
                .find_map(|base| {
                    let candidate = match base {
                        tcl_syntax::naming::VariableLookupBase::Context(namespace) => namespace,
                        tcl_syntax::naming::VariableLookupBase::Root => ROOT_NS,
                    };
                    self.var_table(VarTableOwner::Namespace(candidate))
                        .is_some_and(|table| table.contains_key(name))
                        .then_some(candidate)
                })
                .unwrap_or(token);
                VarTableOwner::Namespace(namespace)
            } else {
                self.compiled_local_dynamic_slot(start, name)
                    .map_or(VarTableOwner::Frame(start), |slot| {
                        VarTableOwner::CompiledLocal { level: start, slot }
                    })
            }
        };
        Some(VarBinding {
            owner,
            name: NameBytes::from(name),
        })
    }

    fn qualified_var_binding_from_bytes(
        &self,
        name: &[u8],
        start: usize,
        qualification: tcl_syntax::naming::NativeNameQualification,
    ) -> Option<VarBinding> {
        let absolute = qualification == tcl_syntax::naming::NativeNameQualification::Absolute;
        let protocol = self.name_policy_protocol()?.recipe();
        let tcl_syntax::naming::NativeVariableRootGeometry::CNamespace {
            namespace: qualifiers,
            simple,
        } = protocol.variable_root_geometry(tcl_syntax::naming::NativeNameContext::root(), name)
        else {
            return None;
        };
        let context = if absolute {
            ROOT_NS
        } else {
            self.resolution_stacks
                .ns_id_stack
                .get(start)
                .and_then(NamespaceExecutionContext::resolved_token)?
        };
        let walk = |base| {
            let token = match base {
                tcl_syntax::naming::VariableLookupBase::Context(namespace) => namespace,
                tcl_syntax::naming::VariableLookupBase::Root => ROOT_NS,
            };
            self.namespace_descendant_token(token, qualifiers.as_segments())
        };
        let mut bases = tcl_syntax::naming::variable_namespace_lookup_bases(
            absolute,
            context,
            self.ns_var_global_fallback(),
        );
        let selected = walk(bases.next()?);
        if selected.is_none_or(|token| {
            self.var_table(VarTableOwner::Namespace(token))
                .is_none_or(|table| !table.contains_key(simple.as_bytes()))
        }) {
            for base in bases {
                let Some(token) = walk(base) else {
                    continue;
                };
                let alternate = VarBinding {
                    owner: VarTableOwner::Namespace(token),
                    name: simple.clone(),
                };
                if self.bound_var_id(&alternate).is_some() {
                    return Some(alternate);
                }
            }
        }
        Some(VarBinding {
            owner: VarTableOwner::Namespace(selected?),
            name: simple,
        })
    }

    #[cfg(test)]
    pub(in super::super) fn namespace_child_token(
        &self,
        parent: NsId,
        simple: impl AsRef<[u8]>,
    ) -> Option<NsId> {
        self.name_world
            .borrow()
            .namespace_child_token(parent, simple.as_ref())
    }

    pub(in super::super) fn namespace_var_binding_from_token_bytes(
        &self,
        namespace: NsId,
        original: &[u8],
    ) -> Option<VarBinding> {
        if self.observed_names.is_some() {
            return None;
        }

        let protocol = self.name_policy_protocol()?.recipe();
        let projection = protocol.variable_root_input(original);
        let name = projection.selected();
        if protocol.is_jim084() {
            let context = Namespaces::name_bytes(self, namespace);
            let global = tcl_syntax::naming::jim_global_variable_key_bytes(&context, name);
            return Some(VarBinding {
                owner: VarTableOwner::Namespace(ROOT_NS),
                name: NameBytes::from(global),
            });
        }
        match protocol.variable_root_geometry(tcl_syntax::naming::NativeNameContext::root(), name) {
            tcl_syntax::naming::NativeVariableRootGeometry::Local(simple) => Some(VarBinding {
                owner: VarTableOwner::Namespace(namespace),
                name: simple,
            }),
            tcl_syntax::naming::NativeVariableRootGeometry::CNamespace {
                namespace: qualifiers,
                simple,
            } => {
                let base = if projection.qualification()
                    == tcl_syntax::naming::NativeNameQualification::Absolute
                {
                    ROOT_NS
                } else {
                    namespace
                };
                Some(VarBinding {
                    owner: VarTableOwner::Namespace(
                        self.namespace_descendant_token(base, qualifiers.as_segments())?,
                    ),
                    name: simple,
                })
            }
            tcl_syntax::naming::NativeVariableRootGeometry::JimAbsolute(_) => None,
        }
    }

    pub(super) fn bound_var_id(&self, binding: &VarBinding) -> Option<VarId> {
        let raw = self.raw_variable_binding(binding)?;
        if !matches!(
            self.var_arena.get(raw).map(crate::vars::VarCell::state),
            Some(Local::NameLink(_))
        ) {
            return self.var_arena.resolve(raw);
        }
        self.resolve_binding_var(binding.clone(), None, &mut HashSet::new())?
            .id
    }

    pub(in super::super) fn resolve_var_from(
        &self,
        name: &str,
        start: usize,
    ) -> Option<ResolvedVar> {
        self.resolve_var_from_bytes(name.as_bytes(), start)
    }

    pub(in super::super) fn resolve_var_from_bytes(
        &self,
        original: &[u8],
        start: usize,
    ) -> Option<ResolvedVar> {
        if self.observed_names.is_some() {
            return self.observed_resolved_variable(original, start).ok();
        }
        let (binding, element) = self.select_var_from_bytes(original, start)?;
        self.resolve_binding_var(binding, element, &mut HashSet::new())
    }

    pub(super) fn select_var_from_bytes(
        &self,
        original: &[u8],
        start: usize,
    ) -> Option<(VarBinding, Option<NameBytes>)> {
        let projection = self
            .execution_name_policy()?
            .variable_input(
                tcl_syntax::naming::NativeVariableInputForm::Combined(original),
                tcl_syntax::naming::ObservedVariableNamePurpose::ScalarReceiver,
            )
            .ok()?;
        let binding = self.var_binding_from_bytes(projection.root(), start)?;
        let element = projection.element().map(tcl_core_types::NameBytes::from);
        Some((binding, element))
    }

    /// Follow each dialect's alias representation in the same storage resolver.
    /// Name links select their original activation on every access; a deleted
    /// array or namespace can therefore acquire a new same-name target without
    /// reviving C's detached variable cells.
    pub(in super::super) fn resolve_var_parts_from_bytes(
        &self,
        root: &[u8],
        element: Option<&[u8]>,
        start: usize,
    ) -> Option<ResolvedVar> {
        let (binding, element) = self.select_var_parts_from_bytes(root, element, start)?;
        self.resolve_binding_var(binding, element, &mut HashSet::new())
    }

    pub(super) fn select_var_parts_from_bytes(
        &self,
        root: &[u8],
        element: Option<&[u8]>,
        start: usize,
    ) -> Option<(VarBinding, Option<NameBytes>)> {
        let parts = self
            .name_policy_protocol()?
            .recipe()
            .separate_variable_input(root, element);
        let binding = self.var_binding_from_bytes(parts.root().selected(), start)?;
        Some((
            binding,
            parts.element().map(|part| NameBytes::from(part.selected())),
        ))
    }

    pub(in super::super) fn resolve_binding_var(
        &self,
        mut binding: VarBinding,
        mut elem: Option<tcl_core_types::NameBytes>,
        visited: &mut HashSet<VarBinding>,
    ) -> Option<ResolvedVar> {
        let raw = loop {
            if let VarTableOwner::Frame(level) = binding.owner
                && let Some(slot) = self.compiled_local_dynamic_slot(level, binding.name.as_bytes())
            {
                binding.owner = VarTableOwner::CompiledLocal { level, slot };
            }
            let raw = self.raw_variable_binding(&binding);
            let Some(Local::NameLink(target)) = raw
                .and_then(|id| self.var_arena.get(id))
                .map(crate::vars::VarCell::state)
            else {
                break raw;
            };
            if !visited.insert(binding) {
                return None;
            }
            // An element alias is already a scalar access, so indexing it
            // again cannot address an element of the containing array.
            if elem.is_some() && target.element.is_some() {
                return None;
            }
            binding = target.binding.clone();
            elem = elem.or_else(|| target.element.clone());
        };
        let base_id = raw.and_then(|id| self.var_arena.resolve(id));
        let id = match (base_id, elem.as_deref()) {
            (Some(base_id), Some(key)) => match self.var_arena.get(base_id)?.state() {
                VarState::Array(elements) => elements
                    .get(key)
                    .copied()
                    .and_then(|id| self.var_arena.resolve(id)),
                _ => None,
            },
            (id, None) => id,
            (None, Some(_)) => None,
        };
        Some(ResolvedVar {
            binding,
            base_id,
            id,
            elem,
        })
    }

    pub(in super::super) fn bind_new_var(
        &mut self,
        binding: &VarBinding,
        state: VarState,
    ) -> Option<VarId> {
        let id = self.var_arena.alloc(state)?;
        self.install_new_var(binding, id);
        Some(self.raw_variable_binding(binding).unwrap_or(id))
    }

    pub(super) fn bind_new_link(
        &mut self,
        binding: &VarBinding,
        target: VarId,
        origin: FrameLinkOrigin,
    ) -> Option<VarId> {
        let id = self.var_arena.alloc_link(target, origin)?;
        self.install_new_var(binding, id);
        Some(self.raw_variable_binding(binding).unwrap_or(id))
    }

    fn install_new_var(&mut self, binding: &VarBinding, id: VarId) {
        let compiled = match binding.owner {
            VarTableOwner::CompiledLocal { level, slot } => Some((level, slot)),
            VarTableOwner::Frame(level) => self
                .compiled_local_dynamic_slot(level, binding.name.as_bytes())
                .map(|slot| (level, slot)),
            _ => None,
        };
        if let Some((level, slot)) = compiled {
            let original = self.frames[level].compiled_locals[slot].1;
            assert!(
                self.var_arena.transfer_unbound_state(id, original),
                "new variable state"
            );
            self.var_arena.discard_unbound(id);
            return;
        }
        if let VarTableOwner::Namespace(namespace) = binding.owner {
            self.var_arena.set_namespace_owner(id, namespace);
        }
        let old = self
            .var_table_mut(binding.owner)
            .expect("resolved variable owner has a table")
            .insert(binding.name.clone(), id);
        self.var_arena.bind(id);
        if let Some(old) = old {
            let original = variable_cleanup::OriginalVariableCleanup::enter(self);
            original.purge_binding(self, old);
        }
    }

    pub(in super::super) fn ensure_base_var_from(
        &mut self,
        name: &str,
        start: usize,
    ) -> Option<(VarBinding, VarId)> {
        self.ensure_base_var_from_bytes(name.as_bytes(), start)
    }

    pub(in super::super) fn ensure_base_var_from_bytes(
        &mut self,
        name: &[u8],
        start: usize,
    ) -> Option<(VarBinding, VarId)> {
        let resolved = self.resolve_var_from_bytes(name, start)?;
        if resolved.elem.is_some() {
            return None;
        }
        let binding = resolved.binding;
        let id = resolved
            .id
            .or_else(|| self.bind_new_var(&binding, VarState::Undefined))?;
        Some((binding, id))
    }

    /// Resolve an upvar target once and materialise its undefined cell. An
    /// element target creates the containing array and its own stable element
    /// cell, just as Tcl's `MakeUpvar` does.
    pub(super) fn ensure_target_var_from(&mut self, name: &str, start: usize) -> Option<VarId> {
        self.ensure_target_var_from_bytes(name.as_bytes(), start)
    }

    pub(in super::super) fn ensure_target_var_from_bytes(
        &mut self,
        original: &[u8],
        start: usize,
    ) -> Option<VarId> {
        let projection = self
            .name_policy_protocol()?
            .recipe()
            .combined_variable_input(original);
        let binding = self.var_binding_from_bytes(projection.root().selected(), start)?;
        self.ensure_target_var_at_binding(
            &binding,
            projection
                .element()
                .map(tcl_syntax::naming::NativeNameProjection::selected),
        )
    }

    pub(in super::super) fn ensure_target_var_at_binding(
        &mut self,
        binding: &VarBinding,
        element: Option<&[u8]>,
    ) -> Option<VarId> {
        let base_id = self
            .bound_var_id(binding)
            .or_else(|| self.bind_new_var(binding, VarState::Undefined))?;
        let Some(key) = element else {
            return Some(base_id);
        };
        match self.var_arena.get(base_id).map(crate::vars::VarCell::state) {
            Some(Local::Undefined) => {
                if !self
                    .var_arena
                    .replace_state(base_id, Local::Array(VarTable::new()))
                {
                    return None;
                }
            }
            Some(Local::Array(_)) => {}
            Some(Local::Scalar(_) | Local::Link(_) | Local::NameLink(_)) | None => return None,
        }
        if let Some(id) = match self.var_arena.get(base_id).map(crate::vars::VarCell::state) {
            Some(Local::Array(elements)) => elements.get(key).copied(),
            _ => None,
        } {
            return self.var_arena.resolve(id);
        }
        let element_id = self.var_arena.alloc_element(
            Local::Undefined,
            base_id,
            tcl_core_types::NameBytes::from(key),
        )?;
        let inserted =
            self.var_arena
                .array_insert(base_id, tcl_core_types::NameBytes::from(key), element_id);
        if inserted {
            self.var_arena.bind(element_id);
            Some(element_id)
        } else {
            self.var_arena.discard_unbound(element_id);
            None
        }
    }
}
