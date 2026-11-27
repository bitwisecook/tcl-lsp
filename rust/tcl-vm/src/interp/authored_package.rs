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

//! Isolated authored package tables and their scoped dispatch purpose.

use super::native_operation_currency::NativeOperationScope;
use super::{InterpId, PackagePrefer, PackageState, Vm, err, ok};
use crate::Value;
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_core_types::{Completion, NameBytes};
use tcl_registry::native_package::{AuthoredPackageProvider, NativePackageProtocol};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum PackageTablePurpose {
    #[default]
    Native,
    Authored,
}

#[derive(Clone)]
pub(crate) struct PackageTableReceipt {
    interpreter: InterpId,
    identity: Rc<()>,
    purpose: PackageTablePurpose,
    previous: PackageTablePurpose,
    tracks_files: bool,
}

impl PackageTableReceipt {
    fn selected<'a>(&self, vm: &'a Vm) -> Option<&'a PackageState> {
        let state = vm.st_of(self.interpreter)?;
        match self.purpose {
            PackageTablePurpose::Native => Some(&state.package_state),
            PackageTablePurpose::Authored => Some(&state.authored_packages.as_ref()?.table),
        }
    }

    fn owns_table(&self, vm: &Vm) -> bool {
        self.selected(vm)
            .is_some_and(|table| Rc::ptr_eq(&table.identity, &self.identity))
    }

    pub(crate) fn check(
        &self,
        vm: &mut Vm,
        original: &NativeOperationScope,
    ) -> Result<(), CmdError> {
        original.check(vm)?;
        if !vm.interp_is_current(self.interpreter)
            || vm.package_table_purpose != self.purpose
            || !self.owns_table(vm)
        {
            return Err(vm
                .retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(
                    NativeValueAccessRefusal::CommandProtocolUnavailable("original package table"),
                ))
                .into());
        }
        Ok(())
    }

    /// Mandatory cleanup targets only the retained physical interpreter and table.
    pub(crate) fn cleanup<R>(
        &self,
        vm: &mut Vm,
        original: &NativeOperationScope,
        callback: impl FnOnce(&mut Vm) -> R,
    ) -> Result<R, CmdError> {
        self.with_saved(vm, original, true, callback)
    }

    fn with_saved<R>(
        &self,
        vm: &mut Vm,
        original: &NativeOperationScope,
        restore_temporary: bool,
        callback: impl FnOnce(&mut Vm) -> R,
    ) -> Result<R, CmdError> {
        if !self.owns_table(vm) {
            return Err(vm
                .retain_tcl_host_failure(crate::error::TclHostFailure::ValueAccess(
                    NativeValueAccessRefusal::CommandProtocolUnavailable("saved package table"),
                ))
                .into());
        }
        vm.in_interp(self.interpreter, |vm| {
            let cleanup = NativeOperationScope::cleanup_saved(vm, original.currency())
                .map_err(CmdError::from_execution_refusal)?;
            let previous = vm.package_table_purpose;
            vm.package_table_purpose = self.purpose;
            let result = callback(vm);
            if restore_temporary {
                vm.package_table_purpose = previous;
            }
            drop(cleanup);
            Ok(result)
        })
    }

    pub(crate) fn end_loading(
        &self,
        vm: &mut Vm,
        original: &NativeOperationScope,
        name: &NameBytes,
        version: &NameBytes,
    ) -> Result<(), CmdError> {
        self.cleanup(vm, original, |vm| {
            vm.end_package_loading_using(name, version, self.tracks_files);
        })
    }
}

pub(super) struct AuthoredPackageState {
    pub(super) provider: AuthoredPackageProvider,
    pub(super) table: PackageState,
    pub(super) unknown: Option<Vec<u8>>,
    pub(super) prefer: PackagePrefer,
    pub(super) order: Vec<NameBytes>,
    active_depth: usize,
}

impl AuthoredPackageState {
    pub(super) fn new(provider: AuthoredPackageProvider) -> Self {
        let mut table = PackageState::default();
        table
            .packages
            .insert(NameBytes::from("Tcl"), NameBytes::from("8.4"));
        Self {
            provider,
            table,
            unknown: None,
            prefer: PackagePrefer::Stable,
            order: vec![NameBytes::from("Tcl")],
            active_depth: 0,
        }
    }
}

