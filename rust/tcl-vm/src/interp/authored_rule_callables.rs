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

fn selected_target(
    vm: &mut Vm,
    owner: &Value,
    target: &Value,
    original: &super::original_authored_operands::OriginalAuthoredOperandScope,
) -> Result<RuleProcedureTarget, tcl_cmd_core::CmdError> {
    let owner = original.unicode(vm, owner)?;
    let target = original.unicode(vm, target)?;
    let owner =
        RuleIdentity::new(owner).map_err(|error| tcl_cmd_core::CmdError::new(error.to_string()))?;
    RuleProcedureTarget::resolve(&target, Some(&owner))
        .map_err(|error| tcl_cmd_core::CmdError::new(error.to_string()))
}

fn declare(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    super::original_authored_operands::with_original_authored_operands(vm, |vm, original| {
        declare_in_original(vm, args, original)
    })
}

fn declare_in_original(
    vm: &mut Vm,
    args: &[Value],
    original: &super::original_authored_operands::OriginalAuthoredOperandScope,
) -> Result<Completion<Value>, tcl_cmd_core::CmdError> {
    if !vm.authored_rule_providers.callables {
        return Ok(vm.refuse_host_command("authored rule callable provider is unavailable".into()));
    }
    let [owner, name, parameters, body] = args else {
        return Ok(err(
            "rule declaration requires owner, name, parameters and body",
        ));
    };
    let target = selected_target(vm, owner, name, original)?;
    let owner_text = original.unicode(vm, owner)?;
    if target.rule.as_path() != owner_text.as_str()
        || tcl_syntax::naming::is_qualified(original.unicode(vm, name)?.as_bytes())
    {
        return Ok(err(
            "a rule procedure declaration requires an unqualified local name",
        ));
    }
    original.check(vm)?;
    vm.declare_namespace(&format!(
        "::tmm::_rule_callables::{}",
        target.rule.as_path()
    ));
    original.check(vm)?;
    let private = private_name(&target);
    Ok(crate::command::cmd_authored_rule_proc(
        vm,
        &[
            Value::string(private.as_str()),
            parameters.clone(),
            body.clone(),
        ],
    ))
}

fn target(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    super::original_authored_operands::with_original_authored_operands(vm, |vm, original| {
        target_in_original(vm, args, original)
    })
}

fn target_in_original(
    vm: &mut Vm,
    args: &[Value],
    original: &super::original_authored_operands::OriginalAuthoredOperandScope,
) -> Result<Completion<Value>, tcl_cmd_core::CmdError> {
    if !vm.authored_rule_providers.callables {
        return Ok(vm.refuse_host_command("authored rule callable provider is unavailable".into()));
    }
    let [owner, name] = args else {
        return Ok(err("rule target requires owner and procedure"));
    };
    let target = selected_target(vm, owner, name, original)?;
    let private = private_name(&target);
    original.check(vm)?;
    if !matches!(vm.lookup_command(&private), Some(Command::Proc(_))) {
        return Ok(err(format!("proc {} not found", target.procedure)));
    }
    original.check(vm)?;
    Ok(ok(Value::list(vec![
        Value::string(target.rule.as_path()),
        Value::string(private),
    ])))
}

fn string_member(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    super::original_authored_operands::with_original_authored_operands(vm, |vm, operands| {
        Ok(string_member_in_original(vm, args, operands))
    })
}

