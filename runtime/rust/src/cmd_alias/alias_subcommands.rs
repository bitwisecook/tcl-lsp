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

//! `interp alias` and `interp aliases`, and the child-handle alias operation.

use super::child_interpreters::{interp_path, not_found_original_path};
use super::{alias_loop_error, only_single_interp};
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;

/// `interp alias {} aliasName ?{} target ?arg ...??` — create / query / delete.
pub(super) fn interp_alias(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_alias_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_alias_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"interp alias srcPath srcCmd ?targetPath targetCmd? ?arg ...?");
    }
    let source_path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    if source_path.len() > 1 {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "nested Runtime alias source interpreter",
            )
            .into(),
        );
    }
    let child = source_path.first();
    if child.is_some_and(|child| !interp.child_exists(child)) {
        return not_found_original_path(interp, argv[2]);
    }
    if argv.len() >= 5
        && !match interp_path(interp, argv[4]) {
            Ok(path) => path,
            Err(code) => return code,
        }
        .is_empty()
    {
        return only_single_interp(interp);
    }
    let Some(recipe) = interp
        .name_policy_protocol()
        .map(|protocol| protocol.recipe())
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original alias capture issuer",
            )
            .into(),
        );
    };
    // C84/85 read the target before AliasCreate captures members. C86+ reach
    // that same original target only after publication, in the loop check.
    let delete = if argv.len() >= 6 && recipe.alias_target_string_precedes_capture() {
        let target = match arguments.bytes(5) {
            Ok(target) => target,
            Err(error) => return interp.report_cmd_error(error),
        };
        if recipe.alias_pretested_target_is_empty(&target) == Some(true) {
            if argv.len() > 6 {
                return interp
                    .wrong_args(b"interp alias srcPath srcCmd ?targetPath targetCmd? ?arg ...?");
            }
            true
        } else {
            false
        }
    } else {
        argv.len() == 5
    };
    // The capture is allocation-only. Its List retains original members, not
    // their bytes, before the C name getter; no prefix getter occurs here.
    let captured = if argv.len() >= 6 && !delete {
        Some((
            std::rc::Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::retain(
                argv[3],
            ))),
            std::rc::Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::fresh(
                interp.new_list_object(&argv[5..]),
            ))),
        ))
    } else {
        None
    };
    let original_name = match arguments.bytes(3) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error),
    };
    let name = match recipe.alias_publication_input(
        tcl_syntax::naming::NativeNameContext::root(),
        &original_name,
    ) {
        Ok(name) => name.selected().to_vec(),
        Err(_) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "alias publication input",
                )
                .into(),
            );
        }
    };
    if delete {
        if let Some(child) = child {
            let deleted = interp.with_child(child, |owner| {
                let scope =
                    match crate::interp::native_operation_currency::NativeOperationScope::enter(
                        owner,
                    ) {
                        Ok(scope) => scope,
                        Err(first) => {
                            owner.refuse_native_execution(first);
                            return Err(owner.clone());
                        }
                    };
                if let Err(error) = owner.delete_original_alias_registration(&name) {
                    owner.refuse_completion_value_access(error);
                    return Err(owner.clone());
                }
                match scope.currency().ensure_current_or_refuse() {
                    Ok(()) => Ok(()),
                    Err(first) => {
                        owner.refuse_native_execution(first);
                        Err(owner.clone())
                    }
                }
            });
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            if let Some(Err(owner)) = deleted {
                return interp.transport_host_refusal_from(&owner);
            }
        } else {
            if let Err(error) = interp.delete_original_alias_registration(&name) {
                return interp.report_cmd_error(error.into());
            }
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
        }
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    if argv.len() == 4 {
        let queried = if let Some(child) = child {
            interp
                .with_child(child, |owner| {
                    let scope =
                        match crate::interp::native_operation_currency::NativeOperationScope::enter(
                            owner,
                        ) {
                            Ok(scope) => scope,
                            Err(first) => {
                                owner.refuse_native_execution(first);
                                return Err(owner.clone());
                            }
                        };
                    let query = owner.alias_query_object(&name);
                    if let Err(first) = scope.currency().ensure_current_or_refuse() {
                        owner.refuse_native_execution(first);
                        return Err(owner.clone());
                    }
                    match query {
                        Ok(query) => Ok(query),
                        Err(error) => {
                            owner.refuse_completion_value_access(error);
                            Err(owner.clone())
                        }
                    }
                })
                .expect("source child was selected")
        } else {
            match interp.alias_query_object(&name) {
                Ok(query) => Ok(query),
                Err(error) => {
                    interp.refuse_completion_value_access(error);
                    Err(interp.clone())
                }
            }
        };
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        return match queried {
            Ok(Some(original)) => {
                interp.set_result(original.as_ptr());
                Code::Ok
            }
            Ok(None) => {
                interp.set_result_bytes(b"");
                Code::Ok
            }
            Err(owner) => interp.transport_host_refusal_from(&owner),
        };
    }
    let (registration_name, original_prefix) =
        captured.expect("actual original alias creation operands");
    if let Some(child) = child {
        let mut owner = interp
            .with_child(child, |owner| owner.clone())
            .expect("source child was selected");
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(&owner) {
                Ok(scope) => scope,
                Err(first) => {
                    owner.refuse_native_execution(first);
                    return interp.transport_host_refusal_from(&owner);
                }
            };
        let retained_registration = std::rc::Rc::clone(&registration_name);
        let installed = interp.install_parent_alias_objects(
            child,
            &original_name,
            Some(registration_name),
            std::rc::Rc::clone(&original_prefix),
        );
        let generation = owner.original_parent_alias_generation(&name, &original_prefix);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
            owner.retire_original_parent_alias(&name, &original_prefix, generation);
            return interp.transport_host_refusal_from(&owner);
        }
        if let Err(error) = arguments.check() {
            owner.retire_original_parent_alias(&name, &original_prefix, generation);
            return interp.report_cmd_error(error);
        }
        match installed {
            Ok(true) => {}
            Ok(false) => return not_found_original_path(interp, argv[2]),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
        if recipe.alias_publication_checks_original_target() {
            // The supported cross-interpreter alias targets its parent. Reach
            // the original target under that parent's genuine operation;
            // never resolve an equal spelling in the child's namespace.
            if let Err(error) = arguments.bytes(5) {
                owner.retire_original_parent_alias(&name, &original_prefix, generation);
                return interp.report_cmd_error(error);
            }
        }
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
            owner.retire_original_parent_alias(&name, &original_prefix, generation);
            return interp.transport_host_refusal_from(&owner);
        }
        let token = match owner.complete_original_alias_registration(
            &original_name,
            tcl_syntax::naming::NativeNamePurpose::ChildAliasPublication,
            &retained_registration,
            generation,
        ) {
            Ok(token) => token,
            Err(error) => {
                owner.report_cmd_error(error);
                return interp.transport_host_refusal_from(&owner);
            }
        };
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        if let Some(token) = token {
            interp.set_result(token.as_ptr());
        }
    } else {
        let installed =
            interp.install_alias_objects(&original_name, registration_name, original_prefix);
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        match installed {
            Ok(None) => {}
            Ok(Some(simple)) => return alias_loop_error(interp, &simple),
            Err(error) => return interp.report_cmd_error(error),
        }
    }
    if recipe == tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4) {
        interp.set_result(argv[3]);
    }
    Code::Ok
}

