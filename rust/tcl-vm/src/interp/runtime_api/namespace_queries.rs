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

//! `Namespaces` and `Aliases` over the structured namespace world: finding commands and namespaces, children, and the commands, procedures and variables of a namespace.

use super::super::commands::CommandSidecarKey;
use super::super::namespace_context::NamespaceExecutionContext;
use super::super::namespaces::{
    NativeNamespaceAddress, NativeNamespaceAddressError, retained_children_of,
};
use super::super::{VarTableOwner, Vm, native_operation_currency, oo_cleanup};
use crate::command::Command;
use crate::value::Value;
use crate::vars::VarState as Local;
use tcl_runtime_api::{CommandId, Namespaces, NsId, ROOT_NS};

/// Namespace name resolution over the VM's structured namespace arena.
/// [`current`](Namespaces::current) returns the interned id of the current
/// namespace (interned when pushed). [`find_command`](Namespaces::find_command)
/// resolves `name` from `cxt` to its command key and interns that to a stable
/// `CommandId`. Note: the handle is produced for command *identity* only —
/// nothing dispatches by it yet (the `Commands` trait dispatches by name), the
/// open `find_command`/`CommandId` consumer question.
impl tcl_runtime_api::Aliases for Vm {
    type OriginalAliasError = tcl_cmd_core::CmdError;
    fn alias_prefix_original_value(
        &mut self,
        original_name: &Self::Value,
    ) -> Result<tcl_runtime_api::AliasPrefixLookup<Self::Value>, Self::OriginalAliasError> {
        self.alias_prefix_original_value_at(self.current_ns_id(), original_name)
    }

    fn alias_prefix_original_value_at(
        &mut self,
        context: NsId,
        original_name: &Self::Value,
    ) -> Result<tcl_runtime_api::AliasPrefixLookup<Self::Value>, Self::OriginalAliasError> {
        let operation = native_operation_currency::NativeOperationScope::enter(self)?;
        let result: Result<_, tcl_cmd_core::CmdError> = (|| {
            if !self.uses_native_jim_lookup() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original Jim alias prefix issuer",
                )
                .into());
            }
            let selected = self.lookup_original_command_at(context, original_name);
            operation.check(self)?;
            let Some((_, command)) = selected? else {
                return Ok(tcl_runtime_api::AliasPrefixLookup::MissingCommand);
            };
            match command {
                Command::CallerAlias(original) => {
                    original.check_native_header()?;
                    Ok(tcl_runtime_api::AliasPrefixLookup::Prefix(original))
                }
                Command::Alias { .. } => {
                    Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original Jim alias prefix object",
                    )
                    .into())
                }
                _ => Ok(tcl_runtime_api::AliasPrefixLookup::NotAlias),
            }
        })();
        if let Err(error) = &result
            && let Some(cause) = error.native_execution_refusal()
        {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
        }
        operation.check(self)?;
        result
    }
}