impl Vm {
    /// Install or withdraw an isolated authored Tcl84 package database.
    /// Actual host registration, database, compiler and object protocols are unchanged.
    /// Actual child interpreters inherit the capability with fresh independent tables.
    #[must_use]
    pub fn set_logical_package_provider(
        &mut self,
        provider: Option<AuthoredPackageProvider>,
    ) -> bool {
        if self
            .authored_packages
            .as_ref()
            .is_some_and(|state| state.active_depth != 0)
        {
            return false;
        }
        if provider.is_some()
            && (self.actual_engine_profile.is_none()
                || self.logical_providers.names
                    != Some(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
                        tcl_dialect::TclVersion::V8_4,
                    )))
        {
            return false;
        }
        if self.authored_packages.as_ref().map(|state| state.provider) == provider {
            return true;
        }
        self.authored_packages = provider.map(AuthoredPackageState::new);
        register_provider(self);
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        true
    }

    pub(crate) fn authored_package_provider(&self) -> Option<AuthoredPackageProvider> {
        if self.logical_providers.names
            != Some(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
                tcl_dialect::TclVersion::V8_4,
            ))
        {
            return None;
        }
        self.authored_packages.as_ref().map(|state| state.provider)
    }

    pub(crate) fn package_table_is_authored(&self) -> bool {
        matches!(self.package_table_purpose, PackageTablePurpose::Authored)
    }

    pub(super) fn selected_package_state(&self) -> &PackageState {
        if self.package_table_is_authored() {
            &self
                .authored_packages
                .as_ref()
                .expect("selected authored table")
                .table
        } else {
            &self.package_state
        }
    }

    pub(super) fn selected_package_state_mut(&mut self) -> &mut PackageState {
        if self.package_table_is_authored() {
            &mut self
                .authored_packages
                .as_mut()
                .expect("selected authored table")
                .table
        } else {
            &mut self.package_state
        }
    }

    pub(crate) fn selected_package_protocol(&self) -> Option<NativePackageProtocol> {
        if self.package_table_is_authored() {
            Some(self.authored_packages.as_ref()?.provider.grammar())
        } else {
            self.actual_native_invocation_dialect()
                .native_package_protocol()
        }
    }

    fn enter_package_table(&mut self, authored: bool) -> PackageTableReceipt {
        if authored {
            self.authored_packages
                .as_mut()
                .expect("selected authored table")
                .active_depth += 1;
        }
        let previous = self.package_table_purpose;
        self.package_table_purpose = if authored {
            PackageTablePurpose::Authored
        } else {
            PackageTablePurpose::Native
        };
        PackageTableReceipt {
            interpreter: self.cur_interp(),
            identity: Rc::clone(&self.selected_package_state().identity),
            purpose: self.package_table_purpose,
            previous,
            tracks_files: self
                .selected_package_protocol()
                .is_some_and(NativePackageProtocol::tracks_files),
        }
    }

    fn restore_package_table(&mut self, entered: &PackageTableReceipt) {
        self.package_table_purpose = entered.previous;
        if entered.purpose == PackageTablePurpose::Authored {
            self.authored_packages
                .as_mut()
                .expect("entered authored table retained")
                .active_depth -= 1;
        }
    }

    pub(crate) fn with_package_table_checked<R>(
        &mut self,
        authored: bool,
        original: &NativeOperationScope,
        callback: impl FnOnce(&mut Self, &PackageTableReceipt) -> R,
    ) -> Result<R, CmdError> {
        original.check(self)?;
        let entered = self.enter_package_table(authored);
        let result = callback(self, &entered);
        entered.with_saved(self, original, false, |vm| {
            vm.restore_package_table(&entered);
        })?;
        original.check(self)?;
        Ok(result)
    }
}

pub(super) fn register_provider(vm: &mut Vm) {
    vm.register(
        "::tmm::_logical_package",
        crate::cmd_package::cmd_authored_package,
    );
    vm.register("::tmm::_logical_namespace_member", namespace_member);
}

fn namespace_member(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    super::original_authored_operands::with_authored_logical_operands(vm, |vm, operands| {
        Ok(namespace_member_in_original(vm, args, operands))
    })
}

fn namespace_member_in_original(
    vm: &mut Vm,
    args: &[Value],
    operands: &super::original_authored_operands::OriginalAuthoredOperandScope,
) -> Completion<Value> {
    if vm.authored_package_provider().is_none() {
        return vm.refuse_host_command("authored core keyword provider is unavailable".into());
    }
    let [original, table] = args else {
        return err("namespace member requires original word and table");
    };
    let original = match operands.string(vm, original) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let members = match operands.list(vm, table) {
        Ok(members) => members,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let names = members
        .iter()
        .map(|member| operands.unicode(vm, member))
        .collect::<Result<Vec<_>, _>>();
    let names = match names {
        Ok(names) => names,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let words = names.iter().map(AsRef::as_ref).collect::<Vec<&str>>();
    match tcl_registry::native_package::select_authored_keyword(&original, &words) {
        Ok(index) => ok(Value::from_native_string_bytes(words[index].as_bytes())),
        Err(message) => crate::command::completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::new_bytes(message),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_package_installation_requires_an_explicit_host_and_name_provider() {
        let mut vm = Vm::default();
        assert!(!vm.set_logical_package_provider(Some(AuthoredPackageProvider::Tcl84Core)));
        assert!(vm.authored_packages.is_none());
    }
}