/// The actual child-handle alias operation. C's target pretest precedes
/// capture on every supported release; Jim captures its parent List before
/// the name and keeps the original target/prefix Strings lazy.
pub(crate) fn child_alias_handle_original(
    interp: &mut Interp,
    child: &[u8],
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    use crate::interp::native_operation_currency::NativeOperationScope;
    use std::rc::Rc;
    use tcl_syntax::value::ValueError;
    let Some(recipe) = interp
        .name_policy_protocol()
        .map(|protocol| protocol.recipe())
    else {
        return interp.report_cmd_error(
            ValueError::CommandProtocolUnavailable("original child alias recipe").into(),
        );
    };
    let usage = b"alias aliasName ?targetName? ?arg ...?";
    if argv.len() < 3 || (!recipe.child_alias_target_string_precedes_capture() && argv.len() < 4) {
        return interp.wrong_args_for_invocation(argv, usage);
    }
    let delete = if argv.len() >= 4 && recipe.child_alias_target_string_precedes_capture() {
        let target = match arguments.bytes(3) {
            Ok(target) => target,
            Err(error) => return interp.report_cmd_error(error),
        };
        if recipe.child_alias_pretested_target_is_empty(&target) == Some(true) {
            if argv.len() != 4 {
                return interp.wrong_args_for_invocation(argv, usage);
            }
            true
        } else {
            false
        }
    } else {
        false
    };
    let captured = if argv.len() >= 4 && !delete {
        let prefix = if recipe.child_alias_target_string_precedes_capture() {
            Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::fresh(
                interp.new_list_object(&argv[3..]),
            )))
        } else {
            match interp.original_jim_parent_alias_prefix(&argv[3..]) {
                Ok(prefix) => prefix,
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        };
        let registration_name = recipe
            .alias_record_retains_original_registration_name()
            .then(|| {
                Rc::new(crate::interp::NativeAliasHeader::new(obj::Owned::retain(
                    argv[2],
                )))
            });
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        Some((registration_name, prefix))
    } else {
        None
    };
    let name = match arguments.bytes(2) {
        Ok(name) => name,
        Err(error) => return interp.report_cmd_error(error),
    };
    let Some(mut owner) = interp.with_child(child, |owner| owner.clone()) else {
        return interp.error(b"could not find interpreter");
    };
    let scope = match NativeOperationScope::enter(&owner) {
        Ok(scope) => scope,
        Err(first) => {
            owner.refuse_native_execution(first);
            return interp.transport_host_refusal_from(&owner);
        }
    };
    if delete {
        if let Err(error) = owner.delete_original_alias_registration(&name) {
            owner.refuse_completion_value_access(error);
            return interp.transport_host_refusal_from(&owner);
        }
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
        }
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        if owner.host_refusal_pending() {
            return interp.transport_host_refusal_from(&owner);
        }
        drop(scope);
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    if argv.len() == 3 {
        let query = owner.alias_query_object(&name);
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            owner.refuse_native_execution(first);
        }
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
        if owner.host_refusal_pending() {
            return interp.transport_host_refusal_from(&owner);
        }
        drop(scope);
        return match query {
            Ok(Some(query)) => {
                interp.set_result(query.as_ptr());
                Code::Ok
            }
            Ok(None) if recipe.is_jim084() => {
                let mut message = b"alias \"".to_vec();
                message.extend_from_slice(&name);
                message.extend_from_slice(b"\" not found");
                interp.set_error(&message)
            }
            Ok(None) => {
                interp.set_result_bytes(b"");
                Code::Ok
            }
            Err(error) => {
                owner.refuse_completion_value_access(error);
                interp.transport_host_refusal_from(&owner)
            }
        };
    }
    let (registration_name, prefix) = captured.expect("actual child alias capture");
    let retained_registration = registration_name.as_ref().map(Rc::clone);
    let installed =
        interp.install_parent_alias_objects(child, &name, registration_name, Rc::clone(&prefix));
    let generation = owner.original_parent_alias_generation(&name, &prefix);
    if let Err(first) = scope.currency().ensure_current_or_refuse() {
        owner.refuse_native_execution(first);
    }
    if let Err(error) = arguments.check() {
        owner.retire_original_parent_alias(&name, &prefix, generation);
        return interp.report_cmd_error(error);
    }
    if owner.host_refusal_pending() {
        owner.retire_original_parent_alias(&name, &prefix, generation);
        return interp.transport_host_refusal_from(&owner);
    }
    match installed {
        Ok(true) => {}
        Ok(false) => return interp.error(b"could not find interpreter"),
        Err(error) => return interp.report_cmd_error(error.into()),
    }
    if recipe.alias_publication_checks_original_target() {
        if let Err(error) = arguments.bytes(3) {
            owner.retire_original_parent_alias(&name, &prefix, generation);
            return interp.report_cmd_error(error);
        }
    }
    if let Err(first) = scope.currency().ensure_current_or_refuse() {
        owner.refuse_native_execution(first);
        owner.retire_original_parent_alias(&name, &prefix, generation);
        return interp.transport_host_refusal_from(&owner);
    }
    if let Err(error) = arguments.check() {
        owner.retire_original_parent_alias(&name, &prefix, generation);
        return interp.report_cmd_error(error);
    }
    let mut token = None;
    if let Some(registration) = retained_registration {
        token = match owner.complete_original_alias_registration(
            &name,
            tcl_syntax::naming::NativeNamePurpose::ChildAliasPublication,
            &registration,
            generation,
        ) {
            Ok(token) => token,
            Err(error) => {
                owner.report_cmd_error(error);
                return interp.transport_host_refusal_from(&owner);
            }
        };
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
    }
    drop(scope);
    if recipe.child_alias_returns_original_registration_name() {
        if recipe == tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_4) {
            interp.set_result(argv[2]);
        } else if let Some(token) = token {
            interp.set_result(token.as_ptr());
        }
    } else {
        interp.set_result_bytes(b"");
    }
    Code::Ok
}

/// `interp aliases ?path?` — every alias command's name in the named interp (the
/// current one for an empty/missing path) as a Tcl list.
pub(super) fn interp_aliases(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"interp aliases ?path?");
    }
    let path = match argv.get(2) {
        None => Vec::new(),
        Some(&original) => match interp_path(interp, original) {
            Ok(path) => path,
            Err(code) => return code,
        },
    };
    let names = match interp.with_child_path(&path, |owner| owner.alias_names()) {
        Some(Ok(names)) => names,
        Some(Err(error)) => return interp.report_cmd_error(error.into()),
        None => return not_found_original_path(interp, argv[2]),
    };
    let elems = names.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
    interp.set_result(interp.new_list_object(&elems));
    Code::Ok
}
