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

//! `global`, `upvar` and `variable`: linking a local name to a variable in another frame or namespace.

use super::error_completion::{completion_from_cmd_error, err_with_code};
use super::frame_level::runtime_frame_selection;
use super::native_wrong_args_bytes;
use crate::interp::{Vm, ok};
use crate::value::Value;
use tcl_runtime_api::Completion;

/// Render a native link failure using exact original operand bytes.
pub(crate) fn upvar_link_error_bytes(
    error: crate::interp::UpvarLinkError,
    other: &[u8],
    local: &[u8],
) -> Completion<Value> {
    upvar_link_error_bytes_in(error, other, local, None)
}

pub(crate) fn upvar_link_error_bytes_in(
    error: crate::interp::UpvarLinkError,
    other: &[u8],
    local: &[u8],
    protocol: Option<tcl_syntax::native_variable_name::NativeVariableNameProtocol>,
) -> Completion<Value> {
    use crate::interp::UpvarLinkError;
    let inversion_reason = protocol.map_or(
        b"can't create namespace variable that refers to procedure variable".as_slice(),
        tcl_syntax::native_variable_name::NativeVariableNameProtocol::alias_namespace_inversion_reason,
    );
    let inversion_suffix = [b"\": ".as_slice(), inversion_reason].concat();
    let (prefix, name, suffix, code): (&[u8], &[u8], &[u8], &[u8]) = match error {
        UpvarLinkError::TargetNamespace => (
            b"can't access \"",
            other,
            b"\": parent namespace doesn't exist",
            b"TCL LOOKUP VARNAME",
        ),
        UpvarLinkError::LocalNamespace => (
            b"can't create \"",
            local,
            b"\": parent namespace doesn't exist",
            b"TCL LOOKUP VARNAME",
        ),
        UpvarLinkError::Inverted => (
            b"bad variable name \"",
            local,
            &inversion_suffix,
            b"TCL UPVAR INVERTED",
        ),
        UpvarLinkError::LocalElement => (
            b"bad variable name \"",
            local,
            b"\": can't create a scalar variable that looks like an array element",
            b"TCL UPVAR LOCAL_ELEMENT",
        ),
        UpvarLinkError::Exists => (
            b"variable \"",
            local,
            b"\" already exists",
            b"TCL UPVAR EXISTS",
        ),
        UpvarLinkError::Traced => (
            b"variable \"",
            local,
            b"\" has traces: can't use for upvar",
            b"TCL UPVAR TRACED",
        ),
        UpvarLinkError::SelfLink => {
            return err_with_code("can't upvar from variable to itself", "TCL UPVAR SELF");
        }
    };
    let mut message = prefix.to_vec();
    message.extend_from_slice(name);
    message.extend_from_slice(suffix);
    let mut error_code = code.to_vec();
    if matches!(
        error,
        UpvarLinkError::TargetNamespace | UpvarLinkError::LocalNamespace
    ) {
        let base = tcl_syntax::naming::split_element_ref_bytes(name).map_or(name, |(base, _)| base);
        error_code.push(b' ');
        tcl_syntax::list::append_list_element(&mut error_code, base, false);
    }
    err_with_code(message, error_code)
}

/// `global name ?name ...?` — link names to the global frame.
///
/// Outside a procedure `global` is a documented no-op: `Tcl_GlobalObjCmd`
/// returns `TCL_OK` before touching its arguments when the current frame *is*
/// the global frame. So the element-name guard below is reached only from a
/// proc — at top level and inside `namespace eval`, even `global (x)` and
/// `global a(b)` are accepted. Verified on 8.6.16 and 9.0.4, which agree.
pub(super) fn cmd_global(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if tcl_registry::VariableAliasDestination::ProcedureLocal
        .is_active_in_frame(vm.variable_alias_frame(), vm.native_invocation_dialect())
        != Some(true)
    {
        return ok(Value::empty());
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("global alias policy is unavailable".into());
    };
    for value in args {
        let name = match vm.native_name_operand_bytes_checked(value) {
            Ok(name) => name,
            Err(error) => {
                return completion_from_cmd_error(vm, error);
            }
        };
        let Some(local) = tcl_syntax::naming::global_local_name_bytes(policy.recipe(), &name)
        else {
            continue;
        };
        if vm.native_c_variable_name_protocol().is_some() {
            let local = if policy
                .recipe()
                .variable_alias_local_input(&name)
                .qualification()
                == tcl_syntax::naming::NativeNameQualification::Unqualified
            {
                value.native_lifetime_lease().into_value()
            } else {
                Value::new_native_string_bytes(local)
            };
            if let Err(error) = vm.link_original_c_variable_objects(value, 0, None, &local) {
                return error;
            }
            continue;
        }
        if let Err(error) = vm.add_link_original(&local, 0, value) {
            return error;
        }
    }
    ok(Value::empty())
}