fn string_member_in_original(
    vm: &mut Vm,
    args: &[Value],
    operands: &super::original_authored_operands::OriginalAuthoredOperandScope,
) -> Completion<Value> {
    if !vm.authored_rule_providers.counted_strings {
        return vm.refuse_host_command("authored counted string provider is unavailable".into());
    }
    let [member] = args else {
        return err("string member requires an original selector");
    };
    if let Err(error) = operands.current_for_value(vm, member) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    if vm.active_native_profile.is_some() {
        return ok(member.clone());
    }
    let bytes = match operands.string(vm, member) {
        Ok(bytes) => bytes,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
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
    super::original_authored_operands::with_original_authored_operands(vm, |vm, operands| {
        Ok(counted_length_in_original(vm, args, operands))
    })
}

fn counted_length_in_original(
    vm: &mut Vm,
    args: &[Value],
    operands: &super::original_authored_operands::OriginalAuthoredOperandScope,
) -> Completion<Value> {
    if !vm.authored_rule_providers.counted_strings {
        return vm.refuse_host_command("authored counted string provider is unavailable".into());
    }
    if vm.active_native_profile.is_some() {
        return crate::cmd_string::physical_string_length(vm, args);
    }
    let [original] = args else {
        return err("wrong # args: should be \"string length string\"");
    };
    if let Err(error) = operands.current_for_value(vm, original) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    let bytes = if original.is_pure_byte_array() {
        original
            .byte_array_representation()
            .expect("pure byte array")
    } else {
        match operands.string(vm, original) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
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
    fn original_authored_string_selector_and_counted_length_keep_actual_provider_and_roles() {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert!(vm.install_irules_counted_string_simulation());
        let selected = string_member(&mut vm, &[Value::string("len")]);
        assert_eq!(selected.code, crate::Code::Ok);
        assert_eq!(selected.result.string_bytes().as_ref(), b"length");
        let counted = Value::new_native_string_bytes(b"A\0\xc3\xa9".as_slice());
        let before = counted.native_object_reference_count();
        let result = counted_length(&mut vm, std::slice::from_ref(&counted));
        assert_eq!(result.code, crate::Code::Ok);
        assert_eq!(result.result.to_str().as_ref(), "4");
        assert_eq!(counted.native_object_reference_count(), before);
        let loan = counted.native_lifetime_lease();
        drop(counted);
        assert_eq!(
            counted_length(&mut vm, std::slice::from_ref(loan.value())).code,
            crate::Code::Error
        );
        assert!(vm.execution_refusal.is_some());
        assert!(!vm.native_operation_context.has_entered_operation());
    }

    #[test]
    fn authored_rule_capabilities_require_an_explicit_host_and_logical_name_provider() {
        let mut vm = Vm::new();
        assert!(!vm.install_irules_rule_callable_simulation());
        assert!(!vm.install_irules_counted_string_simulation());
        assert!(!vm.authored_rule_providers.callables);
        assert!(!vm.authored_rule_providers.counted_strings);
    }
    fn callable_vm() -> Vm {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_logical_name_provider(
            tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)
        ));
        assert!(vm.install_irules_rule_callable_simulation());
        vm
    }

    #[test]
    fn original_rule_callable_text_keeps_actual_provider_declaration_and_domain_failures() {
        let mut vm = callable_vm();
        let owner = Value::string("/Common/folder/request");
        let name = Value::string("local");
        let owner_count = owner.native_object_reference_count();
        let declared = declare(
            &mut vm,
            &[
                owner.clone(),
                name.clone(),
                Value::string(""),
                Value::string("return OK"),
            ],
        );
        assert_eq!(declared.code, crate::Code::Ok);
        let selected = target(&mut vm, &[owner.clone(), name.clone()]);
        assert_eq!(selected.code, crate::Code::Ok);
        let members = selected.result.as_list().unwrap();
        assert_eq!(
            members[0].string_bytes().as_ref(),
            b"/Common/folder/request"
        );
        assert_eq!(
            members[1].string_bytes().as_ref(),
            b"::tmm::_rule_callables::/Common/folder/request::local"
        );
        assert_eq!(owner.native_object_reference_count(), owner_count);
        let invalid = declare(
            &mut vm,
            &[
                owner.clone(),
                Value::string("other::local"),
                Value::string(""),
                Value::string("return BAD"),
            ],
        );
        assert_eq!(invalid.code, crate::Code::Error);
        assert_eq!(
            invalid.result.string_bytes().as_ref(),
            b"a rule procedure declaration requires an unqualified local name"
        );
        let absent = target(&mut vm, &[owner, Value::string("missing")]);
        assert_eq!(absent.code, crate::Code::Error);
        assert_eq!(
            absent.result.string_bytes().as_ref(),
            b"proc missing not found"
        );
        assert!(vm.execution_refusal.is_none());
        assert!(!vm.native_operation_context.has_entered_operation());
    }

    #[test]
    fn original_rule_callable_strict_text_refusal_precedes_unreached_header_and_publication() {
        let mut vm = callable_vm();
        let owner = Value::new_native_string_bytes(b"/Common/\xff".as_slice());
        let name = Value::string("unreached");
        let released = name.native_lifetime_lease();
        drop(name);
        let original =
            super::super::original_authored_operands::OriginalAuthoredOperandScope::capture(
                &mut vm,
            )
            .unwrap();
        let failed = selected_target(&mut vm, &owner, released.value(), &original).unwrap_err();
        assert_eq!(
            failed.native_execution_refusal(),
            vm.execution_refusal.as_ref()
        );
        assert!(original.finish(&mut vm).is_err());
        let first = vm.execution_refusal.clone().unwrap();
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(_)
            )
        ));
        assert!(
            vm.lookup_command("::tmm::_rule_callables::/Common/request::unreached")
                .is_none()
        );
        assert!(!vm.native_operation_context.has_entered_operation());
    }
}
