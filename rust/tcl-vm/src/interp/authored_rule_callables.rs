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

//! Explicit rule-owned callables with a separate root execution namespace.

use super::{Vm, err, ok};
use crate::{Value, command::Command};
use tcl_core_types::Completion;
use tcl_registry::f5::rule_identity::{RuleIdentity, RuleProcedureTarget};

/// Explicit authored callable and counted-string capabilities, independently installed.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AuthoredRuleProviders {
    pub(super) callables: bool,
    pub(super) counted_strings: bool,
}

impl Vm {
    /// Install authored rule ownership measured on BIG-IP 21.1.0.1 build 0.0.26.
    /// Native declaration, body compiler and object protocols remain physical.
    /// Actual child interpreters inherit this capability with separate callables.
    #[must_use]
    pub fn install_irules_rule_callable_simulation(&mut self) -> bool {
        if self.actual_engine_profile.is_none()
            || tcl_registry::native_procedure::procedure_activation_protocol(
                self.actual_native_invocation_dialect(),
            )
            .is_none()
            || self.logical_providers.names
                != Some(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
                    tcl_dialect::TclVersion::V8_4,
                ))
        {
            return false;
        }
        if !self.authored_rule_providers.callables {
            self.authored_rule_providers.callables = true;
            register_provider(self);
            self.bump_cmd_epoch();
            self.profile_generation = self.profile_generation.wrapping_add(1);
            self.eval_cache.clear();
            self.eval_cache_plain.clear();
            self.module_procs.clear();
        }
        true
    }
    /// Install authored counted-byte string length for measured TMM source/value cases.
    /// This does not select a native `StringLength` compiler, header or Unicode index.
    #[must_use]
    pub fn install_irules_counted_string_simulation(&mut self) -> bool {
        if self.actual_engine_profile.is_none()
            || self.logical_providers.names
                != Some(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
                    tcl_dialect::TclVersion::V8_4,
                ))
        {
            return false;
        }
        if !self.authored_rule_providers.counted_strings {
            self.authored_rule_providers.counted_strings = true;
            register_provider(self);
            self.bump_cmd_epoch();
            self.profile_generation = self.profile_generation.wrapping_add(1);
            self.eval_cache.clear();
            self.eval_cache_plain.clear();
            self.module_procs.clear();
        }
        true
    }
}

pub(super) fn register_provider(vm: &mut Vm) {
    if vm.authored_rule_providers.callables {
        vm.register("::tmm::_rule_declare", declare);
        vm.register("::tmm::_rule_target", target);
    }
    if vm.authored_rule_providers.counted_strings {
        vm.register("::tmm::_counted_string_length", counted_length);
        vm.register("::tmm::_logical_string_member", string_member);
    }
}

fn private_name(target: &RuleProcedureTarget) -> String {
    format!(
        "::tmm::_rule_callables::{}::{}",
        target.rule.as_path(),
        target.procedure
    )
}

fn selected_target(owner: &Value, target: &Value) -> Result<RuleProcedureTarget, String> {
    let owner = owner.try_to_str().map_err(|error| error.to_string())?;
    let target = target.try_to_str().map_err(|error| error.to_string())?;
    let owner = RuleIdentity::new(owner.to_string()).map_err(|error| error.to_string())?;
    RuleProcedureTarget::resolve(&target, Some(&owner)).map_err(|error| error.to_string())
}

fn declare(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !vm.authored_rule_providers.callables {
        return vm.refuse_host_command("authored rule callable provider is unavailable".into());
    }
    let [owner, name, parameters, body] = args else {
        return err("rule declaration requires owner, name, parameters and body");
    };
    let target = match selected_target(owner, name) {
        Ok(target) => target,
        Err(error) => return err(error),
    };
    let owner_text = match owner.try_to_str() {
        Ok(owner) => owner,
        Err(error) => return err(error.to_string()),
    };
    if target.rule.as_path() != owner_text.as_ref()
        || name.try_to_str().is_ok_and(|name| name.contains("::"))
    {
        return err("a rule procedure declaration requires an unqualified local name");
    }
    vm.declare_namespace(&format!(
        "::tmm::_rule_callables::{}",
        target.rule.as_path()
    ));
    let private = private_name(&target);
    crate::command::cmd_authored_rule_proc(
        vm,
        &[
            Value::string(private.as_str()),
            parameters.clone(),
            body.clone(),
        ],
    )
}

fn target(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !vm.authored_rule_providers.callables {
        return vm.refuse_host_command("authored rule callable provider is unavailable".into());
    }
    let [owner, name] = args else {
        return err("rule target requires owner and procedure");
    };
    let target = match selected_target(owner, name) {
        Ok(target) => target,
        Err(error) => return err(error),
    };
    let private = private_name(&target);
    if !matches!(vm.lookup_command(&private), Some(Command::Proc(_))) {
        return err(format!("proc {} not found", target.procedure));
    }
    ok(Value::list(vec![
        Value::string(target.rule.as_path()),
        Value::string(private),
    ]))
}

fn string_member(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !vm.authored_rule_providers.counted_strings {
        return vm.refuse_host_command("authored counted string provider is unavailable".into());
    }
    let [member] = args else {
        return err("string member requires an original selector");
    };
    if vm.active_native_profile.is_some() {
        return ok(member.clone());
    }
    let bytes = match vm.native_name_operand_bytes(member) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let profile = crate::environment::profile_for_dialect("tcl8.4");
    let Some(spec) = crate::environment::store_for_profile(profile).get("string") else {
        return vm.refuse_host_command("authored string member table is unavailable".into());
    };
    let table = spec.subcommand_table(Some(crate::environment::surface_point(profile)), None, None);
    let members = table.names().collect::<Vec<_>>();
    match tcl_registry::native_package::select_authored_keyword(&bytes, &members) {
        Ok(index) => ok(Value::string(members[index])),
        Err(message) => crate::command::completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::new_bytes(message),
        ),
    }
}

fn counted_length(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !vm.authored_rule_providers.counted_strings {
        return vm.refuse_host_command("authored counted string provider is unavailable".into());
    }
    if vm.active_native_profile.is_some() {
        return crate::cmd_string::physical_string_length(vm, args);
    }
    let [original] = args else {
        return err("wrong # args: should be \"string length string\"");
    };
    let bytes = if original.is_pure_byte_array() {
        original
            .byte_array_representation()
            .expect("pure byte array")
    } else {
        match vm.native_name_operand_bytes(original) {
            Ok(bytes) => bytes,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        }
    };
    match i64::try_from(bytes.len()) {
        Ok(length) => ok(Value::int(length)),
        Err(error) => vm.refuse_host_command(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_rule_capabilities_require_an_explicit_host_and_logical_name_provider() {
        let mut vm = Vm::new();
        assert!(!vm.install_irules_rule_callable_simulation());
        assert!(!vm.install_irules_counted_string_simulation());
        assert!(!vm.authored_rule_providers.callables);
        assert!(!vm.authored_rule_providers.counted_strings);
    }
}