impl Namespaces for Vm {
    fn native_jim_command_inventory_result(
        &mut self,
        root: NsId,
        pattern: Option<&[u8]>,
        include_spaces: bool,
        kind: tcl_runtime_api::NativeJimCommandInventoryKind,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        self.original_jim_command_inventory_result(root, pattern, include_spaces, kind)
            .map_err(|error| {
                oo_cleanup::retain_error(self, &error);
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original Jim command inventory result",
                )
            })
    }
    type OriginalNameError = tcl_cmd_core::CmdError;
    fn namespace_import_binding(&self) -> Option<tcl_dialect::NamespaceImportBinding> {
        self.dialect_profile().namespace_import_binding()
    }

    fn command_alias_prefix_bytes_checked(
        &mut self,
        cmd: CommandId,
    ) -> Result<Option<Vec<Vec<u8>>>, Self::OriginalNameError> {
        self.original_alias_bytes_query(cmd)
    }
    fn variable_lookup_policy(&self) -> Option<tcl_dialect::VariableLookupPolicy> {
        self.name_policy_protocol()
            .map(tcl_syntax::naming::NamePolicyProtocol::variable_lookup_policy)
    }

    fn find_command(&self, cxt: NsId, name: &str) -> Option<CommandId> {
        self.find_command_bytes(cxt, name.as_bytes())
    }

    fn find_command_bytes_checked(
        &self,
        cxt: NsId,
        name: &[u8],
    ) -> Result<Option<CommandId>, tcl_syntax::value::ValueError> {
        let selected = self
            .resolve_command_bytes_checked(cxt, name, true)
            .map_err(|_| {
                tcl_syntax::value::ValueError::CommandProtocolUnavailable("command lookup")
            })?;
        Ok(selected
            .and_then(|key| self.visible_command_generation(&key))
            .map(|token| CommandId(self.intern_cmd(token))))
    }

    fn find_command_bytes(&self, cxt: NsId, name: &[u8]) -> Option<CommandId> {
        let key = self.resolve_command_from_token_bytes(cxt, name, true)?;
        let token = self.visible_command_generation(&key)?;
        Some(CommandId(self.intern_cmd(token)))
    }

    fn current(&self) -> NsId {
        self.resolution_stacks
            .ns_id_stack
            .last()
            .and_then(NamespaceExecutionContext::resolved_token)
            .expect("operation requires a materialized current namespace")
    }

    fn name(&self, ns: NsId) -> String {
        // The arena holds the canonical (unrooted) name; `namespace current`
        // reports the absolute form (`""` → `"::"`).
        tcl_syntax::naming::root_unrooted_key(&self.ns_name(ns))
    }

    fn namespace_report_parts_checked(
        &mut self,
        namespace: NsId,
    ) -> Result<tcl_runtime_api::NativeNamespaceReportParts, Self::OriginalNameError> {
        self.original_namespace_report_parts(namespace)
    }

    fn name_bytes(&self, ns: NsId) -> Vec<u8> {
        tcl_syntax::naming::root_unrooted_key_bytes(self.ns_name_bytes(ns).as_bytes())
    }

    fn command_name(&self, cmd: CommandId) -> Option<String> {
        String::from_utf8(self.command_name_bytes(cmd)?).ok()
    }

    fn command_name_bytes(&self, cmd: CommandId) -> Option<Vec<u8>> {
        Some(self.rooted_command_sidecar_display_bytes(&self.command_sidecar_key(cmd.0)?))
    }

    // Namespace-tree navigation over the arena. Every namespace is interned on
    // creation (`push_ns`/`declare_namespace`), so these are pure `&self`
    // lookups over explicit parent/child edges.
    fn find_namespace(&self, cxt: NsId, name: &str) -> Option<NsId> {
        self.find_namespace_bytes(cxt, name.as_bytes())
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
        let world = self.name_world.borrow();
        let holder = world.jim_root_namespace_object.as_ref().ok_or(
            ValueError::CommandProtocolUnavailable("Jim root command namespace object"),
        )?;
        if !world.is_live()
            || !context.is_live()
            || holder.native_object_identity() != context.empty_object().native_object_identity()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim root command context owner",
            ));
        }
        Ok(Some(ROOT_NS))
    }

    fn find_namespace_bytes_checked(
        &self,
        cxt: NsId,
        original: &[u8],
    ) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        if !self.name_world.borrow().is_live() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "namespace lookup",
            ));
        }
        let address = match self.namespace_address_from_native_bytes(cxt, original) {
            Ok(address) => address,
            Err(NativeNamespaceAddressError::Projection(
                tcl_syntax::naming::NameProjectionUnavailable::EmptyNamespaceOutsideRoot,
            )) => return Ok(None),
            Err(_) => {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "namespace lookup",
                ));
            }
        };
        Ok(match address {
            NativeNamespaceAddress::C { path, absolute } => self
                .namespace_token_at_path(&path, absolute)
                .filter(|namespace| self.native_namespace_token_has_owner(*namespace)),
            NativeNamespaceAddress::Jim { object, .. } => self
                .name_world
                .borrow()
                .jim_namespace_objects
                .iter()
                .find_map(|(token, actual)| (actual == &object).then_some(*token)),
        })
    }

    fn find_namespace_child_bytes_checked(
        &self,
        parent: NsId,
        member: &[u8],
    ) -> Result<Option<NsId>, tcl_syntax::value::ValueError> {
        let child = self
            .name_world
            .borrow()
            .namespace_child_token(parent, member);
        Ok(child.filter(|child| {
            self.native_namespace_token_has_owner(*child)
                && tcl_runtime_api::Namespaces::parent(self, *child) == Some(parent)
        }))
    }

    fn find_namespace_bytes(&self, cxt: NsId, original: &[u8]) -> Option<NsId> {
        match self
            .namespace_address_from_native_bytes(cxt, original)
            .ok()?
        {
            NativeNamespaceAddress::C { path, absolute } => self
                .namespace_token_at_path(&path, absolute)
                .filter(|namespace| self.native_namespace_token_has_owner(*namespace)),
            NativeNamespaceAddress::Jim { object, .. } => self
                .name_world
                .borrow()
                .jim_namespace_objects
                .iter()
                .find_map(|(token, actual)| (actual == &object).then_some(*token)),
        }
    }

    fn namespace_is_live(&self, ns: NsId) -> bool {
        !self.name_world.borrow().dead_namespaces.contains(&ns)
            && !self.name_world.borrow().dying_namespaces.contains(&ns)
    }

    fn parent(&self, ns: NsId) -> Option<NsId> {
        if let Some(token) = self.name_world.borrow().namespace_name_tokens.get(&ns) {
            return token
                .parent()
                .and_then(|parent| u32::try_from(parent).ok())
                .map(NsId);
        }
        self.name_world.borrow().physical_namespace_parent(ns)
    }

    fn children(&self, ns: NsId) -> Vec<NsId> {
        if let Some(record) = self.retained_record_of(ns) {
            return retained_children_of(
                &record,
                ns,
                &self.name_world.borrow().ns_parents,
                &self.name_world.borrow().ns_arena,
            );
        }
        self.child_namespace_ids(ns)
    }

    fn children_hash_order(&self, ns: NsId) -> Vec<NsId> {
        if let Some(record) = self.retained_record_of(ns) {
            return retained_children_of(
                &record,
                ns,
                &self.name_world.borrow().ns_parents,
                &self.name_world.borrow().ns_arena,
            );
        }
        self.child_namespace_ids_hash_order(ns)
    }

    // Command enumeration selects structured command slots by namespace token;
    // the returned values are presentation-only unqualified tails.
    fn commands_in(&self, ns: NsId) -> Vec<String> {
        self.commands_in_bytes(ns)
            .into_iter()
            .map(|name| {
                String::from_utf8(name)
                    .expect("Unicode command enumeration requires a checked name")
            })
            .collect()
    }

    fn aliases_in_bytes_checked(
        &self,
        ns: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        if !self.uses_native_jim_lookup() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim alias inventory issuer",
            ));
        }
        Ok(self.jim_command_table_report_names_filtered(ns, |command| {
            matches!(command, Command::CallerAlias(_))
        }))
    }

    fn commands_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        if self.uses_native_jim_lookup() {
            return self.jim_command_table_report_names(ns, false);
        }
        self.retained_names_directly_in(ns, false)
            .unwrap_or_else(|| self.names_directly_in_token(ns, false))
            .into_iter()
            .map(|name| name.as_bytes().to_vec())
            .collect()
    }

    fn procs_in(&self, ns: NsId) -> Vec<String> {
        self.procs_in_bytes(ns)
            .into_iter()
            .map(|name| {
                String::from_utf8(name)
                    .expect("Unicode procedure enumeration requires a checked name")
            })
            .collect()
    }

    fn procs_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        if self.uses_native_jim_lookup() {
            return self.jim_command_table_report_names(ns, true);
        }
        self.retained_names_directly_in(ns, true)
            .unwrap_or_else(|| self.names_directly_in_token(ns, true))
            .into_iter()
            .map(|name| name.as_bytes().to_vec())
            .collect()
    }

    fn vars_in(&self, ns: NsId) -> Vec<String> {
        self.vars_in_bytes(ns)
            .into_iter()
            .map(|name| {
                String::from_utf8(name)
                    .expect("byte-valued namespace variables require byte enumeration")
            })
            .collect()
    }

    fn vars_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.var_table(VarTableOwner::Namespace(ns))
            .map_or_else(Vec::new, |table| {
                table
                    .iter()
                    .filter(|(_, id)| {
                        self.var_arena.get(**id).is_some_and(|cell| {
                            !matches!(cell.state(), Local::Undefined) || cell.namespace_declared()
                        })
                    })
                    .map(|(name, _)| name.as_bytes().to_vec())
                    .collect()
            })
    }

    fn vars_in_bytes_checked(
        &self,
        ns: NsId,
    ) -> Result<Vec<Vec<u8>>, tcl_syntax::value::ValueError> {
        self.variable_table_recipe().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native namespace variable table inventory",
            ),
        )?;
        Ok(self.vars_in_bytes(ns))
    }

    fn consts_in(&self, ns: NsId) -> Vec<String> {
        self.consts_in_bytes(ns)
            .into_iter()
            .map(|name| {
                String::from_utf8(name)
                    .expect("byte-valued namespace constants require byte enumeration")
            })
            .collect()
    }

    fn consts_in_bytes(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.var_table(VarTableOwner::Namespace(ns))
            .map_or_else(Vec::new, |table| {
                table
                    .iter()
                    .filter(|(_, raw)| self.const_vars.contains(raw))
                    .map(|(name, _)| name.as_bytes().to_vec())
                    .collect()
            })
    }

    // `Tcl_FindNamespaceVar`'s single probe over the namespace token's table.
    //
    // A `Link` counts. C's exclusion is of `CompiledLocal`s — a *proc*-local
    // `upvar`/`global` alias, which lives in the proc's compiled frame and is
    // never in a namespace `varTable`. A namespace-scoped link is a different
    // animal: `namespace upvar :: x y` (or an `upvar` at the global level)
    // puts a real `VAR_LINK` cell in the namespace's own table, which
    // `Tcl_FindNamespaceVar` and `info vars` both see (tclsh 9.0.4:
    // `namespace which -variable y` → `::n::y`). Proc-locals cannot leak in
    // here regardless, because procedure locals live in frame tables.
    //
    fn namespace_variable_name_bytes_checked(
        &self,
        context: NsId,
        original: &[u8],
    ) -> Result<Option<Vec<u8>>, tcl_syntax::value::ValueError> {
        use tcl_syntax::{naming::NativeNameContext, value::ValueError};
        let policy = self
            .name_policy_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "namespace variable query issuer",
            ))?;
        let protocol = policy.recipe();
        let projected = protocol.namespace_variable_query_input(original);
        if protocol.is_jim084() {
            let path = self.ns_path(context);
            let world = self.name_world.borrow();
            let object = world.jim_namespace_objects.get(&context).ok_or(
                ValueError::CommandProtocolUnavailable("namespace variable query context"),
            )?;
            let canonical = protocol
                .jim_namespace_canonical_input(
                    NativeNameContext::with_jim_namespace(&path, object.as_bytes()),
                    projected.selected(),
                )
                .map_err(|_| {
                    ValueError::CommandProtocolUnavailable(
                        "namespace variable query canonicalisation",
                    )
                })?;
            let report = tcl_syntax::naming::root_unrooted_key_bytes(canonical.selected());
            return Ok(Some(report));
        }
        if !self.name_world.borrow().is_live() {
            return Err(ValueError::CommandProtocolUnavailable(
                "namespace variable query world",
            ));
        }
        let query = projected.variable_namespace_projection().map_err(|_| {
            ValueError::CommandProtocolUnavailable("namespace variable query purpose")
        })?;
        let bases = query
            .namespace_query_bases(context, &ROOT_NS)
            .map_err(|_| {
                ValueError::CommandProtocolUnavailable("namespace variable query lookup policy")
            })?;
        for base in bases {
            let context = match base {
                tcl_syntax::naming::VariableLookupBase::Context(context) => context,
                tcl_syntax::naming::VariableLookupBase::Root => ROOT_NS,
            };
            let Some(binding) =
                self.namespace_var_binding_from_token_bytes(context, projected.selected())
            else {
                continue;
            };
            let Some(raw) = self.raw_variable_binding(&binding) else {
                continue;
            };
            if self.var_arena.get(raw).is_none() {
                continue;
            }
            let VarTableOwner::Namespace(namespace) = binding.owner else {
                return Err(ValueError::CommandProtocolUnavailable(
                    "namespace variable query owner",
                ));
            };
            return Ok(Some(tcl_syntax::naming::rooted_member_key_bytes(
                &Namespaces::name_bytes(self, namespace),
                binding.name.as_bytes(),
            )));
        }
        Ok(None)
    }

    fn namespace_var_exists(&self, ns: NsId, simple: &str) -> bool {
        self.namespace_var_exists_bytes(ns, simple.as_bytes())
    }

    fn namespace_var_exists_bytes(&self, ns: NsId, simple: &[u8]) -> bool {
        let Some(binding) = self.namespace_var_binding_from_token_bytes(ns, simple) else {
            return false;
        };
        self.var_table(binding.owner)
            .and_then(|table| table.get(&binding.name))
            .is_some_and(|id| self.var_arena.get(*id).is_some())
    }

    fn command_origin(&self, cmd: CommandId) -> Option<CommandId> {
        let key = self.command_fqn(cmd.0)?;
        // The trait's `None` means "not an imported command", which is C's
        // `cmdPtr->deleteProc == DeleteImportedCmd` test — *not* "the walk
        // ended where it started". Comparing names cannot stand in for it: a
        // visible import whose chain ends at an equally-named hidden token
        // would compare equal while genuinely being an import. The presence of
        // a provenance record is the VM's exact spelling of C's predicate.
        self.visible_import_binding(&key)?;
        let origin = self.ultimate_import_origin(&CommandSidecarKey::visible(key));
        let token = self.command_token_identity(&origin)?.generation;
        Some(CommandId(self.intern_cmd(token)))
    }
}
