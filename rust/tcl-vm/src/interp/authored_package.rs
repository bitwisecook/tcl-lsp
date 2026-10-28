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

    #[cfg(test)]
    pub(crate) fn with_package_table<R>(
        &mut self,
        authored: bool,
        callback: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let entered = self.enter_package_table(authored);
        let result = callback(self);
        self.in_interp(entered.interpreter, |vm| vm.restore_package_table(&entered));
        result
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
    use std::rc::Rc;
    use tcl_runtime_api::Code;

    fn host() -> Vm {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .expect("actual host core");
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        let _ = vm.set_command_surface_profile(profile);
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert!(vm.set_logical_package_provider(Some(AuthoredPackageProvider::Tcl84Core)));
        vm
    }

    fn package(vm: &mut Vm, words: &[&str]) -> Completion<Value> {
        let mut argv = vec![Value::string("package")];
        argv.extend(words.iter().map(|word| Value::string(*word)));
        crate::cmd_package::cmd_authored_package(vm, &argv)
    }

    #[track_caller]
    fn text(completion: Completion<Value>) -> String {
        assert_eq!(completion.code, Code::Ok, "{completion:?}");
        let text = completion
            .result
            .try_to_str()
            .expect("ASCII package control")
            .to_string();
        drop(completion);
        text
    }

    #[test]
    fn original_authored_namespace_selector_keeps_table_members_and_strict_utf8_refusal() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = host();
        let original = Value::string("ex");
        let table = Value::list(vec![Value::string("exists"), Value::string("eval")]);
        let reached = namespace_member(&mut vm, &[original, table]);
        assert_eq!(reached.code, Code::Ok);
        assert_eq!(reached.result.string_bytes().as_ref(), b"exists");
        assert!(!vm.native_operation_context.has_entered_operation());
        let raw = Value::new_native_string_bytes(b"\xff".as_slice());
        let table = Value::list(vec![raw]);
        assert_eq!(
            namespace_member(&mut vm, &[Value::string("x"), table]).code,
            Code::Error
        );
        assert!(vm.execution_refusal.is_some());
        assert!(!vm.native_operation_context.has_entered_operation());
    }

    #[test]
    fn authored_package_members_mutate_only_the_isolated_table() {
        let mut vm = host();
        assert_eq!(text(package(&mut vm, &["prov", "Tcl"])), "8.4");
        assert_eq!(
            text(package(&mut vm, &["req", "-exact", "Tcl", "8.4"])),
            "8.4"
        );
        assert!(
            vm.package_version_bytes("Tcl")
                .unwrap()
                .as_bytes()
                .starts_with(b"9.0")
        );
        assert_eq!(text(package(&mut vm, &["provi", "Demo", "1.2"])), "");
        assert_eq!(text(package(&mut vm, &["pres", "Demo"])), "1.2");
        assert!(vm.package_version_bytes("Demo").is_none());
        assert_eq!(text(package(&mut vm, &["forget", "Tcl"])), "");
        assert_eq!(text(package(&mut vm, &["provide", "Tcl"])), "");
        assert!(
            vm.package_version_bytes("Tcl")
                .unwrap()
                .as_bytes()
                .starts_with(b"9.0")
        );
        let ambiguous = package(&mut vm, &["pr"]);
        assert_eq!(ambiguous.code, Code::Error);
        assert!(
            ambiguous
                .result
                .try_to_str()
                .unwrap()
                .contains("ambiguous option")
        );
        let arity = package(&mut vm, &["vsat", "1.2", "1.0", "2.0"]);
        assert_eq!(arity.code, Code::Error);
        assert_eq!(
            arity.result.try_to_str().unwrap().as_ref(),
            "wrong # args: should be \"package vsatisfies version1 version2\""
        );
    }

    #[test]
    fn authored_package_loader_requires_independent_script_dispatch_provider() {
        let mut vm = host();
        assert_eq!(
            text(package(
                &mut vm,
                &[
                    "ifneeded",
                    "Demo",
                    "1.2",
                    "::tmm::_logical_package package provide Demo 1.2"
                ]
            )),
            ""
        );
        let completion = package(&mut vm, &["require", "Demo"]);
        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            vm.execution_refusal.as_ref().unwrap().to_string(),
            "native script-object dispatch protocol is unavailable"
        );
        assert!(vm.package_version_bytes("Demo").is_none());
        vm.with_package_table(true, |vm| {
            assert!(vm.package_version_bytes("Demo").is_none());
        });
    }

    #[test]
    fn authored_package_loaders_children_and_host_activation_are_separate() {
        let mut vm = host();
        assert!(vm.set_logical_eval_object_provider(
            tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation
        ));
        assert_eq!(
            text(package(
                &mut vm,
                &[
                    "ifn",
                    "Demo",
                    "1.2",
                    "::tmm::_logical_package package provide Demo 1.2"
                ]
            )),
            ""
        );
        assert_eq!(text(package(&mut vm, &["versions", "Demo"])), "1.2");
        assert_eq!(text(package(&mut vm, &["req", "Demo"])), "1.2");
        assert!(vm.package_version_bytes("Demo").is_none());
        assert_eq!(
            text(package(
                &mut vm,
                &["unknown", "::tmm::_logical_package package provide"]
            )),
            ""
        );
        assert_eq!(
            text(package(&mut vm, &["unknown"])),
            "::tmm::_logical_package package provide"
        );
        let native = vm
            .try_eval_native_host_source("::tmm::_logical_package package provide Tcl")
            .expect("native host activation");
        assert!(text(native).starts_with("9.0"));
        assert_eq!(text(package(&mut vm, &["provide", "Tcl"])), "8.4");
        vm.create_child(Some("worker".into()), false);
        let child = vm.resolve_interp_path("worker").expect("actual child");
        vm.in_interp(child, |vm| {
            assert_eq!(text(package(vm, &["provide", "Tcl"])), "8.4");
            assert_eq!(text(package(vm, &["provide", "Demo"])), "");
            assert_eq!(text(package(vm, &["unknown"])), "");
        });
        vm.with_package_table(true, |vm| assert!(!vm.set_logical_package_provider(None)));
        assert!(vm.set_logical_package_provider(None));
        assert!(vm.authored_package_provider().is_none());
    }

    #[test]
    fn authored_package_installation_requires_an_explicit_host_and_name_provider() {
        let mut vm = Vm::default();
        assert!(!vm.set_logical_package_provider(Some(AuthoredPackageProvider::Tcl84Core)));
        assert!(vm.authored_packages.is_none());
    }
}