/// `upvar ?level? otherVar localVar ?otherVar localVar ...?`.
pub(crate) fn cmd_upvar(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let usage = vm.actual_native_invocation_dialect().native_namespace_upvar_protocol()
        .and_then(tcl_registry::native_namespace_upvar::NativeNamespaceUpvarProtocol::forwarded_wrong_arguments_usage)
        .unwrap_or(b"upvar ?level? otherVar localVar ?otherVar localVar ...?");
    if args.len() < 2 {
        return native_wrong_args_bytes(vm, usage);
    }
    let (width, target) =
        match runtime_frame_selection(vm, tcl_registry::FrameEffectSpec::UPVAR, args) {
            Ok(selected) => selected,
            Err(error) => return error,
        };
    let rest = &args[width..];
    if rest.is_empty() || !rest.len().is_multiple_of(2) {
        return native_wrong_args_bytes(vm, usage);
    }
    let mut i = 0;
    while i + 1 < rest.len() {
        if vm.native_c_variable_name_protocol().is_some() {
            if let Err(error) =
                vm.link_original_c_variable_objects(&rest[i], target, None, &rest[i + 1])
            {
                return error;
            }
            i += 2;
            continue;
        }
        if let Err(error) = vm.link_upvar_original(target, &rest[i], &rest[i + 1]) {
            return error;
        }
        i += 2;
    }
    ok(Value::empty())
}

/// Declare namespace variables and link them into the active variable frame.
pub(crate) fn cmd_variable(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some(grammar) = vm
        .actual_native_invocation_dialect()
        .native_variable_declaration_protocol()
    else {
        return vm
            .refuse_host_command("namespace variable declaration grammar is unavailable".into());
    };
    if let Err(usage) = grammar.arguments(args.len()) {
        let mut full_usage = b"variable ".to_vec();
        full_usage.extend_from_slice(usage);
        return native_wrong_args_bytes(vm, &full_usage);
    }
    if vm.native_c_variable_name_protocol().is_some() {
        for pair in args.chunks(2) {
            if let Err(error) = vm.define_original_c_namespace_variable(&pair[0], pair.get(1)) {
                return error;
            }
        }
        return ok(Value::empty());
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("namespace variable policy is unavailable".into());
    };
    let mut i = 0;
    while i < args.len() {
        let name = match vm.native_name_operand_bytes_checked(&args[i]) {
            Ok(name) => name,
            Err(error) => {
                return completion_from_cmd_error(vm, error);
            }
        };
        let local = tcl_syntax::naming::variable_local_name_bytes(policy.recipe(), &name);
        if !vm.var_parent_exists_bytes(&name) {
            return upvar_link_error_bytes(
                crate::interp::UpvarLinkError::TargetNamespace,
                &name,
                &local,
            );
        }
        if !policy.recipe().is_jim084()
            && policy
                .recipe()
                .combined_variable_input(&name)
                .element()
                .is_some()
        {
            let mut message = b"can't define \"".to_vec();
            message.extend_from_slice(&name);
            message.extend_from_slice(b"\": name refers to an element in an array");
            return err_with_code(message, "TCL UPVAR LOCAL_ELEMENT");
        }
        if let Err(error) = vm.link_namespace_variable_bytes(&local, &name) {
            return upvar_link_error_bytes(error, &name, &local);
        }
        if i + 1 < args.len() {
            if let Err(error) = vm.set_var_bytes(&local, args[i + 1].clone()) {
                return error;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    ok(Value::empty())
}
