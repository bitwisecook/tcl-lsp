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

//! The `namespace` command — `eval` / `current` / `path` / `export` /
//! `import` / `forget` / `which`, plus the cheap introspection forms
//! (`exists` / `parent` / `children` / `qualifiers` / `tail`).
//!
//! Every form is a thin driver over the one namespace tree + resolver in
//! [`crate::namespace`] (the command-binding contract's A1/A2). `import`/`export`
//! match with the shared `string match` glob ([`tcl_syntax::glob`]); an `import`
//! installs a transparent [`Command::Imported`](crate::interp::Command) redirect
//! that dispatch re-resolves by the source FQN, and `forget` removes those
//! redirects by matching the same FQN.
//!
//! See `docs/design/runtime/namespace-tree.md` for the model.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::ensemble::EnsembleConfig;
use crate::interp::{Code, Command, Interp, error_code_list, obj_bytes};
use crate::list;
use crate::namespace::{GLOBAL, NsId};
use crate::obj::{self, TclObj};
use tcl_syntax::value::ValueOps;

/// Register the `namespace` command.
pub fn install(interp: &mut Interp) {
    const NAMES: &[&[u8]] = &[
        b"children".as_slice(),
        b"code".as_slice(),
        b"current".as_slice(),
        b"delete".as_slice(),
        b"ensemble".as_slice(),
        b"eval".as_slice(),
        b"exists".as_slice(),
        b"export".as_slice(),
        b"forget".as_slice(),
        b"import".as_slice(),
        b"inscope".as_slice(),
        b"origin".as_slice(),
        b"parent".as_slice(),
        b"path".as_slice(),
        b"qualifiers".as_slice(),
        b"tail".as_slice(),
        b"unknown".as_slice(),
        b"upvar".as_slice(),
        b"which".as_slice(),
    ];
    let admitted = crate::environment::release_subcommands(
        interp.native_ensemble_profile_name(),
        "namespace",
        NAMES,
    );
    interp.register_stock_ensemble(
        tcl_registry::invocation_words::EnsembleImplementationFamily::Namespace,
        b"namespace",
        namespace_cmd,
        STOCK_MEMBERS,
        admitted,
    );
}

const STOCK_MEMBERS: &[(&[u8], crate::interp::BuiltinFn)] = &[
    (b"children", stock_children),
    (b"code", stock_code),
    (b"current", stock_current),
    (b"delete", stock_delete),
    (b"ensemble", stock_ensemble),
    (b"eval", stock_eval),
    (b"exists", stock_exists),
    (b"export", stock_export),
    (b"forget", stock_forget),
    (b"import", stock_import),
    (b"inscope", stock_inscope),
    (b"origin", stock_origin),
    (b"parent", stock_parent),
    (b"path", stock_path),
    (b"qualifiers", stock_qualifiers),
    (b"tail", stock_tail),
    (b"unknown", stock_unknown),
    (b"upvar", stock_upvar),
    (b"which", stock_which),
];

fn stock_children(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"children"], namespace_cmd)
}

fn stock_code(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"code"], namespace_cmd)
}

fn stock_current(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"current"], namespace_cmd)
}

fn stock_delete(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"delete"], namespace_cmd)
}

fn stock_ensemble(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"ensemble"], namespace_cmd)
}

fn stock_eval(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"eval"], namespace_cmd)
}

fn stock_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"exists"], namespace_cmd)
}

fn stock_export(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"export"], namespace_cmd)
}

fn stock_forget(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"forget"], namespace_cmd)
}

fn stock_import(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"import"], namespace_cmd)
}

fn stock_inscope(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"inscope"], namespace_cmd)
}

fn stock_origin(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"origin"], namespace_cmd)
}

fn stock_parent(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"parent"], namespace_cmd)
}

fn stock_path(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"path"], namespace_cmd)
}

fn stock_qualifiers(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"qualifiers"], namespace_cmd)
}

fn stock_tail(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"tail"], namespace_cmd)
}

fn stock_unknown(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"unknown"], namespace_cmd)
}

fn stock_upvar(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"upvar"], namespace_cmd)
}

fn stock_which(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"namespace", b"which"], namespace_cmd)
}

fn namespace_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args_for_invocation(argv, b"subcommand ?arg ...?");
    }
    // Resolve the subcommand by exact name or unambiguous prefix (the ensemble
    // contract), so e.g. `namespace exist` → `exists`.
    let original = match tcl_syntax::value::ValueOps::native_string_bytes(interp, &argv[1]) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "namespace subcommand issuer",
            )
            .into(),
        );
    };
    let raw = policy
        .recipe()
        .namespace_subcommand_input(&original)
        .selected()
        .to_vec();
    let dialect = Some(crate::environment::surface_point(interp.dialect_profile()));
    let spec = tcl_registry::default_registry()
        .get_for_surface("namespace", dialect)
        .expect("namespace execution profile");
    let Some(subcommand) = spec.resolve_subcommand_bytes_for_dialect(&raw, dialect) else {
        let available: Vec<_> = spec
            .subcommands
            .iter()
            .filter(|candidate| {
                candidate.surface.or(spec.surface).is_none_or(|surface| {
                    tcl_dialect::model::surface_admits(surface, dialect.as_ref())
                })
            })
            .map(|candidate| candidate.name)
            .collect();
        return interp.set_error(&tcl_cmd_core::namespace::unknown_subcommand_message(
            policy.recipe(),
            &available,
            &raw,
        ));
    };
    let sub = subcommand.name.as_bytes();
    match sub {
        b"canonical" => {
            let (namespace, name) = match &argv[2..] {
                [] => {
                    let holder = match interp.jim_current_namespace_object() {
                        Ok(holder) => holder,
                        Err(error) => return interp.report_cmd_error(error.into()),
                    };
                    interp.set_result(holder.as_ptr());
                    return Code::Ok;
                }
                [name] => {
                    let holder = match interp.jim_current_namespace_object() {
                        Ok(holder) => holder,
                        Err(error) => return interp.report_cmd_error(error.into()),
                    };
                    (holder, *name)
                }
                [namespace, name] => (obj::Owned::retain(*namespace), *name),
                _ => return interp.wrong_args_for_prefix(argv, 2, b"?current? ?name?"),
            };
            match interp.jim_canonical_namespace_object(&namespace, name) {
                Ok(canonical) => {
                    interp.set_result(canonical.as_ptr());
                    Code::Ok
                }
                Err(error) => interp.report_cmd_error(error.into()),
            }
        }
        b"current" => ns_current(interp, argv),
        b"delete" => ns_delete(interp, argv),
        b"eval" => ns_eval(interp, argv),
        b"exists" => ns_exists(interp, argv),
        b"parent" => ns_parent(interp, argv),
        b"children" => ns_children(interp, argv),
        b"qualifiers" => ns_qualifiers(interp, argv),
        b"tail" => ns_tail(interp, argv),
        b"which" => ns_which(interp, argv),
        b"origin" => ns_origin(interp, argv),
        b"export" => {
            if interp.dialect_profile().namespace_import_binding()
                == Some(tcl_dialect::NamespaceImportBinding::SourceName)
            {
                interp.set_result_bytes(b"");
                Code::Ok
            } else {
                ns_export(interp, argv)
            }
        }
        b"import" => ns_import(interp, argv),
        b"forget" => ns_forget(interp, argv),
        b"path" => ns_path(interp, argv),
        b"ensemble" => ns_ensemble(interp, argv),
        b"inscope" => ns_inscope(interp, argv),
        b"code" => ns_code(interp, argv),
        b"unknown" => ns_unknown(interp, argv),
        b"upvar" => ns_upvar(interp, argv),
        _ => unreachable!("subcommand resolved to a canonical name above"),
    }
}

// current / eval / exists / parent / children

/// `namespace unknown ?handler?` — get or set the current namespace's
/// unknown-command handler. The global namespace's default is `::unknown`; a
/// sub-namespace with no handler reports the empty string (and falls back to
/// `::unknown` at dispatch). Mirrors `NamespaceUnknownCmd` (`tclNamesp.c`).
fn ns_unknown(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"?script?");
    }
    let cur = interp.current_ns();
    if argv.len() == 3 {
        use tcl_syntax::value::ValueOps;
        let length = match interp.list_len(&argv[2]) {
            Ok(length) => length,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let handler = (length != 0).then(|| obj::Owned::retain(argv[2]));
        let retired = interp.namespaces_mut().set_unknown_handler(cur, None);
        drop(retired);
        let retired = interp.namespaces_mut().set_unknown_handler(cur, handler);
        drop(retired);
        interp.set_result(argv[2]);
        return Code::Ok;
    }
    match interp.namespace_unknown_root(cur, cur == GLOBAL) {
        Some(handler) => interp.set_result(handler.as_ptr()),
        None => interp.set_result_bytes(b""),
    }
    Code::Ok
}

/// `namespace current` — the FQN of the current namespace.
fn ns_current(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args_for_prefix(argv, 2, b"");
    }
    match tcl_cmd_core::namespace::current_original(interp) {
        Ok(value) => {
            interp.set_result(value);
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error),
    }
}

/// `namespace delete ?name name ...?` — delete each named namespace (with its
/// children, commands, and variables). A missing namespace is an error; with no
/// names it is a no-op. Mirrors C's `NamespaceDeleteCmd` (`tclNamesp.c`).
fn ns_delete(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp
        .name_policy_protocol()
        .is_some_and(|policy| policy.recipe().is_jim084())
    {
        return ns_delete_jim(interp, &argv[2..]);
    }
    match tcl_cmd_core::namespace::delete_original(interp, &argv[2..]) {
        Ok(Some(index)) => {
            return ns_operation_not_found(
                interp,
                argv[index + 2],
                tcl_syntax::naming::NativeNamespaceLookupOperation::Delete,
            );
        }
        Ok(None) => {}
        Err(error) => return interp.report_cmd_error(error.into()),
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Jim's script helper removes matching flat commands and globals while
/// retaining the caller's original namespace holder and local variables.
fn ns_delete_jim(interp: &mut Interp, names: &[*mut TclObj]) -> Code {
    use tcl_syntax::value::ValueOps;
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim namespace delete issuer",
            )
            .into(),
        );
    };
    let current = match interp.jim_current_namespace_object() {
        Ok(current) => current,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let empty = obj::Owned::fresh(obj::new_string_bytes(b""));
    let root = obj::Owned::fresh(obj::new_string_bytes(b"::"));
    let matcher = tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy);
    for &name in names {
        let skip =
            match tcl_syntax::native_equality::full_native_equality(interp, &name, &empty.as_ptr())
            {
                Ok(true) => true,
                Ok(false) => match tcl_syntax::native_equality::full_native_equality(
                    interp,
                    &name,
                    &root.as_ptr(),
                ) {
                    Ok(equal) => equal,
                    Err(error) => return interp.report_cmd_error(error.into()),
                },
                Err(error) => return interp.report_cmd_error(error.into()),
            };
        if skip {
            continue;
        }
        let canonical = match interp.jim_canonical_namespace_object(&current, name) {
            Ok(canonical) => canonical,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let canonical = match interp.native_string_bytes(&canonical.as_ptr()) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let mut pattern = canonical.to_vec();
        pattern.extend_from_slice(b"::*");
        let candidates = interp.visible_command_names_in(GLOBAL);
        let commands =
            match policy
                .recipe()
                .jim_info_command_names(b"", Some(&pattern), &candidates, false)
            {
                Ok(commands) => commands,
                Err(_) => {
                    return interp.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "Jim namespace delete enumeration",
                        )
                        .into(),
                    );
                }
            };
        for command in commands {
            let word = obj::Owned::fresh(obj::new_string_bytes(b"rename"));
            let command = obj::Owned::fresh(obj::new_string_bytes(&command));
            let invocation = obj::Owned::fresh(interp.new_list_object(&[
                word.as_ptr(),
                command.as_ptr(),
                empty.as_ptr(),
            ]));
            let code = interp.eval_uplevel_obj(0, invocation.as_ptr());
            if code != Code::Ok {
                return code;
            }
        }
        let globals = interp.namespaces().var_names(GLOBAL);
        let mut operands = vec![obj::Owned::fresh(obj::new_string_bytes(b"unset"))];
        for global in globals {
            match matcher.match_name_pattern(
                tcl_syntax::native_glob::NativeNameGlobPurpose::InfoVariablesScan,
                &pattern,
                &global,
            ) {
                Ok(true) => operands.push(obj::Owned::fresh(obj::new_string_bytes(&global))),
                Ok(false) => {}
                Err(_) => {
                    return interp.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "Jim namespace delete globals",
                        )
                        .into(),
                    );
                }
            }
        }
        let words: Vec<_> = operands.iter().map(obj::Owned::as_ptr).collect();
        let invocation = obj::Owned::fresh(interp.new_list_object(&words));
        let code = interp.eval_uplevel_obj(0, invocation.as_ptr());
        if code != Code::Ok {
            return code;
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace eval name arg ?arg ...?` — evaluate a body in `name` (multiple
/// `arg`s are concatenated with spaces, like `eval`).
fn ns_eval(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"name arg ?arg...?");
    }
    if argv.len() == 4 {
        return interp.ns_eval_objects_with_arguments(
            argv[2],
            argv[3],
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
            Some(argv),
        );
    }
    let body = match tcl_cmd_core::list::concat_selected(interp, &argv[3..]) {
        Ok(body) => obj::Owned::fresh(body),
        Err(error) => return interp.report_cmd_error(error),
    };
    interp.ns_eval_objects_with_arguments(
        argv[2],
        body.as_ptr(),
        tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceConcat,
        Some(argv),
    )
}

/// `namespace exists name` — whether the namespace resolves.
fn ns_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"name");
    }
    match interp.native_namespace_object_lookup(argv[2]) {
        Ok(namespace) => {
            let value = tcl_syntax::value::ValueOps::new_bool(interp, namespace.is_some());
            interp.set_result(value);
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error.into()),
    }
}

/// `namespace parent ?name?` — the FQN of the (named, or current) ns's parent,
/// via the shared `tcl_cmd_core::namespace` core over `Namespaces`.
fn ns_parent(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"?name?");
    }
    let target = if let Some(&original) = argv.get(2) {
        match interp.native_namespace_object_lookup(original) {
            Ok(Some(target)) => target,
            Ok(None) => {
                return ns_operation_not_found(
                    interp,
                    original,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::Parent,
                );
            }
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    } else {
        interp.current_ns()
    };
    let target = match u32::try_from(target) {
        Ok(target) => tcl_runtime_api::NsId(target),
        Err(_) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "namespace parent token width",
                )
                .into(),
            );
        }
    };
    match tcl_cmd_core::namespace::parent_original(interp, target) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error.into()),
    }
}

/// `namespace children ?name? ?pattern?` — child namespace FQNs (glob-filtered),
/// via the shared core.
fn ns_children(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"?name? ?pattern?");
    }
    let target = if let Some(&original) = argv.get(2) {
        match interp.native_namespace_object_lookup(original) {
            Ok(Some(target)) => target,
            Ok(None) => {
                return ns_operation_not_found(
                    interp,
                    original,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::Children,
                );
            }
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    } else {
        interp.current_ns()
    };
    let target = match u32::try_from(target) {
        Ok(target) => tcl_runtime_api::NsId(target),
        Err(_) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "namespace children token width",
                )
                .into(),
            );
        }
    };
    let pattern = if let Some(&original) = argv.get(3) {
        match tcl_syntax::value::ValueOps::native_string_bytes(interp, &original) {
            Ok(bytes) => Some(bytes),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    } else {
        None
    };
    let children = match tcl_cmd_core::namespace::children_tokens_checked(
        interp,
        target,
        pattern.as_deref(),
    ) {
        Ok(children) => children,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    match tcl_cmd_core::namespace::children_original(interp, &children) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error.into()),
    }
}

/// `namespace qualifiers string` — everything before the last `::` (pure text).
fn ns_qualifiers(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"string");
    }
    let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(interp, &argv[2]) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "namespace qualifiers issuer",
            )
            .into(),
        );
    };
    namespace_text_result(
        interp,
        argv[2],
        policy.recipe().namespace_text_result(&bytes, false),
    )
}

/// `namespace tail string` — the simple name after the last `::` (pure text).
fn ns_tail(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"string");
    }
    let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(interp, &argv[2]) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("namespace tail issuer")
                .into(),
        );
    };
    namespace_text_result(
        interp,
        argv[2],
        policy.recipe().namespace_text_result(&bytes, true),
    )
}

/// Apply the selected direct worker producer through the existing object owners.
fn namespace_text_result(
    interp: &mut Interp,
    original: *mut TclObj,
    result: tcl_syntax::naming::NativeNamespaceTextResult<'_>,
) -> Code {
    use tcl_syntax::naming::NativeNamespaceTextResult;
    match result {
        NativeNamespaceTextResult::Original => interp.set_result(original),
        NativeNamespaceTextResult::Counted(bytes) => interp.set_result_bytes(bytes),
        NativeNamespaceTextResult::Append(bytes) => {
            let dialect = interp.native_invocation_dialect();
            let Some(append) = dialect.native_object_append_protocol(None) else {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "namespace text append protocol",
                    )
                    .into(),
                );
            };
            let objects = crate::value_ops::RuntimeAppendObjects {
                dialect,
                binary_recipe: None,
            };
            let receiver = crate::value_ops::RuntimeAppendValue::fresh_string(b"");
            match tcl_cmd_core::native_append::append_counted_bytes(
                &objects,
                append.recipe(),
                &receiver,
                bytes,
            ) {
                Ok(result) => interp.set_result(result.as_ptr()),
                Err(error) => return interp.report_cmd_error(error.into()),
            }
        }
    }
    Code::Ok
}

/// `namespace which ?-command? ?-variable? name` — the FQN `name` resolves to.
/// The variable query uses the actual selected C cell or Jim textual helper.
fn ns_which(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let args = match argv[2..]
        .iter()
        .map(|arg| {
            tcl_syntax::value::ValueOps::native_string_bytes(interp, arg)
                .map(|bytes| bytes.to_vec())
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(args) => args,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let Some((kind, name_index)) = tcl_cmd_core::namespace::which_request(&args) else {
        return interp.wrong_args_for_prefix(argv, 2, b"?-command? ?-variable? name");
    };
    let name = &args[name_index];
    if kind == tcl_cmd_core::namespace::WhichKind::Variable {
        // `-variable` through the shared `Tcl_FindNamespaceVar` core — the
        // 8.x global-fallback candidate is a release axis, so the profile
        // goes with it.
        match tcl_cmd_core::namespace::which_variable_bytes_checked(interp, name) {
            Ok(fqn) => interp.set_result_bytes(&fqn.unwrap_or_default()),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    } else {
        // `-command` via the shared `Namespaces` resolution core.
        match tcl_cmd_core::namespace::which_command_bytes_checked(interp, name) {
            Ok(fqn) => interp.set_result_bytes(&fqn.unwrap_or_default()),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    }
    Code::Ok
}

// export / import / forget

/// `namespace export ?-clear? ?pattern ...?` — query / append (or clear+set)
/// the current namespace's export patterns.
fn ns_export(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let cur = interp.current_ns();
    // No words at all ⇒ query (C's `objc == 1` arm, before `-clear` is even
    // looked at).
    if argv.len() == 2 {
        let pats = interp.namespaces().exports(cur).to_vec();
        set_list_bytes(interp, &pats);
        return Code::Ok;
    }
    // `-clear` is *positional*: C tests only `objv[1]`, so a second `-clear`
    // is an ordinary pattern (the registry says the same with
    // `max_leading_option_words: Some(1)`).
    let mut words = argv[2..].iter().map(|&a| obj_bytes(a));
    let mut first = words.next();
    let clear = first.as_deref() == Some(b"-clear");
    if clear {
        first = words.next();
    }
    let patterns: Vec<Vec<u8>> = first.into_iter().chain(words).collect();
    // `-clear` commits before any pattern is even looked at: C spends a whole
    // `Tcl_Export(…, "::", 1)` call on it, which resets the list and then
    // fails its own qualifier check — an error `NamespaceExportCmd` discards
    // with `Tcl_ResetResult`. So a later invalid pattern cannot undo it.
    if clear {
        interp.namespaces_mut().clear_exports(cur);
    }
    // An export pattern names commands in the *current* namespace only, so it
    // may not be namespace-qualified (C's `NamespaceExportCmd`). C calls
    // `Tcl_Export` once per pattern and returns on the first failure, leaving
    // the earlier patterns committed — this is a per-pattern loop, not a
    // batch gate.
    for p in &patterns {
        if tcl_syntax::naming::is_qualified(p) {
            let mut m = b"invalid export pattern \"".to_vec();
            m.extend_from_slice(p);
            m.extend_from_slice(b"\": pattern can't specify a namespace");
            return interp.set_error(&m);
        }
        interp.namespaces_mut().export(cur, p);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace import ?-force? pattern ?pattern ...?` — install transparent
/// redirects in the current ns for the exported commands matching each pattern.
fn ns_import(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp.dialect_profile().namespace_import_binding()
        == Some(tcl_dialect::NamespaceImportBinding::SourceName)
    {
        return ns_import_source_names(interp, &argv[2..]);
    }
    let dest = interp.current_ns();
    // The introspection form is `objc == 1` — literally no words after
    // `import` — and C tests it *before* looking at `-force`. So a bare
    // `namespace import -force` is NOT a query: it takes the flag, imports
    // nothing, and yields "".
    if argv.len() == 2 {
        let names = interp.imported_command_tails(dest);
        set_list_bytes(interp, &names);
        return Code::Ok;
    }
    // `-force` is positional in the same way `-clear` is: C reads it from
    // `objv[1]` only, so a trailing `-force` becomes a pattern — and then
    // fails the "the pattern must name a source namespace" check below.
    let mut words = argv[2..].iter().map(|&a| obj_bytes(a));
    let mut first = words.next();
    let force = first.as_deref() == Some(b"-force");
    if force {
        first = words.next();
    }
    let patterns: Vec<Vec<u8>> = first.into_iter().chain(words).collect();
    if patterns.is_empty() {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    for pat in &patterns {
        let destination = tcl_runtime_api::Namespaces::current(interp);
        let validated = match tcl_cmd_core::namespace::import_pattern(interp, destination, pat) {
            Ok(validated) => validated,
            Err(problem @ tcl_cmd_core::namespace::ImportPatternError::Unknown(_)) => {
                // A namespace being deleted is absent from the public tree but
                // its command table remains token-addressable during delete
                // callbacks. Tcl_Import can therefore import a callback-created
                // command from that dying table even though `namespace exists`
                // is false. Keep the shared qualifier/tail parser and only
                // substitute this lifecycle-specific namespace lookup.
                let qualifier = match tcl_cmd_core::namespace::qualifier(pat) {
                    tcl_cmd_core::namespace::Qualifier::Absolute(prefix)
                    | tcl_cmd_core::namespace::Qualifier::Relative(prefix) => prefix,
                    tcl_cmd_core::namespace::Qualifier::Unqualified => {
                        return interp.set_error(&problem.message());
                    }
                };
                let Some(source) = interp.namespaces().dying_namespace(dest, qualifier) else {
                    return interp.set_error(&problem.message());
                };
                tcl_cmd_core::namespace::ImportPattern {
                    source: tcl_runtime_api::NsId(source as u32),
                    tail: tcl_cmd_core::namespace::tail(pat).to_vec(),
                }
            }
            Err(problem) => return interp.set_error(&problem.message()),
        };
        let src_ns = validated.source.0 as usize;
        let tail_pat = validated.tail;
        // Collect the matching, exported source commands first (borrow ends).
        let src_fqn = interp.namespaces().qualified_name(src_ns);
        let mut to_import: Vec<Vec<u8>> = Vec::new();
        for name in interp.visible_command_names_in(src_ns) {
            if glob_match_bytes(&tail_pat, &name) && interp.namespaces().is_exported(src_ns, &name)
            {
                to_import.push(name);
            }
        }
        for simple in to_import {
            let mut source = src_fqn.clone();
            if source != b"::" {
                source.extend_from_slice(b"::");
            }
            source.extend_from_slice(&simple);
            let Some((source, source_generation, ensemble)) =
                interp.import_metadata_in(src_ns, &simple)
            else {
                continue;
            };
            // Without `-force`, re-importing the *same* command from the *same*
            // source is a silent no-op (C's `TclGetOriginalCommand` reimport
            // check, tclNamesp.c) — common when a file and its sourced helper
            // both import `::tcltest::*`. `-force` deliberately replaces even
            // that same-origin import with a fresh command token.
            let existing_import = match interp.namespaces().command_in(dest, &simple) {
                Some(Command::Imported {
                    source_generation, ..
                }) => Some(source_generation),
                _ => None,
            };
            if !force && existing_import == Some(source_generation) {
                continue;
            }
            // Reject clobbering an existing (different) command unless -force.
            if !force && (existing_import.is_some() || dest_has_own(interp, dest, &simple)) {
                let mut m = b"can't import command \"".to_vec();
                m.extend_from_slice(&simple);
                m.extend_from_slice(b"\": already exists");
                return interp.error_with_code(&m, b"TCL IMPORT OVERWRITE");
            }
            // Follow immediate import origins before mutating the destination.
            // If the source chain already reaches the command being replaced,
            // this new edge would close Tcl's ImportRef graph into a cycle.
            let destination_generation = interp.namespaces().command_generation(dest, &simple);
            if destination_generation
                .is_some_and(|needle| interp.import_chain_contains(source_generation, needle))
            {
                let mut destination_fqn = interp.namespaces().qualified_name(dest);
                if destination_fqn != b"::" {
                    destination_fqn.extend_from_slice(b"::");
                }
                destination_fqn.extend_from_slice(&simple);
                let mut message = b"import pattern \"".to_vec();
                message.extend_from_slice(pat);
                message.extend_from_slice(b"\" would create a loop containing command \"");
                message.extend_from_slice(&destination_fqn);
                message.push(b'"');
                let error_code = error_code_list(&[b"TCL", b"IMPORT", b"LOOP"]);
                return interp.error_with_code(&message, &error_code);
            }
            interp.bind_command_replacement(
                dest,
                &simple,
                Command::Imported {
                    source,
                    source_generation,
                    ensemble,
                    identity: std::rc::Rc::new(crate::interp::ImportToken),
                },
            );
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

fn ns_import_source_names(interp: &mut Interp, patterns: &[*mut TclObj]) -> Code {
    use tcl_syntax::value::ValueOps;
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("Jim import name issuer")
                .into(),
        );
    };
    let protocol = policy.recipe();
    let current = match interp.jim_current_namespace_object() {
        Ok(current) => current,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let current_name = match interp.native_string_bytes(&current.as_ptr()) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    for &pattern in patterns {
        let original = match interp.native_string_bytes(&pattern) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let canonical = match interp.jim_canonical_namespace_object(&current, pattern) {
            Ok(canonical) => canonical,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let qualified = match interp.native_string_bytes(&canonical.as_ptr()) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let candidates = interp.visible_command_names_in(GLOBAL);
        let names = match protocol.jim_info_command_names(b"", Some(&qualified), &candidates, false)
        {
            Ok(names) => names,
            Err(_) => {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "Jim import command enumeration",
                    )
                    .into(),
                );
            }
        };
        for source in names {
            if protocol.namespace_qualifier_bytes(&source) == current_name.as_ref() {
                let mut error = b"import pattern \"".to_vec();
                error.extend_from_slice(&original);
                error.extend_from_slice(b"\" tries to import from namespace \"");
                error.extend_from_slice(&current_name);
                error.extend_from_slice(b"\" into itself");
                return interp.set_error(&error);
            }
            let mut destination = current_name.to_vec();
            destination.extend_from_slice(b"::");
            destination.extend_from_slice(protocol.namespace_tail_bytes(&source));
            let mut alias = source.clone();
            let mut seen = std::collections::BTreeSet::new();
            while let Some((target, prefix)) = match interp.alias_info_at(GLOBAL, &alias) {
                Ok(prefix) => prefix,
                Err(error) => return interp.report_cmd_error(error.into()),
            } {
                if !seen.insert(alias.clone()) {
                    return interp.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "Jim namespace import cyclic alias chain",
                        )
                        .into(),
                    );
                }
                let mut words = vec![obj::Owned::fresh(obj::new_string_bytes(&target))];
                words.extend(
                    prefix
                        .iter()
                        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word))),
                );
                let pointers: Vec<_> = words.iter().map(obj::Owned::as_ptr).collect();
                let alias_object = obj::Owned::fresh(interp.new_list_object(&pointers));
                alias = match interp.native_string_bytes(&alias_object.as_ptr()) {
                    Ok(bytes) => bytes.to_vec(),
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                let destination_object = obj::Owned::fresh(obj::new_string_bytes(&destination));
                let closes_loop = match tcl_syntax::native_equality::full_native_equality(
                    interp,
                    &alias_object.as_ptr(),
                    &destination_object.as_ptr(),
                ) {
                    Ok(equal) => equal,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                if closes_loop {
                    let mut error = b"import pattern \"".to_vec();
                    error.extend_from_slice(&original);
                    error.extend_from_slice(b"\" would create a loop");
                    return interp.set_error(&error);
                }
            }
            if let Err(name) = interp.install_alias(&destination, source, Vec::new()) {
                let mut error = b"import pattern \"".to_vec();
                error.extend_from_slice(&original);
                error.extend_from_slice(b"\" would create a loop containing command \"");
                error.extend_from_slice(&name);
                error.push(b'"');
                return interp.set_error(&error);
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace forget ?pattern ...?` — remove imported redirects in the current
/// ns whose source FQN matches each (resolved) pattern.
fn ns_forget(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let dest = interp.current_ns();
    for &a in &argv[2..] {
        let pat = obj_bytes(a);
        // Resolve the pattern's namespace to an absolute FQN so we match against
        // the redirect's stored source FQN.
        let q = match tcl_cmd_core::namespace::qualifier(&pat) {
            tcl_cmd_core::namespace::Qualifier::Absolute(q)
            | tcl_cmd_core::namespace::Qualifier::Relative(q) => q,
            tcl_cmd_core::namespace::Qualifier::Unqualified => b"",
        };
        let tail_pat = tcl_cmd_core::namespace::tail(&pat);
        let Some(src_ns) = interp.namespaces().find_namespace(dest, q) else {
            // C's `Tcl_ForgetImport` errors if the pattern's namespace qualifier
            // names a namespace that does not exist (an unqualified pattern
            // resolves to the current namespace, which always exists).
            let mut m = b"unknown namespace in namespace forget pattern \"".to_vec();
            m.extend_from_slice(&pat);
            m.push(b'"');
            return interp.set_error(&m);
        };
        let mut src_fqn = interp.namespaces().qualified_name(src_ns);
        src_fqn.extend_from_slice(b"::");
        src_fqn.extend_from_slice(tail_pat);
        let victims: Vec<Vec<u8>> = interp
            .namespaces()
            .imported_in(dest)
            .into_iter()
            .filter(|(_, source)| glob_match_bytes(&src_fqn, source))
            .map(|(simple, _)| simple)
            .collect();
        for simple in victims {
            interp.namespaces_mut().remove_in(dest, &simple);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// path

/// `namespace path ?nsList?` — query (FQN list) or set the current ns's path.
fn ns_path(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"?nsList?");
    }
    let cur = interp.current_ns();
    if argv.len() == 2 {
        let path = interp.namespaces().path(cur).to_vec();
        let path = match path
            .into_iter()
            .map(|namespace| u32::try_from(namespace).map(tcl_runtime_api::NsId))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(path) => path,
            Err(_) => {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "namespace path token width",
                    )
                    .into(),
                );
            }
        };
        return match tcl_cmd_core::namespace::namespace_objects_original(
            interp,
            &path,
            tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::Path,
        ) {
            Ok(result) => {
                interp.set_result(result);
                Code::Ok
            }
            Err(error) => interp.report_cmd_error(error.into()),
        };
    }
    // Retain the original list members through namespace conversion.
    let original = obj::Owned::retain(argv[2]);
    let elems = match tcl_syntax::value::ValueOps::list_elements(interp, &original.as_ptr()) {
        Ok(elements) => elements,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let mut path: Vec<NsId> = Vec::with_capacity(elems.len());
    for &element in &elems {
        let ns = match interp.native_namespace_object_lookup(element) {
            Ok(Some(namespace)) => namespace,
            Ok(None) => {
                return ns_operation_not_found(
                    interp,
                    element,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
                );
            }
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        path.push(ns);
    }
    interp.namespaces_mut().set_path(cur, path);
    interp.set_result_bytes(b"");
    Code::Ok
}

// helpers

/// Report the reached native operation's error without changing the name extent.
pub(crate) fn ns_operation_not_found(
    interp: &mut Interp,
    original: *mut TclObj,
    operation: tcl_syntax::naming::NativeNamespaceLookupOperation,
) -> Code {
    let bytes = match tcl_syntax::value::ValueOps::native_string_bytes(interp, &original) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "namespace operation diagnostic issuer",
            )
            .into(),
        );
    };
    let current = interp.namespaces().qualified_name(interp.current_ns());
    let error = match tcl_syntax::naming::report_native_namespace_operation_error(
        policy.recipe(),
        operation,
        &bytes,
        &current,
    ) {
        Ok(error) => error,
        Err(_) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "namespace operation diagnostic purpose",
                )
                .into(),
            );
        }
    };
    let materialization = match interp
        .native_invocation_dialect()
        .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
        Some(materialization) if materialization.protocol() == error.string_protocol => {
            materialization
        }
        _ => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "namespace diagnostic String producer issuer",
                )
                .into(),
            );
        }
    };
    let code = match error.error_code {
        Some(code) => interp.error_with_code(&error.message, &code),
        None => interp.set_error(&error.message),
    };
    if let Err(error) =
        obj::retain_native_string_representation(interp.result_obj(), materialization)
    {
        return interp.report_cmd_error(error.into());
    }
    code
}

/// Does the dest namespace hold a command of this name?
fn dest_has_own(interp: &Interp, dest: NsId, simple: &[u8]) -> bool {
    interp.namespaces().command_names(dest).contains(&simple)
}

/// `namespace qualifiers` text op: everything before the last `::` separator,
/// with the whole trailing colon-run trimmed (`foo:::` → `foo`, `:::::` → ``);
/// empty if unqualified.
/// `string match` over bytes (UTF-8 → shared [`tcl_syntax::glob`]; a non-UTF-8
/// pattern or text can only match byte-identically, handled by the equality
/// fallback).
fn glob_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    tcl_syntax::glob::string_match_bytes(pattern, text)
}

/// Set the interp result to a Tcl list of the given byte strings.
fn set_list_bytes(interp: &mut Interp, items: &[Vec<u8>]) {
    let elems: Vec<*mut TclObj> = items.iter().map(|n| obj::new_string_bytes(n)).collect();
    interp.set_result(interp.new_list_object(&elems));
    for e in elems {
        drop_fresh(e);
    }
}

/// Free a freshly created (`rc 0`) object once `new_list_obj` retained it.
fn drop_fresh(obj: *mut TclObj) {
    // SAFETY: `obj` is a live rc-0 object; retain-then-release frees it cleanly.
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(obj);
    }
}

/// `namespace inscope ns cmd ?arg ...?` — evaluate `cmd` (with the extra args
/// appended) in namespace `ns`. Like `namespace eval` but used by
/// `namespace code` scripts.
///
/// `NamespaceInscopeCmd` (`generic/tclNamesp.c`) is the one member of the
/// `Tcl_ConcatObj` eval family whose trailing words are **not** space-joined
/// into the script text: it collects the extra args into a **list object**
/// and concatenates that list's string rep onto the script, so each extra
/// word reaches the invoked command as exactly one argument however much
/// whitespace or list punctuation it holds:
///
/// ```text
/// namespace inscope :: {puts} {a b}   → prints "a b"  (one argument)
/// namespace eval    :: {puts} {a b}   → error: can not find channel named "a"
/// ```
///
/// (This mirrors the bytecode VM's handling of the same construct.) With no
/// extra args, C takes the `objc == 3` arm and evaluates the script
/// verbatim — no concat, so no trim and no trailing space, which the early
/// return mirrors.
fn ns_inscope(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"name arg ?arg...?");
    }
    let namespace = match interp.native_namespace_object_lookup(argv[2]) {
        Ok(Some(namespace)) => namespace,
        Ok(None) => {
            return ns_operation_not_found(
                interp,
                argv[2],
                tcl_syntax::naming::NativeNamespaceLookupOperation::Inscope,
            );
        }
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let (body, purpose) = if argv.len() == 4 {
        (
            obj::Owned::retain(argv[3]),
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
        )
    } else {
        let tail = obj::Owned::fresh(interp.new_list_object(&argv[4..]));
        let body = match tcl_cmd_core::list::concat_selected(interp, &[argv[3], tail.as_ptr()]) {
            Ok(body) => obj::Owned::fresh(body),
            Err(error) => return interp.report_cmd_error(error),
        };
        (
            body,
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceConcat,
        )
    };
    let location = interp.arg_location(argv[3]);
    interp.ns_eval_in_token(
        namespace,
        &[],
        location,
        true,
        Some((purpose, body.as_ptr())),
        Some(argv),
    )
}

/// `namespace origin command` — the fully-qualified original name of `command`
/// (following `namespace import` chains to the source).
fn ns_origin(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"name");
    }
    let origin = if interp
        .native_invocation_dialect()
        .native_command_name_protocol()
        .is_some()
    {
        interp.native_namespace_origin(argv[2])
    } else {
        // Jim's scripted helper follows source-name aliases, not C command tokens.
        let name = match interp.native_string_bytes(&argv[2]) {
            Ok(name) => name,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        tcl_cmd_core::namespace::origin_bytes_checked(interp, &name)
    };
    let origin = match origin {
        Ok(origin) => origin,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    match origin {
        Some(fqn) => {
            if interp
                .native_invocation_dialect()
                .native_command_name_protocol()
                .is_some()
            {
                let original = match interp.native_namespace_origin_result(&fqn) {
                    Ok(original) => original,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                interp.set_result(original.as_ptr());
            } else {
                interp.set_result_bytes(&fqn);
            }
            Code::Ok
        }
        None => {
            if interp
                .native_invocation_dialect()
                .native_command_name_protocol()
                .is_some()
            {
                return interp.native_namespace_origin_failure(argv[2]);
            }
            let name = match interp.native_string_bytes(&argv[2]) {
                Ok(name) => name,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            let mut m = b"invalid command name \"".to_vec();
            m.extend_from_slice(&name);
            m.push(b'"');
            let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", &name]);
            interp.error_with_code(&m, &error_code)
        }
    }
}

/// `namespace code script` — capture `script` together with the current
/// namespace so it can be evaluated later in the right context (used by
/// callbacks). Returns `::namespace inscope <currentNs> <script>`, built as a
/// proper list so `script` is correctly quoted. A script that is already such a
/// capture is returned unchanged (`NamespaceCodeCmd`, `tclNamesp.c`).
fn ns_code(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"arg");
    }
    let Some(policy) = interp
        .native_invocation_dialect()
        .namespace_code_handler_policy(Some(
            tcl_registry::native_namespace_code::LogicalNamespaceCodeProvider::Tcl84CoreSimulation,
        ))
    else {
        return interp.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "namespace code",
            ),
        );
    };
    let original_bytes = match interp.native_string_bytes(&argv[2]) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    if policy.preserves_argument(&original_bytes) {
        interp.set_result(argv[2]);
        return Code::Ok;
    }
    let cur = interp.current_ns();
    let cur = match u32::try_from(cur) {
        Ok(namespace) => tcl_runtime_api::NsId(namespace),
        Err(_) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "namespace code token width",
                )
                .into(),
            );
        }
    };
    let ns_name = match tcl_cmd_core::namespace::NamespaceObjectBackend::produce_namespace_object(
        interp,
        cur,
        tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::CodeContext,
    ) {
        Ok(value) => obj::Owned::fresh(value),
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let elems = [
        crate::interp::new_string(b"::namespace"),
        crate::interp::new_string(b"inscope"),
        ns_name.as_ptr(),
        argv[2],
    ];
    interp.set_result(interp.new_list_object(&elems));
    Code::Ok
}

/// `namespace upvar ns ?otherVar myVar ...?` — link each `myVar` in the current
/// frame to `otherVar`, a variable resolved in namespace `ns` (mirrors C's
/// `NamespaceUpvarCmd`: the other-var is looked up with the var frame's
/// namespace temporarily set to `ns`).
fn ns_upvar(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(grammar) = interp
        .native_invocation_dialect()
        .native_namespace_upvar_protocol()
    else {
        return interp.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "namespace upvar argument grammar",
            ),
        );
    };
    let arguments = match grammar.arguments(argv.len().saturating_sub(2)) {
        Ok(arguments) => arguments,
        Err(_) => return interp.wrong_args_for_prefix(argv, 2, grammar.wrong_arguments_suffix()),
    };
    let protocol = match interp.require_variable_name_protocol() {
        Ok(protocol) => protocol,
        Err(error) => return crate::builtins::var_error(interp, b"", error),
    };
    if protocol.is_jim084() {
        return ns_upvar_jim(interp, &argv[2..], arguments);
    }
    let ns = match interp.native_namespace_object_lookup(argv[2]) {
        Ok(Some(namespace)) => namespace,
        Ok(None) => {
            return ns_operation_not_found(
                interp,
                argv[2],
                tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
            );
        }
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    for (target_word, local_word) in arguments.pairs() {
        let local_word = local_word.expect("selected C grammar has complete pairs") + 2;
        let bytes =
            match tcl_syntax::value::ValueOps::native_string_bytes(interp, &argv[local_word]) {
                Ok(bytes) => bytes,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
        let local = match protocol.namespace_upvar_local_input(&bytes) {
            Ok(input) => crate::obj::Owned::fresh(crate::obj::new_string_bytes(input.selected())),
            Err(_) => {
                return interp.refuse_native_access(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "namespace upvar local name",
                    ),
                );
            }
        };
        let code =
            interp.link_original_c_namespace_objects(argv[target_word + 2], ns, local.as_ptr());
        if code != Code::Ok {
            return code;
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Jim's helper canonicalizes once, pads an odd local and tailcalls actual upvar.
fn ns_upvar_jim(
    interp: &mut Interp,
    original: &[*mut TclObj],
    arguments: tcl_registry::native_namespace_upvar::NativeNamespaceUpvarArguments,
) -> Code {
    let current = match interp.jim_current_namespace_object() {
        Ok(current) => current,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let canonical = match interp.jim_canonical_namespace_object(&current, original[0]) {
        Ok(canonical) => canonical,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let namespace = match interp.native_string_bytes(&canonical.as_ptr()) {
        Ok(namespace) => namespace,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let mut owned = vec![
        obj::Owned::fresh(obj::new_string_bytes(b"upvar")),
        obj::Owned::fresh(obj::new_string_bytes(b"0")),
    ];
    let mut forwarded = vec![owned[0].as_ptr(), owned[1].as_ptr()];
    for (target_word, local_word) in arguments.pairs() {
        let other = match interp.native_string_bytes(&original[target_word]) {
            Ok(other) => other,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let mut target = b"::".to_vec();
        target.extend_from_slice(&namespace);
        target.extend_from_slice(b"::");
        target.extend_from_slice(&other);
        owned.push(obj::Owned::fresh(obj::new_string_bytes(&target)));
        forwarded.push(owned.last().expect("generated target owner").as_ptr());
        forwarded.push(match local_word {
            Some(word) => original[word],
            None => {
                owned.push(obj::Owned::fresh(obj::new_string_bytes(b"")));
                owned.last().expect("Jim odd-tail padding owner").as_ptr()
            }
        });
    }
    interp.dispatch_in_lookup_namespace(GLOBAL, &forwarded)
}

// ensemble

/// `namespace ensemble create|exists ...` — the canonical `ens sub`→target
/// redirect (the generalised `dict for`→`::tcl::dict::for` mechanism).
fn ns_ensemble(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() >= 3 {
        let original = match interp.native_string_bytes(&argv[1]) {
            Ok(original) => original,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        if let Some(helper) = interp
            .native_invocation_dialect()
            .native_namespace_scripted_helper(&original)
        {
            let helper = obj::Owned::fresh(crate::interp::new_string(&helper));
            let mut forwarded = Vec::with_capacity(argv.len() - 1);
            forwarded.push(helper.as_ptr());
            forwarded.extend_from_slice(&argv[2..]);
            return interp.dispatch(&forwarded);
        }
    }
    if let Some(protocol) = interp
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    {
        let lifecycle = match interp.native_current_namespace_lifecycle() {
            Ok(lifecycle) => lifecycle,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        if !protocol.permits_namespace_lifecycle(lifecycle) {
            return interp.report_cmd_error(tcl_cmd_core::CmdError::with_error_code_bytes(
                b"tried to manipulate ensemble of deleted namespace".to_vec(),
                b"TCL ENSEMBLE DEAD".to_vec(),
            ));
        }
    }

    if argv.len() < 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"subcommand ?arg ...?");
    }
    // The shared `ensembleSubcommands` table (flags 0, so `cr` is `create`).
    match interp.native_static_string_option_index(
        argv[2],
        tcl_cmd_core::ensemble::SUBCOMMANDS.names(),
        false,
        "subcommand",
    ) {
        Ok(0) => ens_configure(interp, argv),
        Ok(1) => ens_create(interp, argv),
        Ok(_) => ens_exists(interp, argv),
        Err(message) => interp.report_cmd_error(message),
    }
}

/// `namespace ensemble create ?-command name? ?-map dict? ?-subcommands list?
/// ?-prefixes bool?` — register an ensemble over the current namespace.
fn ens_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(options) = interp
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "ensemble configuration options",
            )
            .into(),
        );
    };

    let ns = interp.current_ns();
    // Mapping targets use the implementation namespace; publication retains
    // the original -command operand and its independent token routing.
    let mut command = None;
    let mut cfg = EnsembleConfig {
        originals: Default::default(),
        ns,
        map: None,
        subcommands: None,
        prefixes: true,
        parameters: Vec::new(),
        unknown: Vec::new(),
    };

    let opts = &argv[3..];
    // C checks the pair arity before it looks at any option word
    // (`if (objc & 1)` → `wrong # args`, `tclEnsemble.c:192-196`).
    if opts.len() % 2 != 0 {
        return interp.wrong_args_for_prefix(argv, 2, b"create ?option value ...?");
    }
    for pair in opts.chunks_exact(2) {
        // `ensembleCreateOptions`: `-command` (create-only) names the ensemble
        // command, the rest are the shared configuration options, and there is
        // deliberately no `-namespace`.
        let resolved = match interp.native_static_string_option_index(
            pair[0],
            options.create_options(),
            false,
            "option",
        ) {
            Ok(index) => options.create_option(index),
            Err(error) => return interp.report_cmd_error(error),
        };
        let Some(shared) = resolved.shared() else {
            command = Some(match interp.native_string_bytes(&pair[1]) {
                Ok(bytes) => bytes,
                Err(error) => return interp.report_cmd_error(error.into()),
            });
            continue;
        };
        if let Err(e) = apply_ensemble_option(&mut cfg, shared, pair[1], ns, interp, true) {
            return interp.report_cmd_error(e);
        }
    }

    let selected = interp
        .namespaces_mut()
        .ensemble_publication_at(ns, command.as_deref());
    let Some((holder, simple)) = selected else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "ensemble publication context",
            )
            .into(),
        );
    };
    let reporting = interp.namespaces().command_fqn_at(holder, &simple);
    interp.create_ensemble_in_slot(holder, &simple, cfg);
    interp.set_result_bytes(&reporting);
    Code::Ok
}

fn native_ensemble_map_prefix_input<'a>(
    interp: &Interp,
    namespace: NsId,
    original: &'a [u8],
) -> Result<std::borrow::Cow<'a, [u8]>, tcl_syntax::value::ValueError> {
    let unavailable = tcl_syntax::value::ValueError::CommandProtocolUnavailable(
        "ensemble map-prefix construction",
    );
    let protocol = interp
        .native_invocation_dialect()
        .native_name_protocol()
        .ok_or_else(|| unavailable.clone())?;
    let report = interp.namespaces().qualified_name(namespace);
    let has_parent = interp.namespaces().parent(namespace).is_some();
    protocol
        .ensemble_map_prefix_input(&report, has_parent, original)
        .map_err(|_| unavailable)
}

/// Apply one already-resolved shared `-option value` to an
/// [`EnsembleConfig`] (`namespace ensemble create` and `configure` both land
/// here; the option word itself is resolved by the caller's own table).
/// Returns the C error text on a bad value.
///
/// `map_ns` is the namespace unqualified `-map` targets are resolved against.
/// C uses the ensemble's own namespace on `create` and the namespace current
/// at the call on `configure`; both are the current namespace at the point
/// each command runs, which is what every caller passes.
fn apply_ensemble_option(
    cfg: &mut EnsembleConfig,
    opt: tcl_cmd_core::ensemble::SharedOption,
    value: *mut TclObj,
    map_ns: NsId,
    interp: &Interp,
    creating: bool,
) -> Result<(), tcl_cmd_core::CmdError> {
    use tcl_cmd_core::ensemble::{EnsembleObjectRole, SharedOption};
    if opt == SharedOption::Prefixes {
        cfg.prefixes =
            crate::typed_value::native_boolean(value, interp.native_invocation_dialect())?;
        return Ok(());
    }
    let protocol = interp
        .eval_frame_dialect()
        .native_string_materialization(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "ensemble configured objects",
        ))?
        .protocol();
    if opt == SharedOption::Map {
        let pairs = crate::dict::native_dict_pairs(value, protocol)?;
        let mut patched: Option<obj::Owned> = None;
        let mut map = Vec::with_capacity(pairs.len());
        for (key, prefix) in pairs {
            let mut members = list::list_elements_native_checked(prefix, protocol)?;
            if members.is_empty() {
                return Err(tcl_cmd_core::CmdError::with_error_code_bytes(
                    b"ensemble subcommand implementations must be non-empty lists".to_vec(),
                    b"TCL ENSEMBLE EMPTY_TARGET".to_vec(),
                ));
            }
            let target = crate::dict::native_object_bytes(members[0], protocol)?;
            let selected = native_ensemble_map_prefix_input(interp, map_ns, &target)?;
            if let std::borrow::Cow::Owned(qualified) = selected {
                let qualified = obj::Owned::fresh(obj::new_string_bytes(&qualified));
                let copy = obj::Owned::fresh(if creating {
                    list::new_list_obj_native(&members, protocol)
                } else {
                    obj::duplicate(prefix)
                });
                members[0] = qualified.as_ptr();
                let replacement = list::replace_elements_native(copy.as_ptr(), &members, protocol)?;
                let root = patched.get_or_insert_with(|| obj::Owned::fresh(obj::duplicate(value)));
                crate::dict::dict_set(root.as_ptr(), key, replacement.as_ptr()).map_err(
                    |error| tcl_cmd_core::CmdError::new_bytes(error.message_bytes().to_vec()),
                )?;
            }
            let name = crate::dict::native_object_bytes(key, protocol)?;
            let words = members
                .iter()
                .map(|&word| crate::dict::native_object_bytes(word, protocol))
                .collect::<Result<Vec<_>, _>>()?;
            map.push((name, words));
        }
        cfg.map = (!map.is_empty()).then_some(map);
        cfg.originals.map = cfg.map.as_ref().map(|_| {
            EnsembleObjectRole::new(patched.map_or_else(
                || crate::ensemble::NativeEnsembleRoot::pending(value),
                crate::ensemble::NativeEnsembleRoot::owned,
            ))
        });
        return Ok(());
    }
    let members = list::list_elements_native_checked(value, protocol)?;
    let words = members
        .iter()
        .map(|&word| crate::dict::native_object_bytes(word, protocol))
        .collect::<Result<Vec<_>, _>>()?;
    let role = (!members.is_empty())
        .then(|| EnsembleObjectRole::new(crate::ensemble::NativeEnsembleRoot::pending(value)));
    match opt {
        SharedOption::Subcommands => {
            cfg.subcommands = (!words.is_empty()).then_some(words);
            cfg.originals.subcommands = role;
        }
        SharedOption::Parameters => {
            cfg.parameters = words;
            cfg.originals.parameters = role;
        }
        SharedOption::Unknown => {
            cfg.unknown = words;
            cfg.originals.unknown = role;
        }
        SharedOption::Map | SharedOption::Prefixes => unreachable!("handled above"),
    }
    Ok(())
}

/// `namespace ensemble configure cmd ?-option? ?value …?` — read or update an
/// existing ensemble's configuration (`tclEnsemble.c`). No options: a dict of
/// all settings; one bare `-option`: its value; `-option value …` pairs: update.
fn ens_configure(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(options) = interp
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "ensemble configuration options",
            )
            .into(),
        );
    };

    if argv.len() < 4 {
        return interp
            .wrong_args(b"namespace ensemble configure cmdname ?-option value ...? ?arg ...?");
    }
    let cmd = obj_bytes(argv[3]);
    // Follow a `namespace import` alias to the ensemble that owns the config:
    // reads and writes both act on the origin, so both spellings observe one
    // configuration and the alias stays an alias.
    let Some(token) = interp.ensemble_config_at(&cmd) else {
        // Distinguish a missing command from a non-ensemble one (C's wording).
        if interp.command_exists(&cmd) {
            let mut m = b"\"".to_vec();
            m.extend_from_slice(&cmd);
            m.extend_from_slice(b"\" is not an ensemble command");
            let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"ENSEMBLE", &cmd]);
            return interp.error_with_code(&m, &error_code);
        }
        let mut m = b"unknown command \"".to_vec();
        m.extend_from_slice(&cmd);
        m.push(b'"');
        let error_code = error_code_list(&[b"TCL", b"LOOKUP", b"COMMAND", &cmd]);
        return interp.error_with_code(&m, &error_code);
    };
    let mut cfg = token.config();
    let rest = &argv[4..];
    // Native configure returns the ordered key/value List.
    if rest.is_empty() {
        let d = match ensemble_config_list(interp, &cfg) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        // The checked owned List remains live through result publication.
        unsafe {
            interp.set_obj_result(d.as_ptr());
        }
        return Code::Ok;
    }
    // Read a single option's value (`ensembleConfigOptions`, abbreviating).
    if rest.len() == 1 {
        return match interp.native_static_string_option_index(
            rest[0],
            options.configure_options(),
            false,
            "option",
        ) {
            Ok(index) => match ensemble_option_value(interp, &cfg, options.configure_option(index))
            {
                Ok(value) => {
                    // The selected original option root is retained by this owner.
                    unsafe {
                        interp.set_obj_result(value.as_ptr());
                    }
                    Code::Ok
                }
                Err(error) => interp.report_cmd_error(error.into()),
            },
            Err(error) => interp.report_cmd_error(error),
        };
    }
    // Update: `-option value` pairs. C's arity gate is
    // `objc != 4 && !(objc & 1)`, i.e. anything but 0, 1, or an even number of
    // trailing words is `wrong # args`.
    if rest.len() % 2 != 0 {
        return interp
            .wrong_args(b"namespace ensemble configure cmdname ?-option value ...? ?arg ...?");
    }
    // CONF_MAP qualifies against `TclGetCurrentNamespace(interp)` — the
    // namespace current at the `configure` call, NOT the ensemble's own
    // namespace (which CRT_MAP uses at create time). They coincide in the
    // common `namespace eval M {namespace ensemble configure …}` shape, but
    // configuring an ensemble from outside its namespace resolves relative
    // targets against the caller.
    let map_ns = interp.current_ns();
    for pair in rest.chunks_exact(2) {
        let resolved = match interp.native_static_string_option_index(
            pair[0],
            options.configure_options(),
            false,
            "option",
        ) {
            Ok(index) => options.configure_option(index),
            Err(error) => return interp.report_cmd_error(error),
        };
        let Some(shared) = resolved.shared() else {
            return interp
                .error_with_code(b"option -namespace is read-only", b"TCL ENSEMBLE READ_ONLY");
        };
        if let Err(e) = apply_ensemble_option(&mut cfg, shared, pair[1], map_ns, interp, false) {
            return interp.report_cmd_error(e);
        }
    }
    cfg.originals.activate();
    token.configure(cfg);
    interp.note_native_ensemble_configuration_changed(&token);
    interp.set_result_bytes(b"");
    Code::Ok
}

/// One ensemble `configure`/cget option's value (string form).
fn ensemble_option_value(
    interp: &Interp,
    cfg: &EnsembleConfig,
    opt: tcl_cmd_core::ensemble::ConfigOption,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    use tcl_cmd_core::ensemble::ConfigOption;
    let original = match opt {
        ConfigOption::Map => &cfg.originals.map,
        ConfigOption::Unknown => &cfg.originals.unknown,
        ConfigOption::Parameters => &cfg.originals.parameters,
        ConfigOption::Subcommands => &cfg.originals.subcommands,
        ConfigOption::Namespace | ConfigOption::Prefixes => &None,
    };
    if let Some(pointer) = crate::ensemble::NativeEnsembleObjects::pointer(original) {
        return Ok(obj::Owned::retain(pointer));
    }
    if opt == ConfigOption::Prefixes {
        return Ok(obj::Owned::fresh(obj::new_boolean_obj(i32::from(
            cfg.prefixes,
        ))));
    }
    let bytes = match opt {
        ConfigOption::Namespace => interp.namespaces().qualified_name(cfg.ns),
        ConfigOption::Prefixes => {
            if cfg.prefixes {
                b"1".to_vec()
            } else {
                b"0".to_vec()
            }
        }
        ConfigOption::Parameters => join_words(interp, &cfg.parameters)?,
        ConfigOption::Unknown => join_words(interp, &cfg.unknown)?,
        ConfigOption::Subcommands => cfg
            .subcommands
            .as_deref()
            .map(|words| join_words(interp, words))
            .transpose()?
            .unwrap_or_default(),
        ConfigOption::Map => match &cfg.map {
            Some(map) => {
                let mut words = Vec::new();
                for (key, prefix) in map {
                    words.push(key.clone());
                    words.push(join_words(interp, prefix)?);
                }
                join_words(interp, &words)?
            }
            None => Vec::new(),
        },
    };
    Ok(obj::Owned::fresh(obj::new_string_bytes(&bytes)))
}

fn join_words(
    interp: &Interp,
    words: &[Vec<u8>],
) -> Result<Vec<u8>, tcl_syntax::value::ValueError> {
    let protocol = interp
        .eval_frame_dialect()
        .native_string_materialization(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace ensemble configuration List string",
        ))?
        .protocol();
    let elements: Vec<_> = words
        .iter()
        .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)))
        .collect();
    let pointers: Vec<_> = elements.iter().map(obj::Owned::as_ptr).collect();
    let value = obj::Owned::fresh(list::new_list_obj_native(&pointers, protocol));
    crate::dict::native_object_bytes(value.as_ptr(), protocol)
}

fn ensemble_config_list(
    interp: &Interp,
    cfg: &EnsembleConfig,
) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
    let protocol = interp
        .eval_frame_dialect()
        .native_string_materialization(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "namespace ensemble configuration List",
        ))?
        .protocol();
    let mut members = Vec::new();
    let options = interp
        .native_invocation_dialect()
        .native_ensemble_configuration_protocol()
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "ensemble configuration options",
        ))?;
    for option in options.configuration_options() {
        members.push(obj::Owned::fresh(obj::new_string_bytes(
            option.name().as_bytes(),
        )));
        members.push(ensemble_option_value(interp, cfg, option)?);
    }
    let pointers: Vec<_> = members.iter().map(obj::Owned::as_ptr).collect();
    Ok(obj::Owned::fresh(list::new_list_obj_native(
        &pointers, protocol,
    )))
}

/// `namespace ensemble exists command` — 1 if it resolves to an ensemble.
fn ens_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"exists cmdname");
    }
    let exists = interp.is_ensemble(&obj_bytes(argv[3]));
    interp.set_result_bytes(if exists { b"1" } else { b"0" });
    Code::Ok
}

#[cfg(test)]
mod tests {
    use crate::interp::{Code, Interp};
    use crate::{counters, list, obj};

    #[test]
    fn origin_reuses_the_original_stringless_command_cache() {
        for version in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(interp.eval_str(b"proc p {} {return P}"), Code::Ok);
            let words = [b"namespace".as_slice(), b"origin", b"p"]
                .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)));
            let argv = words.each_ref().map(|word| word.as_ptr());
            assert_eq!(super::ns_origin(&mut interp, &argv), Code::Ok);
            let cache = obj::native_command_name_cache(argv[2]).unwrap();
            obj::invalidate_string(argv[2]);
            assert_eq!(super::ns_origin(&mut interp, &argv), Code::Ok);
            assert_eq!(interp.result_bytes(), b"::p");
            assert!(!obj::has_string_rep(argv[2]));
            assert_eq!(obj::native_command_name_cache(argv[2]), Some(cache));
            assert!(!interp.host_refusal_pending());
        }
    }

    #[test]
    fn origin_refuses_a_stringless_original_namespace_name_before_lookup() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.1"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"namespace current"), Code::Ok);
        let original = obj::Owned::retain(interp.result_obj());
        assert!(obj::native_namespace_name::cache(original.as_ptr()).is_some());
        obj::invalidate_string(original.as_ptr());
        interp.set_result_bytes(b"BEFORE");
        let before = interp.result_obj();
        let head = obj::Owned::fresh(obj::new_string_bytes(b"namespace"));
        let subcommand = obj::Owned::fresh(obj::new_string_bytes(b"origin"));
        assert_eq!(
            super::ns_origin(
                &mut interp,
                &[head.as_ptr(), subcommand.as_ptr(), original.as_ptr()]
            ),
            Code::Error
        );
        assert!(interp.host_refusal_pending());
        assert_eq!(interp.result_obj(), before);
        assert!(!obj::has_string_rep(original.as_ptr()));
    }

    #[test]
    fn original_namespace_origin_matches_five_native_import_chains() {
        // Native proof: naming.namespace-origin.script-import-chain
        // docs/design/analysis/name-resolution-proofs/namespace-origin.script-import-chain.md
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for row in include_str!("../tests/data/native_namespace_origin/windows.tsv").lines() {
            let fields = row.split('\t').collect::<Vec<_>>();
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(&format!("tcl{}", fields[0])),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            assert_eq!(
                interp.eval_str(&decode(fields[1])).as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{}",
                fields[0]
            );
            assert_eq!(interp.result_bytes(), decode(fields[3]), "{}", fields[0]);
            assert!(!interp.host_refusal_pending());
            compared += 1;
        }
        assert_eq!(compared, 5);
    }

    #[test]
    fn namespace_unknown_retains_the_original_root_and_validates_before_replacement() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                let command = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"namespace"));
                let subcommand = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"unknown"));
                let original =
                    crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"::unknown tag"));
                let prefix = [command.as_ptr(), subcommand.as_ptr()];
                assert_eq!(
                    super::ns_unknown(interp, &[prefix[0], prefix[1], original.as_ptr()]),
                    Code::Ok
                );
                assert_eq!(interp.result_obj(), original.as_ptr());
                assert_eq!(super::ns_unknown(interp, &prefix), Code::Ok);
                assert_eq!(interp.result_obj(), original.as_ptr());
                let malformed = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"{"));
                assert_eq!(
                    super::ns_unknown(interp, &[prefix[0], prefix[1], malformed.as_ptr()]),
                    Code::Error
                );
                assert_eq!(super::ns_unknown(interp, &prefix), Code::Ok);
                assert_eq!(interp.result_obj(), original.as_ptr());
                let empty = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b" \t "));
                assert_eq!(
                    super::ns_unknown(interp, &[prefix[0], prefix[1], empty.as_ptr()]),
                    Code::Ok
                );
                assert_eq!(interp.result_obj(), empty.as_ptr());
                assert_eq!(super::ns_unknown(interp, &prefix), Code::Ok);
                assert_ne!(interp.result_obj(), original.as_ptr());
                assert_eq!(interp.result_bytes(), b"::unknown");
            });
        }
    }

    #[test]
    fn namespace_unknown_keeps_reached_prefix_members_after_handler_reset() {
        pins(
            br#"namespace eval N {
            proc handler {tag cmd args} {namespace unknown {}; return [list $tag $cmd $args]}
            namespace unknown {::N::handler retained}
            missing arg
        }"#,
            b"retained missing arg",
        );
    }

    #[test]
    fn ensemble_original_map_queries_do_not_extend_native_owner_lifetime() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                let arguments: Vec<_> = [
                    b"namespace".as_slice(),
                    b"ensemble",
                    b"create",
                    b"-command",
                    b"::E",
                    b"-map",
                    b"a ::target",
                ]
                .into_iter()
                .map(|word| crate::obj::Owned::fresh(crate::obj::new_string_bytes(word)))
                .collect();
                let pointers: Vec<_> = arguments.iter().map(crate::obj::Owned::as_ptr).collect();
                let original = arguments[6].as_ptr();
                assert_eq!(super::ens_create(interp, &pointers), Code::Ok);
                let token = interp.ensemble_config_at(b"::E").unwrap();
                let query = token.config();
                assert_eq!(
                    crate::ensemble::NativeEnsembleObjects::pointer(&query.originals.map),
                    Some(original)
                );
                // SAFETY: argv's owning handle retains this original header.
                let references = unsafe { (*original).ref_count };
                let another_query = query.clone();
                assert_eq!(unsafe { (*original).ref_count }, references);
                let configure: Vec<_> = [
                    b"namespace".as_slice(),
                    b"ensemble",
                    b"configure",
                    b"::E",
                    b"-map",
                ]
                .into_iter()
                .map(|word| crate::obj::Owned::fresh(crate::obj::new_string_bytes(word)))
                .collect();
                let configure: Vec<_> = configure.iter().map(crate::obj::Owned::as_ptr).collect();
                assert_eq!(super::ens_configure(interp, &configure), Code::Ok);
                assert_eq!(interp.result_obj(), original);
                interp.set_result_bytes(b"");
                assert!(interp.delete_command(b"::E"));
                assert!(
                    crate::ensemble::NativeEnsembleObjects::pointer(&query.originals.map).is_none()
                );
                assert!(
                    crate::ensemble::NativeEnsembleObjects::pointer(&another_query.originals.map)
                        .is_none()
                );
                assert_eq!(unsafe { (*original).ref_count }, references - 1);
            });
        }
    }

    #[test]
    fn original_ensemble_options_use_actual_release_tables() {
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                let arguments: Vec<_> = [
                    b"namespace".as_slice(),
                    b"ensemble",
                    b"create",
                    b"-command",
                    b"::E",
                ]
                .into_iter()
                .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                .collect();
                assert_eq!(
                    super::ens_create(
                        interp,
                        &arguments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()
                    ),
                    Code::Ok
                );
                for (option, expected) in [
                    (
                        b"-parameters".as_slice(),
                        if version == tcl_dialect::TclVersion::V8_5 {
                            Code::Error
                        } else {
                            Code::Ok
                        },
                    ),
                    (
                        b"-p".as_slice(),
                        if version == tcl_dialect::TclVersion::V8_5 {
                            Code::Ok
                        } else {
                            Code::Error
                        },
                    ),
                ] {
                    let arguments: Vec<_> = [
                        b"namespace".as_slice(),
                        b"ensemble",
                        b"configure",
                        b"::E",
                        option,
                    ]
                    .into_iter()
                    .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                    .collect();
                    assert_eq!(
                        super::ens_configure(
                            interp,
                            &arguments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()
                        ),
                        expected
                    );
                }
                let arguments: Vec<_> =
                    [b"namespace".as_slice(), b"ensemble", b"configure", b"::E"]
                        .into_iter()
                        .map(|word| obj::Owned::fresh(obj::new_string_bytes(word)))
                        .collect();
                assert_eq!(
                    super::ens_configure(
                        interp,
                        &arguments.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>()
                    ),
                    Code::Ok
                );
                let protocol = interp
                    .native_invocation_dialect()
                    .native_string_materialization(None)
                    .unwrap()
                    .protocol();
                assert_eq!(
                    list::list_elements_native_checked(interp.result_obj(), protocol)
                        .unwrap()
                        .len(),
                    if version == tcl_dialect::TclVersion::V8_5 {
                        10
                    } else {
                        12
                    }
                );
            });
        }
    }

    #[test]
    fn explicit_ensemble_members_and_unknown_relative_context_match_native_c() {
        for version in [
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                let source = br#"proc target {} {return GLOBAL_NS}
proc return_relative {args} {return {target}}
namespace eval ::n {
    proc target {} {return ENSEMBLE_NS}
    namespace ensemble create -command ::e -map {a ::n::target b ::n::target} -subcommands a
    namespace ensemble create -command ::fallback -unknown ::return_relative
}
namespace eval ::caller {
    proc target {} {return CALLER_NS}
    list [catch {::e b} result] $result [::fallback absent]
}"#;
                assert_eq!(
                    interp.eval_str(source),
                    Code::Ok,
                    "{version:?}: {:?}",
                    interp.result_bytes()
                );
                let selected = if version == tcl_dialect::TclVersion::V8_5 {
                    b"GLOBAL_NS".as_slice()
                } else {
                    b"ENSEMBLE_NS"
                };
                let mut expected =
                    b"1 {unknown or ambiguous subcommand \"b\": must be a} ".to_vec();
                expected.extend_from_slice(selected);
                assert_eq!(interp.result_bytes(), expected, "{version:?}");
            });
        }
    }

    #[test]
    fn ensemble_configuration_preserves_native_raw_byte_list_words() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            leak_free(|interp| {
                interp.set_runtime_version(version);
                assert_eq!(
                    interp.eval_str(b"namespace ensemble create -command ::E"),
                    Code::Ok
                );
                let token = interp.ensemble_config_at(b"::E").unwrap();
                let mut config = token.config();
                let original = b"#\xff\0x y".to_vec();
                config.subcommands = Some(vec![original.clone()]);
                config.map = Some(vec![(
                    b"key".to_vec(),
                    vec![b"::target".to_vec(), original.clone()],
                )]);
                token.configure(config);
                for (option, expected) in [
                    (b"-subcommands".as_slice(), b"{#\xff\0x y}".as_slice()),
                    (
                        b"-map".as_slice(),
                        b"key {::target {#\xff\0x y}}".as_slice(),
                    ),
                ] {
                    let arguments: Vec<_> = [
                        b"namespace".as_slice(),
                        b"ensemble".as_slice(),
                        b"configure".as_slice(),
                        b"::E".as_slice(),
                        option,
                    ]
                    .into_iter()
                    .map(|bytes| crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes)))
                    .collect();
                    let pointers: Vec<_> =
                        arguments.iter().map(crate::obj::Owned::as_ptr).collect();
                    assert_eq!(super::ens_configure(interp, &pointers), Code::Ok);
                    // Pinned C8.5/C8.6/C9.0/C9.1 Tcl_EvalObjv observations,
                    // including the literal invalid UTF-8 byte and embedded NUL.
                    assert_eq!(interp.result_bytes(), expected, "{version:?}");
                }
                assert_eq!(token.config().subcommands, Some(vec![original]));
            });
        }
    }

    #[test]
    fn ensemble_configuration_refuses_an_unselected_list_updater() {
        let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
        )));
        leak_free(|interp| {
            interp.set_dialect_profile(unknown);
            interp.set_result_bytes(b"UNCHANGED");
            let error = super::join_words(interp, &[b"opaque\xff".to_vec()]).unwrap_err();
            assert!(error.native_access_refusal().is_some());
            assert_eq!(interp.result_bytes(), b"UNCHANGED");
        });
    }

    #[test]
    fn current_and_code_wrong_arity_preserve_native_header_bytes() {
        for version in tcl_dialect::TclVersion::ALL {
            let mut interp = Interp::new();
            interp.set_runtime_version(version);
            let command =
                crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"n s\0tail\xc0\x80\xff"));
            let member = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"current"));
            let extra = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"extra"));
            let expected: &[u8] = match version {
                tcl_dialect::TclVersion::V8_4 => b"n s",
                tcl_dialect::TclVersion::V8_5 | tcl_dialect::TclVersion::V8_6 => {
                    b"n s\0tail\xc0\x80\xff"
                }
                tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1 => {
                    b"{n s\0tail\xc0\x80\xff}"
                }
            };
            assert_eq!(
                super::ns_current(
                    &mut interp,
                    &[command.as_ptr(), member.as_ptr(), extra.as_ptr()]
                ),
                Code::Error
            );
            let mut message = b"wrong # args: should be \"".to_vec();
            message.extend_from_slice(expected);
            message.extend_from_slice(b" current\"");
            assert_eq!(interp.result_bytes(), message);
            assert_eq!(
                interp.error_code(),
                interp
                    .native_invocation_dialect()
                    .wrong_arguments_error_code()
                    .unwrap()
                    .as_bytes()
            );
            let member = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"code"));
            assert_eq!(
                super::ns_code(&mut interp, &[command.as_ptr(), member.as_ptr()]),
                Code::Error
            );
            let mut message = b"wrong # args: should be \"".to_vec();
            message.extend_from_slice(expected);
            message.extend_from_slice(b" code arg\"");
            assert_eq!(interp.result_bytes(), message);
            assert_eq!(
                crate::obj::bytes_of(command.as_ptr()),
                b"n s\0tail\xc0\x80\xff"
            );
        }
    }

    #[test]
    fn code_preserves_original_argument_objects_for_all_native_prefix_fixtures() {
        let specimens: [&[u8]; 11] = [
            b"::namespace inscope ",
            b"::namespace inscope :: cmd",
            b"namespace inscope :: cmd",
            b":namespace inscope :: cmd",
            b"::::namespace inscope :: cmd",
            b"namespace    inscope :: cmd",
            b"namespace\tinscope :: cmd",
            b"namespaceinscopeXX",
            b"namespaceinscopeX",
            b"::namespace inscopeX",
            b"::namespace inscope \0",
        ];
        let jim = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let mut rows = 0;
        for profile in tcl_dialect::TclVersion::ALL
            .map(|version| {
                tcl_dialect::DialectProfile::find(version.dialect_profile_name())
                    .expect("C profile")
            })
            .into_iter()
            .chain(std::iter::once(&*jim))
        {
            let preserved: &[usize] = if std::ptr::eq(profile, &*jim) {
                &[0, 1, 10]
            } else if profile.vm_runtime_version == tcl_dialect::TclVersion::V8_4 {
                &[0, 1, 2, 3, 4, 5, 7, 9, 10]
            } else {
                &[1, 10]
            };
            leak_free(|interp| {
                interp.set_dialect_profile(profile);
                let namespace = crate::obj::Owned::fresh(crate::interp::new_string(b"namespace"));
                let code = crate::obj::Owned::fresh(crate::interp::new_string(b"code"));
                for (index, bytes) in specimens.into_iter().enumerate() {
                    let script = crate::obj::Owned::fresh(crate::interp::new_string(bytes));
                    assert_eq!(
                        super::ns_code(
                            interp,
                            &[namespace.as_ptr(), code.as_ptr(), script.as_ptr()]
                        ),
                        Code::Ok
                    );
                    if preserved.contains(&index) {
                        assert_eq!(
                            interp.result_obj(),
                            script.as_ptr(),
                            "{}: {index}",
                            profile.name
                        );
                    } else {
                        let elements = crate::list::list_elements(interp.result_obj()).unwrap();
                        assert_eq!(elements.len(), 4);
                        assert_eq!(super::obj_bytes(elements[0]), b"::namespace");
                        assert_eq!(super::obj_bytes(elements[1]), b"inscope");
                        assert_eq!(super::obj_bytes(elements[2]), b"::");
                        assert_eq!(elements[3], script.as_ptr(), "{}: {index}", profile.name);
                    }
                    assert_eq!(super::obj_bytes(script.as_ptr()), bytes);
                    rows += 1;
                }
            });
        }
        assert_eq!(rows, 66);
    }

    #[test]
    fn code_retains_non_unicode_bytes_and_refuses_unknown_native_policy() {
        leak_free(|interp| {
            interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            let namespace = crate::obj::Owned::fresh(crate::interp::new_string(b"namespace"));
            let code = crate::obj::Owned::fresh(crate::interp::new_string(b"code"));
            for bytes in [
                b"\xff\0script".as_slice(),
                b"::namespace inscope \xff\0".as_slice(),
            ] {
                let script = crate::obj::Owned::fresh(crate::interp::new_string(bytes));
                assert_eq!(
                    super::ns_code(
                        interp,
                        &[namespace.as_ptr(), code.as_ptr(), script.as_ptr()]
                    ),
                    Code::Ok
                );
                let retained = if interp.result_obj() == script.as_ptr() {
                    interp.result_obj()
                } else {
                    crate::list::list_elements(interp.result_obj()).unwrap()[3]
                };
                assert_eq!(retained, script.as_ptr());
                assert_eq!(super::obj_bytes(retained), bytes);
            }
            let unknown = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79),
            )));
            interp.set_dialect_profile(unknown);
            // Installing a profile performs no unavailable namespace-code
            // operation. The actual attempted purpose owns its typed refusal.
            assert_eq!(interp.native_access_refusal(), None);
            assert!(
                interp
                    .native_invocation_dialect()
                    .namespace_code_handler_policy(Some(
                        tcl_registry::native_namespace_code::LogicalNamespaceCodeProvider::Tcl84CoreSimulation,
                    ))
                    .is_none()
            );
            let script = crate::obj::Owned::fresh(crate::interp::new_string(b"puts example"));
            interp.set_result_bytes(b"UNCHANGED");
            assert_eq!(
                super::ns_code(
                    interp,
                    &[namespace.as_ptr(), code.as_ptr(), script.as_ptr()]
                ),
                Code::Error
            );
            assert_eq!(interp.result_bytes(), b"UNCHANGED");
            assert!(interp.host_refusal_pending());
            assert_eq!(
                interp.native_access_refusal(),
                Some(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "namespace code",
                    )
                )
            );
        });
    }

    #[test]
    fn code_uses_the_explicit_f5_logical_provider() {
        leak_free(|interp| {
            interp.set_dialect_profile(tcl_dialect::DialectProfile::irules());
            let namespace = crate::obj::Owned::fresh(crate::interp::new_string(b"namespace"));
            let code = crate::obj::Owned::fresh(crate::interp::new_string(b"code"));
            let script = crate::obj::Owned::fresh(crate::interp::new_string(b"namespaceinscopeXX"));
            assert_eq!(
                super::ns_code(
                    interp,
                    &[namespace.as_ptr(), code.as_ptr(), script.as_ptr()]
                ),
                Code::Ok
            );
            assert_eq!(interp.result_obj(), script.as_ptr());
            assert!(
                interp
                    .native_invocation_dialect()
                    .native_namespace_code_policy()
                    .is_none()
            );
        });
    }

    #[test]
    fn import_redefinition_rename_reexport_and_retirement_follow_dialect() {
        let jim = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        let script = br#"namespace eval S {proc p {} {return ONE}; namespace export *}; namespace eval D {namespace import ::S::*; namespace export *}; namespace eval E {namespace import ::D::*}; namespace eval S {proc p {} {return TWO}}; set result [list [D::p] [E::p] [namespace origin E::p]]; rename S::p S::moved; lappend result [catch {D::p} value] [catch {namespace origin E::p} value]; proc S::p {} {return THREE}; lappend result [D::p] [E::p]; rename S::moved {}; lappend result [llength [info commands D::p]] [llength [info commands E::p]]; set result"#;
        for profile in tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(|release| {
                tcl_dialect::DialectProfile::find(release.dialect_profile_name())
                    .expect("C profile")
            })
            .chain(std::iter::once(&*jim))
        {
            leak_free(|interp| {
                interp.set_dialect_profile(profile);
                assert_eq!(
                    interp.eval_str(script),
                    Code::Ok,
                    "{}: {:?}",
                    profile.name,
                    interp.result_bytes()
                );
                let expected = if profile.namespace_import_binding()
                    == Some(tcl_dialect::NamespaceImportBinding::SourceName)
                {
                    b"TWO TWO ::S::p 1 1 THREE THREE 1 1".as_slice()
                } else {
                    b"TWO TWO ::S::p 0 0 TWO TWO 0 0".as_slice()
                };
                assert_eq!(interp.result_bytes(), expected, "{}", profile.name);
            });
        }
    }

    #[test]
    fn jim_import_ignores_exports_overwrites_and_retains_missing_source_alias() {
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        )));
        leak_free(|interp| {
            interp.set_dialect_profile(profile);
            let script = br#"namespace eval S {proc p {} {return SOURCE}}; namespace eval D {proc p {} {return DESTINATION}; namespace import -force ::S::*; set imported [namespace import]}; rename S::p {}; set result [list [llength [info commands D::p]] [catch {D::p}] [catch {namespace origin D::p}] [namespace eval D {namespace import ::missing::*}] [namespace eval D {namespace export}]]; proc S::p {} {return REVIVED}; lappend result [D::p] [namespace origin D::p] [catch {namespace path}]; set result"#;
            assert_eq!(
                interp.eval_str(script),
                Code::Ok,
                "{:?}",
                interp.result_bytes()
            );
            assert_eq!(interp.result_bytes(), b"1 1 1 {} {} REVIVED ::S::p 1");
        });
    }

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    /// `namespace` is a `TclMakeEnsemble` command, so its exact-then-unique-
    /// prefix scan and its whole miss sentence belong to
    /// `tcl_cmd_core::ensemble`, rather than a hand-rolled scan with the
    /// 19-entry enumeration duplicated as a literal beside the table that
    /// already owns it.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   namespace cu -> ::
    ///   namespace {} -> unknown or ambiguous subcommand "": must be children,
    ///                   code, current, delete, ensemble, eval, exists, export,
    ///                   forget, import, inscope, origin, parent, path,
    ///                   qualifiers, tail, unknown, upvar, or which
    ///   namespace e  -> unknown or ambiguous subcommand "e": must be <same>
    ///                   (ensemble/eval/exists/export)
    #[test]
    fn namespace_ensemble_miss_comes_from_the_owner() {
        const MUST: &str = "must be children, code, current, delete, ensemble, eval, exists, \
                            export, forget, import, inscope, origin, parent, path, qualifiers, \
                            tail, unknown, upvar, or which";
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace cu"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::");
            assert_eq!(i.eval_str(b"namespace {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("unknown or ambiguous subcommand \"\": {MUST}").as_bytes()
            );
            assert_eq!(i.eval_str(b"namespace e"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("unknown or ambiguous subcommand \"e\": {MUST}").as_bytes()
            );
        });
    }

    #[test]
    fn current_is_global_at_top_level() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace current"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::");
        });
    }

    #[test]
    fn eval_switches_current_and_defines_in_ns() {
        leak_free(|i| {
            // A command defined inside `namespace eval` lands in that ns.
            assert_eq!(
                i.eval_str(b"namespace eval foo { set ::probe [namespace current] }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"set ::probe"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::foo");
            // Back at top level current is global again.
            assert_eq!(i.eval_str(b"namespace current"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::");
        });
    }

    /// `-prefixes` reads its value through the runtime's one boolean
    /// acceptor, so every spelling `tclsh9.0` accepts here is accepted: a
    /// unique word prefix, and any number against zero. Only the ambiguous
    /// `o` — shared by `on` and `off` — is refused.
    #[test]
    fn ensemble_prefixes_accepts_every_boolean_spelling_tclsh_does() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval ens { proc bar {} {}; namespace export bar; \
                      namespace ensemble create }"
                ),
                Code::Ok
            );
            for (spelling, expected) in [
                (&b"tru"[..], b"1".as_slice()),
                (b"ye", b"1"),
                (b"of", b"0"),
                (b"2", b"1"),
                (b"0.0", b"0"),
            ] {
                let mut script = b"namespace ensemble configure ::ens -prefixes ".to_vec();
                script.extend_from_slice(spelling);
                assert_eq!(
                    i.eval_str(&script),
                    Code::Ok,
                    "{}: {}",
                    String::from_utf8_lossy(spelling),
                    String::from_utf8_lossy(&i.result_bytes())
                );
                assert_eq!(
                    i.eval_str(b"namespace ensemble configure ::ens -prefixes"),
                    Code::Ok
                );
                assert_eq!(i.result_bytes(), expected);
            }
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::ens -prefixes o"),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"expected boolean value but got \"o\"");
        });
    }

    #[test]
    fn exists_and_parent_and_children() {
        leak_free(|i| {
            i.eval_str(b"namespace eval a { namespace eval b {} }");
            assert_eq!(i.eval_str(b"namespace exists ::a::b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"namespace exists ::a::nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"namespace parent ::a::b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::a");
            assert_eq!(i.eval_str(b"namespace children ::a"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::a::b");

            assert_eq!(
                i.eval_str(
                    b"namespace eval ::a {
                          catch {namespace parent {not here}} pm po
                          catch {namespace children {also not here}} cm co
                          list $pm [dict get $po -errorcode] \
                               $cm [dict get $co -errorcode]
                      }"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{namespace \"not here\" not found in \"::a\"} \
                  {TCL LOOKUP NAMESPACE {not here}} \
                  {namespace \"also not here\" not found in \"::a\"} \
                  {TCL LOOKUP NAMESPACE {also not here}}"
            );
        });
    }

    #[test]
    fn qualifiers_and_tail() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"namespace qualifiers ::foo::bar::baz"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::foo::bar");
            assert_eq!(i.eval_str(b"namespace tail ::foo::bar::baz"), Code::Ok);
            assert_eq!(i.result_bytes(), b"baz");
            assert_eq!(i.eval_str(b"namespace qualifiers plain"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(i.eval_str(b"namespace tail plain"), Code::Ok);
            assert_eq!(i.result_bytes(), b"plain");
        });
    }

    #[test]
    fn subcommand_prefix_abbreviation() {
        leak_free(|i| {
            // `namespace` subcommands accept unambiguous prefixes.
            assert_eq!(i.eval_str(b"namespace exist ::nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"namespace cur"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::");
            // An ambiguous prefix still errors.
            assert_eq!(i.eval_str(b"namespace ex foo"), Code::Error);
        });
    }

    #[test]
    fn which_resolves_command_fqn() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace which -command set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
            assert_eq!(i.eval_str(b"namespace which nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            // Prefix abbreviation of the flags (`-com`, `-var`).
            assert_eq!(i.eval_str(b"namespace which -com set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
        });
    }

    #[test]
    fn which_variable_resolves_namespace_var_fqn() {
        leak_free(|i| {
            // `namespace which -variable` resolves a namespace variable to its
            // FQN (ignoring local proc links); a missing one yields "".
            assert_eq!(
                i.eval_str(b"namespace eval ::n { variable gv 1 }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace which -variable ::n::gv"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::n::gv");
            assert_eq!(i.eval_str(b"namespace which -var ::n::gv"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::n::gv");
            assert_eq!(i.eval_str(b"namespace which -variable ::n::nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    /// Vectors measured against tclsh 9.0.4. These exercise the runtime
    /// adapter through the same shared namespace grammar as the bytecode VM.
    #[test]
    fn namespace_issue_1584_oracle_vectors() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval declared {variable only
                      list [namespace which -variable only] [info vars] [info exists only]}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::declared::only only 0");

            for script in [
                &b"namespace which -zork puts"[..],
                b"namespace which -command puts extra",
            ] {
                assert_eq!(i.eval_str(script), Code::Error);
                assert_eq!(
                    i.result_bytes(),
                    b"wrong # args: should be \"namespace which ?-command? ?-variable? name\""
                );
            }

            assert_eq!(
                i.eval_str(b"namespace eval self {namespace import ::self::*}"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"import pattern \"::self::*\" tries to import from namespace \"self\" into itself"
            );
            assert_eq!(
                i.eval_str(b"namespace eval dest {namespace import ::nosuch::*}"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"unknown namespace in import pattern \"::nosuch::*\""
            );

            for script in [&b"namespace origin"[..], b"namespace origin set extra"] {
                assert_eq!(i.eval_str(script), Code::Error);
                assert_eq!(
                    i.result_bytes(),
                    b"wrong # args: should be \"namespace origin name\""
                );
            }

            assert_eq!(
                i.eval_str(
                    b"namespace eval order {}
                      foreach n {one two three four five six seven eight nine ten} {
                          namespace eval ::order::$n {}
                      }
                      namespace children ::order"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"::order::six ::order::four ::order::three ::order::eight \
                  ::order::seven ::order::nine ::order::five ::order::two \
                  ::order::one ::order::ten"
            );

            assert_eq!(
                i.eval_str(
                    b"namespace eval p {}
                      foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {
                          namespace eval ::p::$n {}
                      }
                      foreach i {1 2 4 5 6 7 8 9 10 11} {namespace delete ::p::a$i}
                      namespace children ::p"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::p::a0 ::p::a3");
        });

        // The active command survives long enough to return even though the
        // global namespace and command table have been torn down.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace delete ::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(i.eval_str(b"puts hi"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"puts\"");
        });
    }

    #[test]
    fn path_fallback_resolves_unqualified() {
        leak_free(|i| {
            // A command in ::lib (the alias creates ::lib), then ::lib on ::app's path.
            i.eval_str(b"interp alias {} ::lib::ping {} set pinged");
            assert_eq!(
                i.eval_str(b"namespace eval app { namespace path ::lib }"),
                Code::Ok
            );
            // Now from ::app, bare `ping` resolves through the path. Its body
            // `set pinged` runs with the current namespace = ::app, so the
            // variable lands in ::app's table, not the global frame.
            assert_eq!(i.eval_str(b"namespace eval app { ping yes }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"yes");
            assert_eq!(i.eval_str(b"set ::app::pinged"), Code::Ok);
            assert_eq!(i.result_bytes(), b"yes");
            // …and it is NOT visible as a bare global.
            assert_eq!(i.eval_str(b"set pinged"), Code::Error);
        });
    }

    #[test]
    fn export_import_forget_roundtrip() {
        leak_free(|i| {
            // ::lib exports g* ; provide a command ::lib::greet via an alias.
            i.eval_str(b"interp alias {} ::lib::greet {} set greeted");
            i.eval_str(b"namespace eval lib { namespace export g* }");
            // Import into ::app, then call the bare name there.
            assert_eq!(
                i.eval_str(b"namespace eval app { namespace import ::lib::* }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace eval app { greet hi }"), Code::Ok);
            // `set greeted` ran in ::app (the call's current namespace).
            assert_eq!(i.eval_str(b"set ::app::greeted"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hi");
            // Forget removes the redirect.
            assert_eq!(
                i.eval_str(b"namespace eval app { namespace forget ::lib::* }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace eval app { greet hi }"), Code::Error);
        });
    }

    #[test]
    fn reimport_same_source_is_idempotent() {
        // Re-importing the *same* command from the *same* source is a silent
        // no-op (C's reimport check) — the common case where a file and its
        // sourced helper both `namespace import ::lib::*` (e.g. tcltest). Only a
        // clobber of a *different* command is a conflict (without -force).
        leak_free(|i| {
            i.eval_str(b"namespace eval lib { proc g {} {return G} ; namespace export g }");
            assert_eq!(i.eval_str(b"namespace import ::lib::*"), Code::Ok);
            // Second import of the same command from the same source: no error.
            assert_eq!(i.eval_str(b"namespace import ::lib::*"), Code::Ok);
            assert_eq!(i.eval_str(b"namespace import ::lib::g"), Code::Ok);
            // A different command of the same simple name does conflict.
            i.eval_str(b"namespace eval other { proc g {} {return O} ; namespace export g }");
            assert_eq!(i.eval_str(b"namespace import ::other::*"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't import command \"g\": already exists"
            );
            // -force overrides the clobber.
            assert_eq!(i.eval_str(b"namespace import -force ::other::*"), Code::Ok);
        });
    }

    #[test]
    fn qualified_command_falls_back_to_global() {
        // A relative qualified command name resolves against the current
        // namespace, then the global one (C's `TclGetNamespaceForQualName`): so
        // `foo::bar` from inside `::a::b` finds `::foo::bar` when `::a::b::foo`
        // doesn't exist. (The bug: `tcl::build-info` failing inside a namespace.)
        leak_free(|i| {
            i.eval_str(b"namespace eval foo { proc bar {} {return BAR} }");
            assert_eq!(
                i.eval_str(b"namespace eval ::a::b { set ::r [foo::bar] }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"BAR");
            // A relative qualifier that *does* exist locally still wins.
            i.eval_str(b"namespace eval x::foo { proc bar {} {return LOCAL} }");
            assert_eq!(i.eval_str(b"namespace eval x { foo::bar }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"LOCAL");
            i.eval_str(b"unset -nocomplain ::r");
        });
    }

    #[test]
    fn qualifier_namespace_existing_does_not_commit_resolution() {
        // tclsh 8.6/9.0: the fallback is *command*-existence-checked, not
        // namespace-existence-checked.  `inner::p` from
        // `::outer` must dispatch `::inner::p` even though the namespace
        // `::outer::inner` exists — it merely holds no `p`.
        leak_free(|i| {
            i.eval_str(b"namespace eval inner { proc p {} {return GLOBAL} }");
            i.eval_str(b"namespace eval outer { namespace eval inner { proc other {} {} } }");
            assert_eq!(
                i.eval_str(b"namespace eval outer { inner::p }"),
                Code::Ok,
                "resolution must fall through to ::inner::p: {:?}",
                String::from_utf8_lossy(&i.result_bytes()),
            );
            assert_eq!(i.result_bytes(), b"GLOBAL");
        });
    }

    /// Every shared command-resolution vector
    /// (`tcl_syntax::naming::conformance`, pinned against real tclsh by
    /// the `tcl-syntax` conformance test) must dispatch identically
    /// through this runtime's namespace tree — the anti-drift gate for
    /// `Namespaces::home_of`.
    ///
    /// `builtins::install` only registers `if`/`while`/`for` under
    /// `have_tommath`, so capturing the call the way the shared
    /// `vector_script` renderer normally does — `if {[catch {…} __r]} {…}`
    /// — would report `invalid command name "if"` on the first vector when
    /// libtommath is not vendored. That is a *capture-script* dependency,
    /// not a dispatch one: nothing about command resolution needs the
    /// numeric tower.
    ///
    /// So the capture is composed here from the tower-free half of the
    /// renderer (`vector_setup` + `vector_call`) with `set` and `catch`
    /// standing in for `if` — `set __r -` then `catch {set __r [call]}`
    /// leaves the dispatched name in `__r`, or `-` when the call raised,
    /// which is exactly what the `if` form computes. This runs the vectors
    /// in **every** build of this crate, tower or no tower.
    #[test]
    fn dispatch_matches_every_conformance_vector() {
        use tcl_syntax::naming::conformance::{vector_call, vector_setup, vectors};
        for v in vectors() {
            let script = format!(
                "{}set __r -\ncatch {{set __r [{}]}}\n",
                vector_setup(&v),
                vector_call(&v),
            );
            let body = script.clone();
            counters::reset();
            {
                let mut i = Interp::new();
                let code = i.eval_str(body.as_bytes());
                assert_eq!(
                    code,
                    Code::Ok,
                    "vector line {}: runtime errored on script:\n{script}\nerror: {}",
                    v.line,
                    String::from_utf8_lossy(&i.result_bytes()),
                );
                assert_eq!(i.eval_str(b"set __r"), Code::Ok);
                let got = String::from_utf8_lossy(&i.result_bytes()).to_string();
                let want = v.want().unwrap_or_else(|| "-".to_string());
                assert_eq!(
                    got, want,
                    "vector line {} (ns={} path={:?} defs={:?} call={}): runtime dispatch \
                     disagrees with C Tcl\nscript:\n{script}",
                    v.line, v.ns, v.path, v.defs, v.call,
                );
            }
            assert_eq!(counters::finalize(), 0, "vector line {}: leak", v.line);
            assert_eq!(counters::double_free_count(), 0);
        }
    }

    /// Run `script` (which fills `::log` through delete-trace callbacks) and
    /// return the resulting log. Every expectation below is an exact tclsh
    /// 9.0.4 result, identical on 8.6.16 unless the test says otherwise.
    fn teardown_log(script: &str) -> String {
        let mut log = String::new();
        leak_free(|i| {
            let recorder = b"set log {}
                 proc rec {old new op} {lappend ::log [namespace tail $old]}";
            assert_eq!(i.eval_str(recorder), Code::Ok);
            assert_eq!(
                i.eval_str(script.as_bytes()),
                Code::Ok,
                "{}",
                String::from_utf8_lossy(&i.result_bytes())
            );
            assert_eq!(i.eval_str(b"set ::log"), Code::Ok);
            log = String::from_utf8_lossy(&i.result_bytes()).into_owned();
        });
        log
    }

    /// Define `names` as procs in `::N` and trace each one's deletion.
    fn traced_procs(names: &str) -> String {
        format!(
            "namespace eval N {{}}
             foreach n {{{names}}} {{
                 proc ::N::$n {{}} {{}}
                 trace add command ::N::$n delete rec
             }}"
        )
    }

    #[test]
    fn namespace_teardown_follows_command_table_hash_order() {
        // TclTeardownNamespace snapshots cmdTable with Tcl_FirstHashEntry, so
        // the delete traces fire in the retained TCL_STRING_KEYS bucket order,
        // not the definition or lexical order.
        assert_eq!(
            teardown_log(&format!(
                "{}
                 namespace delete ::N",
                traced_procs("one two three four five six seven eight nine ten")
            )),
            "six four three eight seven nine five two one ten"
        );
    }

    #[test]
    fn namespace_teardown_hash_order_records_table_growth() {
        // RebuildTable quadruples the bucket array once entries reach three
        // times the bucket count and re-pushes each chain head-first, which
        // reverses it. Thirteen commands cross the first threshold.
        assert_eq!(
            teardown_log(&format!(
                "{}
                 namespace delete ::N",
                traced_procs("c0 c1 c2 c3 c4 c5 c6 c7 c8 c9 c10 c11 c12")
            )),
            "c5 c6 c7 c8 c9 c0 c1 c10 c2 c11 c12 c3 c4"
        );
    }

    #[test]
    fn namespace_teardown_hash_order_retains_capacity_across_deletions() {
        // Tcl_DeleteHashEntry never shrinks the bucket array, so commands
        // created after a bulk deletion land in the grown table and precede
        // the survivors.
        assert_eq!(
            teardown_log(
                "namespace eval N {}
                 foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {proc ::N::$n {} {}}
                 foreach i {1 2 4 5 6 7 8 9 10 11} {rename ::N::a$i {}}
                 foreach n {b1 b2 b3} {proc ::N::$n {} {}}
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
            ),
            "b1 b2 b3 a0 a3"
        );
    }

    #[test]
    fn namespace_teardown_hash_order_moves_a_redefined_command() {
        // TclCreateObjCommandInNs deletes the existing hash entry and creates
        // a fresh one, so redefining `one` moves it to its bucket head.
        assert_eq!(
            teardown_log(
                "namespace eval N {}
                 foreach n {one two three four five six seven eight nine ten} {
                     proc ::N::$n {} {}
                 }
                 proc ::N::one {} {return again}
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
            ),
            "six four three eight seven one nine five two ten"
        );
        // TclRenameCommand creates the destination entry the same way.
        assert_eq!(
            teardown_log(
                "namespace eval N {}
                 foreach n {one two three four five six seven eight nine ten} {
                     proc ::N::$n {} {}
                 }
                 rename ::N::one ::N::uno
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
            ),
            "six four three eight seven uno nine five two ten"
        );
    }

    #[test]
    fn namespace_teardown_visits_a_callback_created_command_in_the_next_pass() {
        // TclTeardownNamespace loops while cmdTable is non-empty, so a command
        // a delete callback creates is torn down by a second snapshot, after
        // every entry of the first one.
        assert_eq!(
            teardown_log(&format!(
                "{}
                 proc mk {{old new op}} {{
                     lappend ::log [namespace tail $old]
                     proc ::N::zz {{}} {{}}
                     trace add command ::N::zz delete rec
                 }}
                 trace add command ::N::six delete mk
                 namespace delete ::N",
                traced_procs("one two three four five six seven eight nine ten")
            )),
            "six six four three eight seven nine five two one ten zz"
        );
    }

    #[test]
    fn namespace_teardown_defers_a_callback_recreated_command_to_the_next_pass() {
        // A redefinition inside the delete callback unlinks the dying token's
        // entry itself (Tcl_DeleteCommandFromToken's CMD_DYING branch), so the
        // replacement is a distinct token the current snapshot no longer names.
        // Native proof: naming.namespace.original-command-holder-routing
        // docs/design/analysis/name-resolution-proofs/namespace-original-command-holder-routing.md
        // The original C86, C90 and C91 controls complete with this order.
        assert_eq!(
            teardown_log(&format!(
                "{}
                 proc remake {{old new op}} {{
                     proc ::N::two {{}} {{}}
                     trace add command ::N::two delete rec
                     lappend ::log remade
                 }}
                 trace add command ::N::two delete remake
                 namespace delete ::N",
                traced_procs("one two three four five six seven eight nine ten")
            )),
            "six four three eight seven nine five remade two one ten two"
        );
    }

    #[test]
    fn namespace_teardown_skips_a_command_a_callback_already_deleted() {
        // The snapshot entry for `one` is gone by the time the loop reaches
        // it, and Tcl_DeleteCommandFromToken returns early rather than firing
        // a second time.
        assert_eq!(
            teardown_log(&format!(
                "{}
                 proc killone {{old new op}} {{rename ::N::one {{}}; lappend ::log killed-one}}
                 trace add command ::N::six delete killone
                 namespace delete ::N",
                traced_procs("one two three four five six seven eight nine ten")
            )),
            "one killed-one six four three eight seven nine five two ten"
        );
    }

    #[test]
    fn namespace_teardown_places_imports_at_their_hash_positions() {
        // An imported redirect occupies an ordinary cmdTable entry, so it is
        // retired at its own hash position rather than as a separate bulk pass
        // over the namespace's imports.
        let expected = "six four three s1 eight seven s2 five two one";
        assert_eq!(
            teardown_log(
                "namespace eval S {proc s1 {} {}; proc s2 {} {}; namespace export s*}
                 namespace eval N {namespace import ::S::s1 ::S::s2}
                 foreach n {one two three four five six seven eight} {proc ::N::$n {} {}}
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
            ),
            expected
        );
        // `namespace forget` deletes the entry and the re-import creates a new
        // one at the same bucket head, so the order is unchanged.
        assert_eq!(
            teardown_log(
                "namespace eval S {proc s1 {} {}; proc s2 {} {}; namespace export s*}
                 namespace eval N {
                     namespace import ::S::s1 ::S::s2
                     namespace forget ::S::s1
                     namespace import ::S::s1
                 }
                 foreach n {one two three four five six seven eight} {proc ::N::$n {} {}}
                 foreach n [info commands ::N::*] {trace add command $n delete rec}
                 namespace delete ::N"
            ),
            expected
        );
    }

    #[test]
    fn namespace_teardown_retires_each_sources_import_tree_before_the_next_entry() {
        // Tcl_DeleteCommandFromToken walks the deleted command's ImportRef
        // list depth-first straight after its own delete trace, so a source's
        // whole import tree fires before the next cmdTable entry.
        let mut log = String::new();
        leak_free(|i| {
            let script = b"set log {}
                 namespace eval N {proc one {} {}; proc two {} {}; proc six {} {}
                                   namespace export *}
                 namespace eval I {namespace import ::N::one; namespace export *}
                 namespace eval J {namespace import ::I::one}
                 foreach c {::N::one ::N::two ::N::six ::I::one ::J::one} {
                     trace add command $c delete [list apply {{c old new op} {
                         lappend ::log $c
                     }} $c]
                 }
                 namespace delete ::N
                 set ::log";
            assert_eq!(
                i.eval_str(script),
                Code::Ok,
                "{}",
                String::from_utf8_lossy(&i.result_bytes())
            );
            log = String::from_utf8_lossy(&i.result_bytes()).into_owned();
        });
        assert_eq!(log, "::N::six ::N::two ::N::one ::I::one ::J::one");
    }

    #[test]
    fn a_refused_alias_rename_keeps_the_command_tables_growth() {
        // `TclRenameCommand` creates the destination hash entry *before*
        // `TclPreventAliasLoop` and deletes it again on a refusal, so the
        // transient twelfth entry rebuilds the eleven-command table from 4
        // buckets to 16 and the grown array outlives the rejected rename.
        let eleven = "namespace eval N {}
             foreach n {one two three four five six seven eight nine ten eleven} {
                 proc ::N::$n {} {}
             }";
        let trace_and_delete = "
             foreach n [info commands ::N::*] {trace add command $n delete rec}
             namespace delete ::N";
        assert_eq!(
            teardown_log(&format!("{eleven}{trace_and_delete}")),
            "six four three eight seven nine five two one eleven ten"
        );
        let mut refusal = String::new();
        let mut alias_survives = String::new();
        leak_free(|i| {
            let recorder = b"set log {}
                 proc rec {old new op} {lappend ::log [namespace tail $old]}";
            assert_eq!(i.eval_str(recorder), Code::Ok);
            assert_eq!(i.eval_str(eleven.as_bytes()), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} ::a {} ::N::b"), Code::Ok);
            assert_eq!(i.eval_str(b"rename ::a ::N::b"), Code::Error);
            refusal = String::from_utf8_lossy(&i.result_bytes()).into_owned();
            assert_eq!(i.eval_str(b"llength [info commands ::a]"), Code::Ok);
            alias_survives = String::from_utf8_lossy(&i.result_bytes()).into_owned();
            assert_eq!(i.eval_str(trace_and_delete.as_bytes()), Code::Ok);
            assert_eq!(i.eval_str(b"set ::log"), Code::Ok);
            assert_eq!(
                String::from_utf8_lossy(&i.result_bytes()),
                "three seven one two four eleven eight five nine six ten"
            );
            assert_eq!(i.eval_str(b"unset ::log"), Code::Ok);
        });
        assert_eq!(
            refusal,
            "cannot define or rename alias \"b\": would create a loop"
        );
        assert_eq!(alias_survives, "1");
    }

    #[test]
    fn namespace_delete_removes_subtree() {
        leak_free(|i| {
            i.eval_str(b"namespace eval foo { proc p {} {return P} ; namespace eval bar {} }");
            assert_eq!(i.eval_str(b"namespace exists ::foo::bar"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"namespace delete ::foo"), Code::Ok);
            // The namespace, its child, and its commands are gone.
            assert_eq!(i.eval_str(b"namespace exists ::foo"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"namespace exists ::foo::bar"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"::foo::p"), Code::Error);
            // Deleting a missing namespace errors (tclsh message).
            assert_eq!(i.eval_str(b"namespace delete ::nope"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown namespace \"::nope\" in namespace delete command"
            );
        });
    }

    /// Run `sheet` and pin its result byte for byte. Every expectation below
    /// was measured on tclsh 9.0.4 and is identical on 8.6.16.
    fn pins(sheet: &[u8], want: &[u8]) {
        leak_free(|i| {
            let code = i.eval_str(sheet);
            let got = String::from_utf8_lossy(&i.result_bytes()).into_owned();
            assert_eq!(code, Code::Ok, "sheet failed: {got}");
            assert_eq!(
                got.as_bytes(),
                want,
                "sheet:\n{}",
                String::from_utf8_lossy(sheet)
            );
        });
    }

    // A deleted namespace is retained for its live frames.
    //
    // C's `Tcl_DeleteNamespace` takes the `activationCount > (nsPtr ==
    // globalNsPtr)` branch: `NS_DYING`, unlink the parent edge, and leave the
    // contents alone. `Tcl_PopCallFrame` calls the deletion again once the last
    // frame holding the token goes away.

    /// `namespace delete ::N` from inside `::N::p` unpublishes the name
    /// immediately, yet the relative `q` still resolves through the frame's
    /// own token.
    #[test]
    fn deleting_the_running_namespace_retains_it_for_the_frame() {
        pins(
            br#"set answer [namespace eval N {
                    proc q {} {return Q}
                    proc p {} {
                        namespace delete ::N
                        set code [catch {q} message]
                        list [namespace exists ::N] [info commands ::N::p] \
                             [info commands ::N::q] $code $message [namespace current]
                    }
                    p
                }]
                list $answer [namespace exists ::N] [info commands ::N::*]"#,
            b"{0 {} {} 0 Q ::N} 0 {}",
        );
    }

    /// A `namespace eval` frame is an activation too (C counts every
    /// `Tcl_PushCallFrame`), so deleting from the body defers just the same.
    #[test]
    fn a_namespace_eval_frame_defers_its_own_deletion() {
        pins(
            br#"set r [namespace eval N {
                    proc q {} {return Q}
                    namespace delete ::N
                    list [namespace exists ::N] [catch {q} m] $m [namespace current] \
                         [info commands ::N::*]
                }]
                list $r [namespace exists ::N]"#,
            b"{0 0 Q ::N {}} 0",
        );
    }

    /// `namespace inscope` and `apply -ns` push the same kind of frame.
    #[test]
    fn inscope_and_apply_frames_defer_their_own_deletion() {
        pins(
            br#"namespace eval N {proc q {} {return Q}}
                set a [namespace inscope N {
                    namespace delete ::N
                    list [q] [namespace current] [namespace exists ::N]
                }]
                list $a [namespace exists ::N]"#,
            b"{Q ::N 0} 0",
        );
        pins(
            br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {proc q {} {return Q}}
                trace add command ::N::q delete rec
                set a [apply [list {} {namespace delete ::N; list [q] [llength $::log]} ::N]]
                list $a $log"#,
            b"{Q 0} {{::N::q delete}}",
        );
    }

    /// The name is free the instant the token is retained: `namespace eval ::N`
    /// builds a wholly separate token, and neither absorbs the other. C nulls
    /// `parentPtr` in the deferred branch, so the child entry is gone before a
    /// recreation looks for it (unlike the synchronous window, which errors
    /// `already exists`).
    #[test]
    fn a_recreated_namespace_is_a_distinct_token_from_the_retained_one() {
        pins(
            br#"namespace eval N {
                    proc q {} {return Q}
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {proc r {} {return R}}
                        list [q] [catch {r} m] $m [info commands ::N::*] [::N::r] \
                             [namespace current] [namespace exists ::N] [namespace which q] \
                             [namespace which r] [info commands q]
                    }
                }
                set a [::N::p]
                list $a [info commands ::N::*] [namespace exists ::N]"#,
            br#"{Q 1 {invalid command name "r"} ::N::r R ::N 1 ::N::q {} q} ::N::r 1"#,
        );
    }

    /// #1764: the object command retained by an active namespace frame keeps
    /// its original OO identity when a new namespace generation publishes a
    /// same-named object. Absolute lookup reaches the new command while the
    /// relative retained command continues to dispatch to the old object.
    #[test]
    fn recreated_namespace_keeps_old_and_new_object_identities_distinct() {
        pins(
            br#"set r [namespace eval N {
                    oo::class create C {method m {} {return OLD}}
                    C create o
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {
                            oo::class create C {method m {} {return NEW}}
                            C create o
                        }
                        set a [catch {o m} am]
                        set b [catch {::N::o m} bm]
                        list $a $am $b $bm [info commands o] [info commands ::N::o]
                    }
                    p
                }]
                set c [catch {::N::o m} cm]
                list $r $c $cm"#,
            b"{0 OLD 0 NEW o ::N::o} 0 NEW",
        );
    }

    #[test]
    fn retained_object_teardown_preserves_recreated_method_dispatch() {
        // Native proof: naming.namespace.original-command-holder-routing
        // docs/design/analysis/name-resolution-proofs/namespace-original-command-holder-routing.md
        // The original public control reports no private dispatcher command,
        // while the recreated object's method still invokes my successfully.
        pins(
            br#"set r [namespace eval N {
                    oo::class create C {method m {} {return OLD}}
                    C create o
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {
                            oo::class create C {
                                method m {} {return NEW}
                                method n {} {my m}
                            }
                            C create o
                        }
                        o m
                    }
                    p
                }]
                list $r [info commands ::N::o::my] [::N::o n]"#,
            b"OLD {} NEW",
        );
    }

    /// A command trace belongs to a token, not to a spelling: the retained
    /// `::N::q` and the recreated `::N::q` each fire their own, in their own
    /// teardown (C walks `cmdPtr->tracePtr`).
    #[test]
    fn a_retained_token_and_its_recreation_fire_separate_command_traces() {
        pins(
            br#"set log {}
                proc rec {old new op} {lappend ::log [list old $old $op]}
                proc rec2 {old new op} {lappend ::log [list new $old $op]}
                namespace eval N {
                    proc q {} {return Q}
                    proc p {} {
                        namespace delete ::N
                        namespace eval ::N {proc q {} {return R}}
                        trace add command ::N::q delete rec2
                        set ::mid $::log
                        return [q]
                    }
                }
                trace add command ::N::q delete rec
                set r [::N::p]
                set after $::log
                namespace delete ::N
                list $r $mid $after $::log"#,
            b"Q {} {{old ::N::q delete}} {{old ::N::q delete} {new ::N::q delete}}",
        );
    }

    /// A qualified definition cannot reach the retained token — the public name
    /// is gone — but an unqualified one lands in it through the frame.
    #[test]
    fn a_retained_token_takes_relative_definitions_only() {
        pins(
            br#"namespace eval N {
                    proc p {} {
                        namespace delete ::N
                        proc r {} {return R}
                        list [r] [info commands ::N::r] [namespace which r] \
                             [catch {proc ::N::s {} {}} m] $m [info commands ::N::*] \
                             [namespace exists ::N]
                    }
                }
                set a [::N::p]
                list $a [info commands ::N::*] [namespace exists ::N]"#,
            br#"{R {} ::N::r 1 {can't create procedure "::N::s": unknown namespace} {} 0} {} 0"#,
        );
    }

    /// Variables ride with the token: `variable v` still reads it, while every
    /// name-addressed probe fails.
    #[test]
    fn a_retained_token_keeps_its_variables() {
        pins(
            br#"namespace eval N {
                    variable v V
                    proc p {} {
                        namespace delete ::N
                        variable v
                        list [catch {set ::N::v} m] $m [set v] [info vars ::N::*] \
                             [info vars [namespace current]::*]
                    }
                }
                ::N::p"#,
            br#"1 {can't read "::N::v": no such variable} V {} {}"#,
        );
    }

    /// The current-namespace probes the issue names, from inside the frame.
    #[test]
    fn a_retained_token_reports_its_own_name_and_commands() {
        pins(
            br#"namespace eval N {
                    variable v 1
                    proc p {} {
                        namespace delete ::N
                        list [namespace exists [namespace current]] \
                             [info commands [namespace current]::*] [info procs] \
                             [namespace which -variable v]
                    }
                }
                ::N::p"#,
            b"0 {} p ::N::v",
        );
    }

    /// A child of the retained token stays reachable relatively, and reachable
    /// by no absolute name at all. The absolute name is built at run time on
    /// purpose: a literal would measure tclsh's `ResolvedCmdName` bytecode
    /// cache, which only invalidates on `NS_DYING` of the command's *own*
    /// namespace.
    #[test]
    fn a_retained_token_keeps_its_children() {
        pins(
            br#"namespace eval N {
                    namespace eval C {proc x {} {return X}}
                    proc p {} {
                        namespace delete ::N
                        set n [join [list {} N C x] ::]
                        list [C::x] [namespace exists ::N::C] [catch {$n} m] $m \
                             [namespace children] [catch {namespace children ::N} m2] $m2
                    }
                }
                set a [::N::p]
                list $a [namespace exists ::N::C]"#,
            br#"{X 0 1 {invalid command name "::N::C::x"} ::N::C 1 {namespace "::N" not found}} 0"#,
        );
    }

    /// Deletion recurses through the same count-checking entry C's
    /// `TclDeleteNamespaceChildren` uses: the parent has no frame and tears down
    /// at once, the child holding the frame is retained until it pops.
    #[test]
    fn an_active_child_defers_while_its_parent_tears_down() {
        pins(
            br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {
                    proc np {} {return NP}
                    namespace eval C {
                        proc cq {} {return CQ}
                        proc p {} {
                            namespace delete ::N
                            set abs [join [list {} N C cq] ::]
                            list [namespace exists ::N] [namespace exists ::N::C] \
                                 [namespace current] [cq] [catch {$abs} m] $m [llength $::log]
                        }
                    }
                }
                trace add command ::N::np delete rec
                trace add command ::N::C::cq delete rec
                set a [::N::C::p]
                list $a $log"#,
            br#"{0 0 ::N::C CQ 1 {invalid command name "::N::C::cq"} 1} {{::N::np delete} {::N::C::cq delete}}"#,
        );
    }

    /// TclOO destruction follows the same per-token activation check as the
    /// namespace command table. Deleting an inactive parent must not tear down
    /// objects owned by its active child generation.
    #[test]
    fn active_child_retains_its_oo_state_during_parent_deletion() {
        pins(
            br#"namespace eval P::N {
                    oo::class create C {method m {} {return OLD}}
                    C create o
                    proc p {} {
                        namespace delete ::P
                        set a [list [namespace current] [namespace exists ::P] \
                                    [catch {o m} om] $om]
                        namespace eval ::P::N {
                            oo::class create C {method m {} {return NEW}}
                            C create o
                        }
                        set b [list [catch {o m} om] $om \
                                    [catch {::P::N::o m} nm] $nm]
                        list $a $b
                    }
                }
                set r [::P::N::p]
                list $r [::P::N::o m]"#,
            b"{{::P::N 0 0 OLD} {0 OLD 0 NEW}} NEW",
        );
    }

    /// An OO command and its default instance namespace created by a delete
    /// trace join the exact dying namespace token. The fixed-point sweep then
    /// retires them instead of publishing a fresh visible namespace tree.
    #[test]
    fn oo_created_by_delete_trace_joins_dying_generation() {
        pins(
            br#"set log {}
                proc mk {old new op} {
                    lappend ::log make
                    oo::object create ::N::late
                    lappend ::log made
                }
                namespace eval N {proc p {} {}}
                trace add command ::N::p delete mk
                namespace delete ::N
                list $log [namespace exists ::N] [info commands ::N::*] \
                     [info object isa object ::N::late] \
                     [catch {::N::late destroy} m] $m"#,
            br#"{make made} 0 {} 0 1 {invalid command name "::N::late"}"#,
        );
    }

    /// Nothing in the retained token fires until the last frame pops — and a
    /// second `namespace eval N` around the call is a second activation, so
    /// even returning from the proc is not enough.
    #[test]
    fn command_delete_traces_wait_for_the_last_activation() {
        pins(
            br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {
                    proc q {} {return Q}
                    proc p {} {namespace delete ::N; set ::mid $::log; return [q]}
                }
                trace add command ::N::p delete rec
                trace add command ::N::q delete rec
                set r [namespace eval N {set inner [p]; set ::after_p $::log; set inner}]
                list $r $mid $after_p $log"#,
            b"Q {} {} {{::N::p delete} {::N::q delete}}",
        );
        pins(
            br#"set log {}
                namespace eval N {
                    variable v 1
                    proc p {} {namespace delete ::N; set ::mid $::log; return ok}
                }
                trace add variable ::N::v unset {apply {{a b c} {lappend ::log unset-v}}}
                set r [::N::p]
                list $r $mid $log"#,
            b"ok {} unset-v",
        );
    }

    /// The deferring frame need not be the one running `namespace delete`, and
    /// the retained token refuses a second deletion (its name is already gone).
    #[test]
    fn a_deferred_namespace_is_unpublished_for_every_caller() {
        pins(
            br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {
                    proc q {} {return Q}
                    proc p2 {} {namespace delete ::N}
                    proc p {} {p2; list [q] [llength $::log]}
                }
                trace add command ::N::q delete rec
                set a [::N::p]
                list $a [llength $log]"#,
            b"{Q 0} 1",
        );
        pins(
            br#"namespace eval N {
                    proc p {} {namespace delete ::N; list [catch {namespace delete ::N} m] $m}
                }
                ::N::p"#,
            br#"1 {unknown namespace "::N" in namespace delete command}"#,
        );
    }

    /// An ensemble the namespace owns dies at once even when the namespace is
    /// retained: C pops `nsPtr->ensembles` before it looks at the count.
    #[test]
    fn an_owned_ensemble_dies_while_its_namespace_is_retained() {
        pins(
            br#"set log {}
                proc rec {old new op} {lappend ::log [list $old $op]}
                namespace eval N {
                    namespace ensemble create -command ::E
                    proc p {} {namespace delete ::N; list [info commands ::E] $::log}
                }
                trace add command ::E delete rec
                ::N::p"#,
            b"{} {{::E delete}}",
        );
    }

    /// The deferred teardown is the ordinary one, run later: the command table
    /// still empties in `Tcl_FirstHashEntry` order, imports included.
    #[test]
    fn the_deferred_teardown_keeps_tcl_hash_order() {
        pins(
            br#"set log {}
                proc rec2 {old new op} {lappend ::log [namespace tail $old]}
                namespace eval N {}
                foreach n {one two three four five six seven eight nine ten} {
                    proc ::N::$n {} {}
                }
                proc ::N::p {} {namespace delete ::N; return [llength $::log]}
                foreach n [info commands ::N::*] {trace add command $n delete rec2}
                set r [::N::p]
                list $r $log"#,
            b"0 {p six four three eight seven nine five two one ten}",
        );
        pins(
            br#"set log {}
                proc rec2 {old new op} {lappend ::log [namespace tail $old]}
                namespace eval S {proc s1 {} {}; namespace export s*}
                namespace eval N {
                    namespace import ::S::s1
                    proc p {} {namespace delete ::N; return}
                }
                foreach n {one two three four five six seven eight nine ten} {
                    proc ::N::$n {} {}
                }
                foreach n [info commands ::N::*] {trace add command $n delete rec2}
                ::N::p
                set log"#,
            b"p three seven one two four eight five nine s1 six ten",
        );
    }

    #[test]
    fn build_info_queries() {
        // `tcl::build-info` (the tcltest constraint source): version/patchlevel
        // parse the build string; feature flags we don't set report 0.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"tcl::build-info version"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9.0");
            assert_eq!(i.eval_str(b"tcl::build-info patchlevel"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9.0.4");
            for feat in [&b"debug"[..], b"purify", b"memdebug", b"no-deprecate"] {
                let mut cmd = b"tcl::build-info ".to_vec();
                cmd.extend_from_slice(feat);
                assert_eq!(i.eval_str(&cmd), Code::Ok);
                assert_eq!(i.result_bytes(), b"0", "feature {feat:?} should be absent");
            }
        });
    }

    #[test]
    fn import_only_takes_exported_commands() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} ::lib::secret {} set s");
            i.eval_str(b"interp alias {} ::lib::pub {} set p");
            i.eval_str(b"namespace eval lib { namespace export pub }");
            i.eval_str(b"namespace eval app { namespace import ::lib::* }");
            // `pub` imported, `secret` not.
            assert_eq!(i.eval_str(b"namespace eval app { pub 1 }"), Code::Ok);
            assert_eq!(i.eval_str(b"namespace eval app { secret 1 }"), Code::Error);
        });
    }

    /// C imports hold the source's command *token*: renaming the source
    /// keeps the import working and `namespace origin` reports the NEW name;
    /// deleting the source leaves the import dangling. Pinned on tclsh
    /// 8.6.16 / 9.0.4 (`rename ::src::e ::src::e2` → `::dst::e` still runs,
    /// origin `::src::e2`; `rename ::src2::f ""` → `::dst2::f` errors).
    #[test]
    fn import_follows_source_rename_but_not_delete() {
        leak_free(|i| {
            i.eval_str(b"namespace eval src { namespace export e; proc e {} { return E } }");
            i.eval_str(b"namespace eval dst { namespace import ::src::e }");
            assert_eq!(i.eval_str(b"rename ::src::e ::src::e2"), Code::Ok);
            assert_eq!(i.eval_str(b"::dst::e"), Code::Ok);
            assert_eq!(i.result_bytes(), b"E");
            assert_eq!(i.eval_str(b"namespace origin ::dst::e"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::src::e2");
            // Delete dangles (lazy miss), as in C.
            assert_eq!(i.eval_str(b"rename ::src::e2 {}"), Code::Ok);
            assert_eq!(i.eval_str(b"::dst::e"), Code::Error);
        });
    }

    #[test]
    fn export_query_returns_patterns() {
        leak_free(|i| {
            i.eval_str(b"namespace eval lib { namespace export a* b* }");
            assert_eq!(
                i.eval_str(b"namespace eval lib { namespace export }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"a* b*");
        });
    }

    #[test]
    fn path_query_returns_list() {
        leak_free(|i| {
            i.eval_str(b"namespace eval lib {}");
            i.eval_str(b"namespace eval app { namespace path ::lib }");
            assert_eq!(
                i.eval_str(b"namespace eval app { namespace path }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::lib");
        });
    }

    // ensembles (targets are aliases, since procs aren't available yet)

    #[test]
    fn ensemble_default_dispatches_to_namespace_commands() {
        leak_free(|i| {
            // ::foo::{bar,baz} as alias→set; export + ensemble over ::foo.
            i.eval_str(b"interp alias {} ::foo::bar {} set");
            i.eval_str(b"interp alias {} ::foo::baz {} set");
            assert_eq!(
                i.eval_str(
                    b"namespace eval foo { namespace export bar baz; namespace ensemble create }"
                ),
                Code::Ok
            );
            // `foo bar v 42` → ::foo::bar v 42 → set v 42.
            assert_eq!(i.eval_str(b"foo bar v 42"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"set v"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            i.eval_str(b"unset v");
        });
    }

    #[test]
    fn ensemble_prefix_match_and_ambiguity() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} ::foo::bar {} set");
            i.eval_str(b"interp alias {} ::foo::baz {} set");
            i.eval_str(
                b"namespace eval foo { namespace export bar baz; namespace ensemble create }",
            );
            // `ba`/`b` are ambiguous between bar and baz.
            assert_eq!(i.eval_str(b"foo ba v 1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown or ambiguous subcommand \"ba\": must be bar, or baz"
            );
            assert_eq!(i.eval_str(b"foo nope v 1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown or ambiguous subcommand \"nope\": must be bar, or baz"
            );
            i.eval_str(b"unset v"); // bar/baz never ran; v is unset → ignore result
        });
    }

    #[test]
    fn ensemble_map_and_subcommands() {
        leak_free(|i| {
            // -map a subcommand to a concrete (builtin) target.
            assert_eq!(
                i.eval_str(b"namespace eval m { namespace ensemble create -map {go ::set} }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"m go v 7"), Code::Ok); // → ::set v 7
            assert_eq!(i.result_bytes(), b"7");
            // a non-mapped subcommand is unknown (the map keys are the set).
            assert_eq!(i.eval_str(b"m set v 7"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown or ambiguous subcommand \"set\": must be go"
            );
            i.eval_str(b"unset v");
        });
    }

    /// Vectors measured against tclsh 9.0.4: dict validation, callback
    /// prefix redispatch/reparse, and the user-facing default target name.
    #[test]
    fn ensemble_issue_1583_oracle_vectors() {
        leak_free(|i| {
            for (script, message) in [
                (
                    &b"namespace ensemble create -map {go}"[..],
                    &b"missing value to go with key"[..],
                ),
                (
                    b"namespace ensemble create -map {go {}}",
                    b"ensemble subcommand implementations must be non-empty lists",
                ),
                (
                    b"set badmap \"go \\{\"; namespace ensemble create -map $badmap",
                    b"unmatched open brace in dict",
                ),
            ] {
                assert_eq!(i.eval_str(script), Code::Error);
                assert_eq!(i.result_bytes(), message);
            }

            assert_eq!(
                i.eval_str(
                    b"proc uh {ens args} {return [list list REPLACED $ens]}
                      namespace eval se5 {
                          namespace ensemble create -command ::se5 -subcommands {} -unknown ::uh
                      }
                      ::se5 nope 1 2"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"REPLACED ::se5 1 2");

            assert_eq!(
                i.eval_str(
                    b"proc define {ens args} {
                          namespace eval se6 {
                              proc nope args {return DEFINED}; namespace export nope
                          }
                          return {}
                      }
                      namespace eval se6 {
                          namespace ensemble create -command ::se6 -unknown ::define
                      }
                      ::se6 nope"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"DEFINED");

            assert_eq!(
                i.eval_str(
                    b"proc target args {return TARGET}
                      proc repair {ens args} {
                          namespace ensemble configure $ens -map {nope ::target}; return {}
                      }
                      namespace eval se7 {
                          namespace ensemble create -command ::se7 -unknown ::repair
                      }
                      ::se7 nope"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"TARGET");

            assert_eq!(
                i.eval_str(
                    b"namespace eval k1 {namespace ensemble create -subcommands ghost}
                      ::k1 ghost"
                ),
                Code::Error
            );
            assert_eq!(i.result_bytes(), b"invalid command name \"ghost\"");
        });
    }

    #[test]
    fn ensemble_unknown_retains_the_live_command_token() {
        // Same-name create is replacement, not mutation: the callback's old
        // active token is dead even though a new ensemble occupies `::E`.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc new args {return NEW}
                      proc rebuild {ens args} {
                          namespace eval ::N {
                              namespace ensemble create -command ::E -map {x ::new}
                          }
                          return {}
                      }
                      namespace eval ::N {
                          namespace ensemble create -command ::E -unknown ::rebuild
                      }
                      set c [catch {::E nope} m o]
                      list $c $m [dict get $o -errorcode] [::E x]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} NEW"
            );
        });

        // Replacement fires and drops the old command's delete trace.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc deleted args {lappend ::seen $args}
                      namespace ensemble create -command ::E
                      trace add command ::E delete deleted
                      set replacement [namespace ensemble create -command ::E]
                      list $seen $replacement"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{::E {} delete}} ::E");
        });

        // The delete trace may move the captured old token. Replacement must
        // retire that identity at its new location, then install the fresh
        // ensemble at the original binding.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc move_old {old new op} {rename $old ::OLD}
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete move_old
                      namespace eval N {namespace ensemble create -command ::E}
                      list [info commands ::E] [info commands ::OLD] \
                           [namespace ensemble exists ::E] \
                           [namespace ensemble exists ::OLD]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::E {} 1 0");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc target args {return TARGET}
                      namespace eval ER {
                          proc repair {ens args} {
                              namespace ensemble configure $ens -map {nope ::target}
                              rename $ens ::ER2
                              return {}
                          }
                          namespace ensemble create -command ::ER -unknown ::ER::repair
                      }
                      list [::ER nope] [namespace ensemble configure ::ER2 -map] [::ER2 nope]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"TARGET {nope ::target} TARGET");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc target args {return TARGET}
                      proc repair {ens args} {
                          set ::seen $ens
                          namespace ensemble configure $ens -map {nope ::target}
                          return {}
                      }
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command E -unknown ::repair
                      }
                      namespace eval I {namespace import ::S::E}
                      list [::I::E nope] $seen [namespace origin ::I::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"TARGET ::S::E ::S::E");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc target args {return TARGET}
                      proc replace {ens args} {
                          rename $ens {}
                          namespace ensemble create -command $ens -map {nope ::target}
                          return \\{
                      }
                      namespace eval D {
                          namespace ensemble create -command E -unknown ::replace
                      }
                      set c [catch {::D::E nope} m o]
                      list $c $m [dict get $o -errorcode] [::D::E nope]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} TARGET"
            );
        });

        leak_free(|i| {
            let code = i.eval_str(
                b"proc target args {return TARGET}
                      set seen {}
                      proc hide_repair {ens args} {
                          namespace ensemble configure $ens -map {nope ::target} -unknown ::hidden_repair
                          interp hide {} $ens heldE
                          return {}
                      }
                      proc hidden_repair {ens args} {
                          set ::seen $ens
                          return [list ::target]
                      }
                      namespace ensemble create -command ::EH -unknown ::hide_repair
                      set first [::EH nope]
                      set seen {}
                      set second [interp invokehidden {} heldE other]
                      list $first $second $seen [info commands ::EH] [interp hidden {}]",
            );
            assert_eq!(
                code,
                Code::Ok,
                "{}",
                String::from_utf8_lossy(&i.result_bytes())
            );
            assert_eq!(i.result_bytes(), b"TARGET TARGET ::heldE {} heldE");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc zap {ens args} {namespace delete ::ND; return {}}
                      namespace eval ND {
                          namespace ensemble create -command ::NDE -unknown ::zap
                      }
                      set c [catch {::NDE nope} m o]
                      list $c $m [dict get $o -errorcode] [info commands ::NDE]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} {TCL ENSEMBLE UNKNOWN_DELETED} {}"
            );
        });
    }

    /// Exact Tcl 9.0.4 command-token oracles: an ensemble import follows its
    /// source token through hide/expose, not a replacement installed at the
    /// vacated name; recreating an occupied ensemble retargets the import to a
    /// new token; true deletion removes the import permanently.
    #[test]
    fn imported_ensemble_retains_source_token_identity() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc tgt_old args {return OLD}
                      proc tgt_new args {return NEW}
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command ::S::E -map {x ::tgt_old}
                      }
                      namespace eval I {namespace import ::S::E}
                      namespace eval S {
                          namespace ensemble create -command ::S::E -map {x ::tgt_new}
                      }
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"NEW ::S::E {x ::tgt_new}");

            assert_eq!(
                i.eval_str(
                    b"rename ::S::E {}
                      set before [list [info commands ::I::E] \
                                           [namespace eval I {namespace import}]]
                      namespace eval S {
                          namespace ensemble create -command ::S::E -map {x ::tgt_new}
                      }
                      list {*}$before [info commands ::I::E] \
                           [namespace eval I {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{} {} {} {}");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc tgt_old args {return OLD}
                      namespace eval Source {
                          namespace ensemble create -command ::E -map {x ::tgt_old}
                      }
                      namespace export E
                      namespace eval I {namespace import ::E}
                      interp hide {} E held
                      proc E args {return REPLACEMENT}
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map] \
                           [namespace ensemble exists ::I::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"OLD ::held {x ::tgt_old} 1");

            assert_eq!(
                i.eval_str(
                    b"interp expose {} held E2
                      list [::I::E x] [namespace origin ::I::E] \
                           [namespace ensemble configure ::I::E -map] [::E2 x]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"OLD ::E2 {x ::tgt_old} OLD");
        });

        // Replacing an occupied source binding with a non-ensemble keeps the
        // import attached to the source command token. The retained ensemble
        // token is now dead, so dispatch and origin fall back to the by-name
        // source, including after that replacement is renamed.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc tgt_old args {return OLD}
                      namespace eval S {
                          namespace export E
                          namespace ensemble create -command E -map {x ::tgt_old}
                      }
                      namespace eval I {namespace import ::S::E}
                      proc ::S::E args {return PROC}
                      set first [list [::I::E x] [namespace origin ::I::E] \
                                          [namespace ensemble exists ::I::E] \
                                          [info commands ::I::E]]
                      rename ::S::E ::S::E2
                      list {*}$first [::I::E x] [namespace origin ::I::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"PROC ::S::E 0 ::I::E PROC ::S::E2");
        });

        // The by-name source shadow moves even while the retained token is
        // live. Replacing the renamed ensemble with a proc retires the token;
        // fallback and origin therefore use the renamed source, and keep
        // following it across a subsequent proc rename.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export E
                          namespace ensemble create -command E
                      }
                      namespace eval I {namespace import ::S::E}
                      rename ::S::E ::S::Moved
                      proc ::S::Moved args {return PROC}
                      set first [list [::I::E] [namespace origin ::I::E] \
                                          [namespace ensemble exists ::I::E]]
                      rename ::S::Moved ::S::Final
                      list {*}$first [::I::E] [namespace origin ::I::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"PROC ::S::Moved 0 PROC ::S::Final");
        });
    }

    /// Retained and recreated namespace tokens may expose identical command
    /// FQNs simultaneously. Imports, renames, origins, dispatch, and teardown
    /// remain qualified by the exact source command generation throughout.
    #[test]
    fn retained_and_recreated_import_sources_stay_generation_distinct() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval N {
                          proc x {} {return OLD}
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {
                                  proc x {} {return NEW}
                                  namespace export x
                              }
                              namespace eval ::B {namespace import ::N::x}
                              rename x y
                              list [namespace origin ::A::x] \
                                   [namespace origin ::B::x] \
                                   [::A::x] [::B::x] [y]
                          }
                          namespace export x p
                      }
                      namespace eval A {namespace import ::N::x}
                      set inside [N::p]
                      list $inside [namespace origin ::B::x] [B::x] \
                           [info commands ::A::x]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{::N::y ::N::x OLD NEW OLD} ::N::x NEW {}"
            );
        });
    }

    /// `namespace origin` follows an imported source chain by command token,
    /// even while a recreated namespace exposes a direct command at the same
    /// intermediate FQN.
    #[test]
    fn retained_import_origin_chain_ignores_a_recreated_same_fqn_command() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval Root {
                          proc x {} {return ROOT}
                          namespace export x
                      }
                      namespace eval O {
                          namespace import ::Root::x
                          namespace export x
                      }
                      namespace eval N {
                          namespace import ::O::x
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {proc x {} {return NEW}}
                              list [namespace origin ::A::x] [::A::x] \
                                   [namespace origin ::N::x] [::N::x]
                          }
                          namespace export x p
                      }
                      namespace eval A {namespace import ::N::x}
                      N::p",
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::Root::x ROOT ::N::x NEW");
        });
    }

    /// Same-origin reimport compares source generations, not display FQNs. A
    /// retained old and recreated new `::N::x` are different import origins.
    #[test]
    fn same_fqn_different_generation_reimport_is_not_idempotent() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval N {
                          proc x {} {return OLD}
                          proc p {} {
                              namespace delete ::N
                              namespace eval ::N {
                                  proc x {} {return NEW}
                                  namespace export x
                              }
                              namespace eval ::A {
                                  set c [catch {namespace import ::N::x} m]
                                  list $c $m [namespace origin x] [x]
                              }
                          }
                          namespace export x p
                      }
                      namespace eval A {namespace import ::N::x}
                      N::p",
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {can't import command \"x\": already exists} ::N::x OLD"
            );
        });
    }

    /// Command replacement adopts surviving imports onto the fresh source
    /// generation, so its later rename and true deletion remain exact too.
    #[test]
    fn replacement_import_source_adopts_the_fresh_generation() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          proc p {} {return OLD}
                          namespace export p
                      }
                      namespace eval I {namespace import ::S::p}
                      proc ::S::p {} {return NEW}
                      rename ::S::p ::S::q
                      set before [list [I::p] [namespace origin ::I::p]]
                      rename ::S::q {}
                      list $before [info commands ::I::p]",
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{NEW ::S::q} {}");
        });
    }

    /// True command deletion removes every import of the source command,
    /// including transitive and hidden ordinary-proc aliases. Recreating the
    /// source binding does not resurrect any of them. Namespace teardown uses
    /// the same generic origin-deletion seam.
    #[test]
    fn true_source_deletion_purges_generic_import_origins() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return OLD}
                      }
                      namespace eval I {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval J {namespace import ::I::p}
                      namespace import ::S::p
                      interp hide {} p heldImport
                      set before [list [namespace origin ::I::p] \
                                           [namespace origin ::J::p] [interp hidden {}]]
                      rename ::S::p {}
                      namespace eval S {proc p {} {return NEW}}
                      list {*}$before [info commands ::I::p] \
                           [info commands ::J::p] [interp hidden {}] \
                           [namespace eval I {namespace import}] \
                           [namespace eval J {namespace import}] \
                           [info commands ::p]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"::S::p ::S::p heldImport {} {} {} {} {} {}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return OLD}
                      }
                      namespace eval I {namespace import ::S::p}
                      namespace delete ::S
                      namespace eval S {
                          namespace export p
                          proc p {} {return NEW}
                      }
                      list [info commands ::I::p] \
                           [namespace eval I {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{} {}");
        });
    }

    /// Each import retains its immediate source command. Replacing the
    /// intermediate import changes what downstream aliases invoke and report as
    /// their origin; true deletion of that intermediate command removes them.
    #[test]
    fn transitive_import_retains_intermediate_binding_lifecycle() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::A::p}
                      proc ::A::p {} {return A}
                      set before [list [::B::p] [namespace origin ::B::p]]
                      rename ::A::p {}
                      list {*}$before [info commands ::B::p] \
                           [namespace eval B {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"A ::A::p {} {}");
        });

        // Independent true-delete path: A is still an imported command when it
        // is deleted. B is therefore deleted with A, while the original source
        // command in S remains live.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::A::p}
                      rename ::A::p {}
                      list [::S::p] [info commands ::B::p] \
                           [namespace eval B {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"S {} {}");
        });
    }

    #[test]
    fn namespace_import_rejects_a_cycle_before_mutating_the_graph() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          proc p {} {return S}
                          namespace export p
                      }
                      namespace eval A {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {
                          namespace import ::A::p
                          namespace export p
                      }
                      set no_force_code [catch {
                          namespace eval S {namespace import ::B::p}
                      } no_force_message no_force_options]
                      set no_force [list $no_force_code $no_force_message \
                          [dict get $no_force_options -errorcode] \
                          [namespace origin ::S::p] \
                          [namespace origin ::A::p] \
                          [namespace origin ::B::p]]
                      set force_code [catch {
                          namespace eval S {namespace import -force ::B::p}
                      } force_message force_options]
                      list {*}$no_force $force_code $force_message \
                           [dict get $force_options -errorcode] \
                           [namespace origin ::S::p] \
                           [namespace origin ::A::p] \
                           [namespace origin ::B::p]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {can't import command \"p\": already exists} \
                  {TCL IMPORT OVERWRITE} ::S::p ::S::p ::S::p \
                  1 {import pattern \"::B::p\" would create a loop containing command \"::S::p\"} \
                  {TCL IMPORT LOOP} ::S::p ::S::p ::S::p"
            );
        });
    }

    /// `namespace import -force` is command replacement, so it uses the same
    /// lifecycle as `proc` redefinition: the displaced token's delete trace
    /// runs while the old command remains visible, and none of its command or
    /// execution trace sidecars transfer to the fresh imported token.
    #[test]
    fn forced_import_replacement_runs_command_lifecycle() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc deleted {old new op} {
                          lappend ::seen [list delete $old [info commands $old]]
                      }
                      proc entered {cmd op} {
                          lappend ::seen [list enter $cmd $op]
                      }
                      namespace eval S {
                          proc p {} {return SRC}
                          namespace export p
                      }
                      namespace eval D {proc p {} {return OLD}}
                      trace add command ::D::p delete deleted
                      trace add execution ::D::p enter entered
                      namespace eval D {namespace import -force ::S::p}
                      set traces [list [trace info command ::D::p] \
                                           [trace info execution ::D::p]]
                      set result [::D::p]
                      list $seen $traces $result [namespace origin ::D::p]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{{delete ::D::p ::D::p}} {{} {}} SRC ::S::p"
            );
        });
    }

    /// Imported-command delete traces run before unbinding, and a command
    /// recreated by the trace callback has a new identity that survives the old
    /// import's deletion. Downstream imports then resolve through that live
    /// intermediate replacement.
    #[test]
    fn imported_delete_trace_observes_and_can_replace_the_binding() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval I {
                          namespace import ::S::p
                          namespace export p
                      }
                      namespace eval B {namespace import ::I::p}
                      set seen {}
                      proc cb {old new op} {
                          lappend ::seen [info commands $old]
                          proc $old {} {return REBORN}
                      }
                      trace add command ::I::p delete cb
                      rename ::S::p {}
                      list $seen [::I::p] [::B::p] [namespace origin ::B::p]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::I::p REBORN REBORN ::I::p");
        });

        // Re-importing the same origin from the delete callback creates a fresh
        // imported-command identity. The outer deletion retires only the old
        // import even though both identities carry identical source metadata.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval A {namespace import ::S::p}
                      proc reimport {old new op} {
                          namespace eval ::A {namespace import -force ::S::p}
                      }
                      trace add command ::A::p delete reimport
                      rename ::A::p {}
                      list [info commands ::A::p] [::A::p] \
                           [namespace origin ::A::p]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::A::p S ::S::p");
        });

        // A trace may move the dying import itself. Identity cleanup follows
        // it and silently drops the moved sidecar, so a later unrelated
        // command at that name cannot fire the old trace.
        leak_free(|i| {
            let code = i.eval_str(
                b"namespace eval S {
                          namespace export p
                          proc p {} {return S}
                      }
                      namespace eval I {namespace import ::S::p}
                      set seen {}
                      proc move_import {old new op} {
                          lappend ::seen [list $old $new $op [info commands $old]]
                          rename $old ::I::q
                      }
                      trace add command ::I::p delete move_import
                      rename ::S::p {}
                      set first $seen
                      proc ::I::q {} {return unrelated}
                      rename ::I::q {}
                      list $first $seen [info commands ::I::p] \
                           [info commands ::I::q]",
            );
            assert_eq!(
                code,
                Code::Ok,
                "{}",
                String::from_utf8_lossy(&i.result_bytes())
            );
            assert_eq!(
                i.result_bytes(),
                b"{{::I::p {} delete ::I::p}} {{::I::p {} delete ::I::p}} {} {}"
            );
        });
    }

    /// Tcl 9.0.4 deletes a hidden real ensemble when its configured namespace
    /// dies, including its hidden-table binding and every import that retains
    /// the token. The active unknown callback therefore observes a dead token.
    #[test]
    fn namespace_delete_retires_hidden_ensemble_token() {
        // Namespace deletion fires a visible namespace-owned ensemble's delete
        // trace before marking/detaching the namespace. Tcl's ensemble list is
        // retired first, so the callback still sees both the owning namespace
        // and the captured command token; teardown removes both afterwards.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc observe {old new op} {
                          lappend ::seen [list [namespace exists ::N] $old \
                              [info commands $old] \
                              [namespace ensemble exists $old]]
                      }
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete observe
                      namespace delete ::N
                      list $seen [namespace exists ::N] [info commands ::E]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{1 ::E ::E 1}} 0 {}");
        });

        // Exposing a hidden victim from its delete callback only moves the old
        // identity; teardown follows it and removes both the command and its
        // moved trace sidecar.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc expose_dying {old new op} {
                          lappend ::seen [list $old [info commands $old]]
                          interp expose {} held E2
                      }
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete expose_dying
                      interp hide {} E held
                      namespace delete ::N
                      list $seen [interp hidden {}] [info commands ::E2]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{::held {}}} {} {}");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc zap {ens args} {
                          interp hide {} NDE held
                          namespace delete ::ND
                          return {}
                      }
                      namespace eval ND {
                          namespace ensemble create -command ::NDE -unknown ::zap
                      }
                      namespace export NDE
                      namespace eval I {namespace import ::NDE}
                      set c [catch {::NDE nope} m o]
                      list $c $m [dict get $o -errorcode] [interp hidden {}] \
                           [info commands ::NDE] [info commands ::I::NDE] \
                           [namespace eval I {namespace import}]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler deleted its ensemble} \
                  {TCL ENSEMBLE UNKNOWN_DELETED} {} {} {} {}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc tgt args {return OK}
                      namespace eval HS {
                          namespace export E
                          namespace ensemble create -command E -map {x ::tgt}
                      }
                      namespace import ::HS::E
                      interp hide {} E heldImport
                      namespace delete ::HS
                      set c [catch {interp invokehidden {} heldImport x} m]
                      list [interp hidden {}] $c $m"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{} 1 {invalid hidden command name \"heldImport\"}"
            );
        });

        // Hidden real ensembles and hidden imported commands both carry their
        // command trace sidecars to the hidden live name. Namespace-driven
        // retirement fires each once and drops it before an unrelated command
        // later occupies the old visible name.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc cb {old new op} {lappend ::seen [list $old $new $op]}
                      namespace eval N {namespace ensemble create -command ::E}
                      trace add command ::E delete cb
                      interp hide {} E held
                      namespace delete ::N
                      proc ::E {} {}
                      list $seen [trace info command ::E] [interp hidden {}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{::held {} delete}} {} {}");
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set seen {}
                      proc cb {old new op} {lappend ::seen [list $old $new $op]}
                      namespace eval S {
                          namespace export p
                          proc p {} {return P}
                      }
                      namespace import ::S::p
                      trace add command ::p delete cb
                      interp hide {} p heldImport
                      namespace delete ::S
                      proc ::p {} {}
                      list $seen [trace info command ::p] [interp hidden {}]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"{{::heldImport {} delete}} {} {}");
        });
    }

    /// A nonempty unknown result is spliced using the post-callback parameter
    /// count. Tcl 9.0.4 treats `missing` as the newly-added second parameter and
    /// removes `newsub` as the live subcommand word.
    #[test]
    fn ensemble_unknown_prefix_uses_live_parameter_layout() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc target args {return $args}
                      proc mutate {ens args} {
                          namespace ensemble configure $ens -parameters {p q}
                          return ::target
                      }
                      namespace eval M {
                          namespace ensemble create -command ::M \
                              -parameters p -unknown ::mutate
                      }
                      ::M P missing newsub tail"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"P missing tail");
        });
    }

    /// Exact Tcl 9.0.4 result-code and list-parser diagnostics for an ensemble
    /// `-unknown` callback.
    #[test]
    fn ensemble_unknown_normalizes_bad_results() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc u {mode ens args} {return -code $mode RESULT}
                      namespace ensemble create -command ::E -unknown {::u break}
                      set out {}
                      foreach mode {break continue return 7} {
                          namespace ensemble configure ::E -unknown [list ::u $mode]
                          set c [catch {::E nope} m o]
                          lappend out $c $m [dict get $o -errorcode]
                      }
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {unknown subcommand handler returned bad code: break} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: continue} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: return} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  1 {unknown subcommand handler returned bad code: 7} \
                  {TCL ENSEMBLE UNKNOWN_RESULT}"
            );

            assert_eq!(
                i.eval_str(
                    b"namespace ensemble configure ::E -unknown {::u break}
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [join [lrange [split [dict get $o -errorinfo] \\n] 0 1] \\n]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown subcommand handler returned bad code: break} \
                  {TCL ENSEMBLE UNKNOWN_RESULT} \
                  {unknown subcommand handler returned bad code: break\n    result of ensemble unknown subcommand handler: ::u break ::E nope}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc malformed {ens args} {return \\{}
                      namespace ensemble create -command ::E -unknown ::malformed
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [join [lrange [split [dict get $o -errorinfo] \\n] 0 1] \\n]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unmatched open brace in list} {TCL VALUE LIST BRACE} \
                  {unmatched open brace in list\n    while parsing result of ensemble unknown subcommand handler}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc malformed {ens args} {return \"{a}junk\"}
                      namespace ensemble create -command ::E -unknown ::malformed
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] \
                           [dict get $o -errorinfo]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{list element in braces followed by \"junk\" instead of space} \
                  {TCL VALUE LIST JUNK} \
                  {list element in braces followed by \"junk\" instead of space\n    while parsing result of ensemble unknown subcommand handler\n    invoked from within\n\"::E nope\"}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc baderr args {
                          return -code error -errorcode {CUSTOM CODE} BOOM
                      }
                      namespace ensemble create -command ::E -unknown ::baderr
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] [dict get $o -errorinfo]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"BOOM {CUSTOM CODE} {BOOM\n    while executing\n\"::baderr ::E nope\"\n    (ensemble unknown subcommand handler)\n    invoked from within\n\"::E nope\"}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc delete_unknown {ens args} {rename $ens {}; return {}}
                      namespace ensemble create -command ::E -unknown ::delete_unknown
                      catch {::E nope} m o
                      list $m [dict get $o -errorcode] [dict get $o -errorinfo]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown subcommand handler deleted its ensemble} \
                  {TCL ENSEMBLE UNKNOWN_DELETED} \
                  {unknown subcommand handler deleted its ensemble\n    (ensemble unknown subcommand handler)\n    invoked from within\n\"::E nope\"}"
            );
        });
    }

    #[test]
    fn ensemble_default_miss_preserves_custom_unknown_options() {
        leak_free(|i| {
            let code = i.eval_str(
                b"proc ::unknown {cmd args} {
                          return -code error -errorcode {CUSTOM CODE} \
                              -errorinfo CUSTOMINFO \
                              -errorstack {INNER foo CALL bar} \
                              \"invalid command name \\\"$cmd\\\"\"
                      }
                      namespace eval N {
                          namespace ensemble create -command ::E -subcommands x
                      }
                      set c [catch {::E x} m o]
                      list $c $m [dict get $o -errorcode] \
                           [dict get $o -errorinfo] [dict get $o -errorstack]",
            );
            assert_eq!(
                code,
                Code::Ok,
                "{}",
                String::from_utf8_lossy(&i.result_bytes())
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {invalid command name \"x\"} {CUSTOM CODE} \
                  {CUSTOMINFO\n    invoked from within\n\"::E x\"} \
                  {INNER foo CALL bar}"
            );
        });
    }

    /// Exact Tcl 9.0.4 lookup/read-only error taxonomy for ensemble configure
    /// and namespace origin.
    #[test]
    fn ensemble_configure_and_origin_error_codes() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc plain {} {}
                      namespace ensemble create -command ::E
                      set out {}
                      foreach script {
                          {namespace ensemble configure ::missing}
                          {namespace ensemble configure ::plain}
                          {namespace ensemble configure ::E -namespace ::N}
                          {namespace origin ::missing}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown command \"::missing\"} {TCL LOOKUP COMMAND ::missing} \
                  {\"::plain\" is not an ensemble command} {TCL LOOKUP ENSEMBLE ::plain} \
                  {option -namespace is read-only} {TCL ENSEMBLE READ_ONLY} \
                  {invalid command name \"::missing\"} {TCL LOOKUP COMMAND ::missing}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set out {}
                      foreach script {
                          {namespace origin {not here}}
                          {namespace ensemble configure {not here}}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode] \
                              [llength [dict get $o -errorcode]]
                      }
                      namespace ensemble create -command ::Q \
                          -subcommands {{not here}}
                      catch {::Q {also not here}} m o
                      lappend out $m [dict get $o -errorcode] \
                          [llength [dict get $o -errorcode]]
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{invalid command name \"not here\"} \
                  {TCL LOOKUP COMMAND {not here}} 4 \
                  {unknown command \"not here\"} \
                  {TCL LOOKUP COMMAND {not here}} 4 \
                  {unknown or ambiguous subcommand \"also not here\": must be not here} \
                  {TCL LOOKUP SUBCOMMAND {also not here}} 4"
            );
        });
    }

    #[test]
    fn ensemble_command_option_and_prefixes_off() {
        leak_free(|i| {
            // -command names the ensemble cmd; -prefixes 0 disables prefix match.
            assert_eq!(
                i.eval_str(
                    b"namespace eval q { namespace ensemble create -command ::top -subcommands longname -map {longname ::set} -prefixes 0 }"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"top longname v 9"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9");
            assert_eq!(i.eval_str(b"top long v 9"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"unknown subcommand \"long\": must be longname"
            );
            i.eval_str(b"unset v");
        });
    }

    #[test]
    fn ensemble_exists() {
        leak_free(|i| {
            i.eval_str(b"namespace eval foo { namespace ensemble create }");
            assert_eq!(i.eval_str(b"namespace ensemble exists foo"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"namespace ensemble exists ::nope"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
        });
    }

    // Needs the numeric tower: the linked var is bumped via `expr`.
    #[cfg(have_tommath)]
    #[test]
    fn namespace_upvar_links_local_to_ns_var() {
        leak_free(|i| {
            i.eval_str(b"namespace eval a { variable x 42 }");
            // `namespace upvar a x lx` links a frame-local `lx` to `::a::x`.
            assert_eq!(
                i.eval_str(
                    b"proc p {} { namespace upvar a x lx; set lx [expr {$lx+1}]; return $lx }"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"p"), Code::Ok);
            assert_eq!(i.result_bytes(), b"43");
            // The write went through to the namespace variable.
            assert_eq!(i.eval_str(b"set ::a::x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"43");
            // A missing namespace is an error.
            assert_eq!(i.eval_str(b"namespace upvar nope v lv"), Code::Error);
        });
    }

    #[test]
    fn ensemble_command_is_qualified_relative_to_current_ns() {
        leak_free(|i| {
            // A relative `-command` binds in the current namespace, not global
            // (the `tcl::tm::path` / safe-base case): `-command path` inside
            // `::a::b` creates `::a::b::path`, resolvable by its FQN.
            assert_eq!(
                i.eval_str(
                    b"namespace eval a::b { namespace export path; namespace ensemble create -command path -map {list ::set} }"
                ),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"namespace which -command ::a::b::path"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::a::b::path");
            // No bare `::path` leaked into the global namespace.
            assert_eq!(i.eval_str(b"namespace which -command ::path"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(i.eval_str(b"::a::b::path list v 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            i.eval_str(b"unset v");
        });
    }

    // Braced defining word.
    //
    // Defining a proc via a braced name word (`proc {p} {} {…}`) does not
    // corrupt the command table: the braces are word quoting, the parser
    // strips them, and `define_proc` never sees them.
    //
    // These pin the real behaviour directly, in tower-free command shapes,
    // so the claim stays verifiable in *every* build rather than only where
    // the numeric tower happens to be linked.

    #[test]
    fn braced_defining_word_defines_the_unbraced_name() {
        leak_free(|i| {
            // Real Tcl defines a command literally named `p`: the braces quote
            // the word, they are not part of the name.
            assert_eq!(i.eval_str(b"proc {p} {} { return {::p} }"), Code::Ok);
            assert_eq!(i.eval_str(b"::p"), Code::Ok, "absolute call must dispatch");
            assert_eq!(i.result_bytes(), b"::p");
            assert_eq!(i.eval_str(b"p"), Code::Ok, "bare call must dispatch");
            assert_eq!(i.result_bytes(), b"::p");
            assert_eq!(i.eval_str(b"namespace which -command ::p"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::p");
            // ... and no command named with the braces survives anywhere.
            assert_eq!(i.eval_str(b"namespace which -command {{p}}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    #[test]
    fn braced_defining_word_leaves_builtin_dispatch_intact() {
        leak_free(|i| {
            // The strongest form of the claim: after defining a proc through
            // a braced name word, *unrelated builtins* must still resolve.
            // Only tower-free builtins are exercised so this holds in a
            // libtommath-less build too.
            assert_eq!(i.eval_str(b"proc {p} {} { return {::p} }"), Code::Ok);
            assert_eq!(i.eval_str(b"set __x 1"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"catch {::nosuch} __e"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"llength [list a b c]"), Code::Ok);
            assert_eq!(i.result_bytes(), b"3");
            assert_eq!(i.eval_str(b"string length abcd"), Code::Ok);
            assert_eq!(i.result_bytes(), b"4");
            // The global command table still holds the builtins by name.
            assert_eq!(i.eval_str(b"namespace which -command ::set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
            i.eval_str(b"unset __x");
            i.eval_str(b"unset __e");
        });
    }

    #[test]
    fn braced_defining_word_binds_in_the_current_namespace() {
        leak_free(|i| {
            // The same quoting rule inside a namespace: `proc {q}` in `::ns`
            // binds `::ns::q`, resolvable bare from inside and absolutely from
            // outside, with the global table untouched.
            assert_eq!(
                i.eval_str(b"namespace eval ns { proc {q} {} { return {::ns::q} } }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"::ns::q"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::ns::q");
            assert_eq!(i.eval_str(b"namespace eval ns { q }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::ns::q");
            assert_eq!(i.eval_str(b"namespace which -command ::q"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    // namespace inscope.
    //
    // These avoid `if`/`while`/`for`/`expr` (and anything else `have_tommath`-
    // gated) entirely, so they run identically with or without the bignum
    // tower.

    #[test]
    fn inscope_zero_tail_args_evaluates_script_verbatim() {
        leak_free(|i| {
            i.eval_str(b"proc probe {args} { return $args }");
            // C's `objc == 3` arm: no tail, so no list is appended and no
            // concat/trim/trailing space happens — the script runs as-is.
            assert_eq!(i.eval_str(b"namespace inscope :: probe"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    #[test]
    fn inscope_tail_args_become_list_elements_not_joined_words() {
        // `NamespaceInscopeCmd` (`generic/tclNamesp.c`) collects the tail
        // into a LIST and concatenates its string rep onto `script`
        // (`Tcl_ConcatObj` over `[script, list(tail)]`), so however many
        // words a tail argument holds, it reaches the invoked command as
        // exactly one argument.
        leak_free(|i| {
            i.eval_str(b"proc probe {args} { return $args }");

            // A tail word with an embedded space stays ONE argument (the
            // pre-fix bug: it split into two words, "x" and "y").
            assert_eq!(
                i.eval_str(b"llength [namespace inscope :: probe {x y}]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe {x y}] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"x y");

            // An empty-string tail arg round-trips as one empty argument.
            assert_eq!(
                i.eval_str(b"llength [namespace inscope :: probe {}]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe {}] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"");

            // Multiple tail words each keep their own arg boundary.
            assert_eq!(
                i.eval_str(b"llength [namespace inscope :: probe a {b c} d]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3");

            // Special characters round-trip through the list-element
            // quoting (`list::append_list_element`): an unbalanced brace, a
            // lone backslash, and an embedded double quote. `lindex`
            // recovers the raw element value regardless of which of the
            // four renderings (none/brace/mask/escape) quoting picked.
            i.eval_str(b"set v1 \"a\\{b\"");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe $v1] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"a{b");

            i.eval_str(b"set v2 \"\\\\\"");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe $v2] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"\\");

            i.eval_str(b"set v3 {a\"b}");
            assert_eq!(
                i.eval_str(b"lindex [namespace inscope :: probe $v3] 0"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"a\"b");

            i.eval_str(b"unset v1 v2 v3");
        });
    }

    #[test]
    fn inscope_uses_a_byte_arrays_string_rep_in_both_arms() {
        // `binary format` returns a typed byte array.  When it becomes a
        // script, both `namespace inscope` forms reach the same unknown
        // command.  Capture the error through `binary encode hex`: this is the
        // C Tcl observable, and proves that the byte array's 0x80 payload is
        // preserved rather than becoming U+FFFD or UTF-8's `c2 80` payload.
        // The vector is identical on tclsh 8.6.18 and 9.0.4.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set name [binary format a3c cmd 128]; \
                      catch {namespace inscope :: $name} result; \
                      binary encode hex $result",
                ),
                Code::Ok,
                "zero-tail arm"
            );
            assert_eq!(
                i.result_bytes(),
                b"696e76616c696420636f6d6d616e64206e616d652022636d648022",
                "zero-tail arm preserves the raw command-name byte"
            );

            assert_eq!(
                i.eval_str(
                    b"catch {namespace inscope :: $name extra} result; \
                      binary encode hex $result",
                ),
                Code::Ok,
                "tail arm must agree with the zero-tail arm"
            );
            assert_eq!(
                i.result_bytes(),
                b"696e76616c696420636f6d6d616e64206e616d652022636d648022",
                "tail arm preserves the same raw command-name byte"
            );

            i.eval_str(b"unset name result");
        });
    }

    // Each expectation below is pinned against tclsh 8.6.16 and 9.0.4; the
    // interpreter emulates 9.0 by default, so a release-axis vector says so.

    /// `namespace which -variable` is `Tcl_FindNamespaceVar`: namespace
    /// variable tables only, never a call frame.
    #[test]
    fn which_variable_never_answers_with_a_proc_local() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"proc t {} {set loc 1; namespace which -variable loc}; t"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"");
            // A local shadowing a real namespace variable still reports the
            // namespace one.
            assert_eq!(
                i.eval_str(
                    b"namespace eval ns2 {variable shadow 5\n\
                      proc q {} {set shadow 9; namespace which -variable shadow}}\n\
                      ns2::q"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::ns2::shadow");
        });
    }

    /// `namespace origin` follows `namespace import` links to their source
    /// through the shared `TclGetOriginalCommand` core.
    #[test]
    fn origin_follows_import_chains() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval src {namespace export p; proc p {} {return P}}\n\
                      namespace eval dst {namespace import ::src::p}\n\
                      namespace origin ::dst::p"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::src::p");
            assert_eq!(i.eval_str(b"namespace origin set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::set");
        });
    }

    /// Only `objv[1]` is the `-clear` / `-force` flag: the registry pins
    /// `max_leading_option_words: Some(1)` and C tests that one word.
    #[test]
    fn only_the_first_word_is_the_export_or_import_flag() {
        leak_free(|i| {
            // tclsh 8.6.16 / 9.0.4: `-clear x`.
            assert_eq!(
                i.eval_str(
                    b"namespace eval e {namespace export -clear -clear x}\n\
                      namespace eval e {namespace export}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"-clear x");
            // A trailing `-force` is an ordinary pattern, and an unqualified
            // pattern names no source namespace.
            i.eval_str(b"namespace eval s3 {namespace export q; proc q {} {return Q}}");
            assert_eq!(
                i.eval_str(b"namespace eval t3 {catch {namespace import ::s3::q -force} m; set m}"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"no namespace specified in import pattern \"-force\""
            );
            // …and the import that preceded it still happened.
            assert_eq!(i.eval_str(b"info commands ::t3::*"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::t3::q");
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// `namespace export -clear` empties the list before the patterns that
    /// follow it are added.
    #[test]
    fn export_clear_resets_the_pattern_list() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval e {namespace export a b; namespace export -clear}\n\
                      namespace eval e {namespace export}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"");
            assert_eq!(
                i.eval_str(
                    b"namespace eval f {namespace export a b; namespace export -clear c}\n\
                      namespace eval f {namespace export}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"c");
        });
    }

    /// The ensemble option tables come from the shared owner: `create` has no
    /// `-namespace`, both tables abbreviate, and the `namespace ensemble`
    /// subcommand word abbreviates too.
    #[test]
    fn ensemble_option_tables_match_c() {
        leak_free(|i| {
            i.eval_str(b"namespace eval e5 {namespace export *; proc go {} {return G}}");
            assert_eq!(
                i.eval_str(b"namespace eval e5 {namespace ensemble create -comm ::ab5 -sub go}"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"::ab5");
            assert_eq!(i.eval_str(b"namespace ensemble ex ::ab5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(
                i.eval_str(b"catch {namespace ensemble frobnicate} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"bad subcommand \"frobnicate\": must be configure, create, or exists"
            );
            assert_eq!(
                i.eval_str(b"catch {namespace ensemble create -namespace ::x} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"bad option \"-namespace\": must be -command, -map, -parameters, \
                  -prefixes, -subcommands, or -unknown"
            );
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// C's `if (objc & 1)` fires before any option word is looked at, so an
    /// odd tail is `wrong # args`, never `bad option`.
    #[test]
    fn ensemble_create_checks_pair_arity_first() {
        leak_free(|i| {
            for src in [
                &b"catch {namespace ensemble create -command} m; set m"[..],
                b"catch {namespace ensemble create -bogus} m; set m",
            ] {
                assert_eq!(i.eval_str(src), Code::Ok);
                assert_eq!(
                    i.result_bytes(),
                    b"wrong # args: should be \"namespace ensemble create ?option value ...?\""
                );
            }
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// `namespace ensemble configure` reads through the shared config table:
    /// `-namespace` is readable but never writable, `-command` is not in it,
    /// and abbreviations resolve.
    #[test]
    fn ensemble_configure_uses_the_config_table() {
        leak_free(|i| {
            i.eval_str(
                b"namespace eval e5 {namespace export *; proc go {} {return G}\n\
                  namespace ensemble create -command ::ab5 -subcommands go}",
            );
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::ab5 -sub"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"go");
            assert_eq!(
                i.eval_str(b"catch {namespace ensemble configure ::ab5 -namespace ::e5} m; set m"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"option -namespace is read-only");
            assert_eq!(
                i.eval_str(b"catch {namespace ensemble configure ::ab5 -command ::zz} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"bad option \"-command\": must be -map, -namespace, -parameters, \
                  -prefixes, -subcommands, or -unknown"
            );
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// TclOO's root object commands are engine-installed on the registry's
    /// behalf, so they follow their introducing release; a script-created
    /// object command does not.
    #[test]
    fn tcloo_roots_follow_their_introducing_release() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
            assert_eq!(
                i.eval_str(b"catch {oo::class create C {}} m; set m"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"invalid command name \"oo::class\"");
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"catch {oo::configurable create C {}} m; set m"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"invalid command name \"oo::configurable\""
            );
            // A user object named after a 9.0-only builtin stays callable.
            assert_eq!(
                i.eval_str(b"oo::class create lpop {method m {} {return M}}; [lpop new] m"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"M");
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
            i.eval_str(b"unset -nocomplain m");
        });
    }

    /// Taking a gate-hidden root's name must work through *every* registration
    /// verb, not just `create`. The root marking is an identity on the entry,
    /// so it is cleared in the one registration funnel (`ns_register`); these
    /// two vectors are the funnels that do not go through `create`.
    #[test]
    fn taking_a_hidden_root_name_works_through_copy_and_rename() {
        // `oo::copy` onto the hidden name: the copy must be callable and listed.
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"oo::class create Src {method m {} {return SRC}}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"oo::copy ::Src ::oo::configurable"), Code::Ok);
            assert_eq!(i.eval_str(b"[oo::configurable new] m"), Code::Ok);
            assert_eq!(i.result_bytes(), b"SRC");
            assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::oo::configurable");
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        });
        // `rename` onto the hidden name: same contract.
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(i.eval_str(b"oo::object create ::mysrc"), Code::Ok);
            assert_eq!(i.eval_str(b"rename ::mysrc ::oo::configurable"), Code::Ok);
            assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::oo::configurable");
            assert_eq!(i.eval_str(b"::oo::configurable destroy"), Code::Ok);
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        });
    }

    /// Taking a gate-hidden root's name via `rename` keeps working — the root
    /// marking is cleared on the rename path itself, not only by the OO
    /// re-registration that follows it.
    #[test]
    fn renaming_onto_a_hidden_root_name_keeps_the_replacement_live() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(
                i.eval_str(b"oo::class create ::Src {method m {} {return SRC}}"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"rename ::Src ::oo::configurable"), Code::Ok);
            assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::oo::configurable");
            assert_eq!(i.eval_str(b"[::oo::configurable new] m"), Code::Ok);
            assert_eq!(i.result_bytes(), b"SRC");
            i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        });
    }

    /// Configuring an ensemble through a `namespace import` alias configures
    /// the ORIGIN, so both spellings observe one config and the alias stays an
    /// alias (tclsh 9.0.4-pinned).
    #[test]
    fn configuring_an_imported_ensemble_updates_the_origin() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {namespace export ens\n\
                       proc impl {} {return ORIG}\n\
                       proc impl2 {} {return NEW}\n\
                       namespace ensemble create -command ::S::ens -map {go impl}}\n\
                       namespace eval T {namespace import ::S::ens}"
                ),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(
                    b"namespace eval S {namespace ensemble configure ::T::ens -map {go impl2}}"
                ),
                Code::Ok
            );
            for spelling in [b"::T::ens go".as_slice(), b"::S::ens go"] {
                assert_eq!(i.eval_str(spelling), Code::Ok);
                assert_eq!(i.result_bytes(), b"NEW");
            }
            for spelling in [
                b"namespace ensemble configure ::T::ens -map".as_slice(),
                b"namespace ensemble configure ::S::ens -map",
            ] {
                assert_eq!(i.eval_str(spelling), Code::Ok);
                assert_eq!(i.result_bytes(), b"go ::S::impl2");
            }
            // Still an alias: configuring it did not fork a second ensemble.
            assert_eq!(i.eval_str(b"namespace origin ::T::ens"), Code::Ok);
            assert_eq!(i.result_bytes(), b"::S::ens");
        });
    }

    /// `namespace which -variable` is byte-preserving: a name carrying bytes
    /// that are not valid UTF-8 round-trips, and two such names stay distinct
    /// (a lossy map would collapse both onto U+FFFD).
    #[test]
    fn which_variable_preserves_byte_valued_names() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace eval nb {}"), Code::Ok);
            assert_eq!(
                i.eval_str(b"namespace eval nb [list variable [binary format H* ff41] 7]"),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(
                    b"binary scan [namespace eval nb [list namespace which -variable \
                       [binary format H* ff41]]] H* h; set h"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3a3a6e623a3aff41");
            // Two distinct invalid-UTF-8 names must not collide.
            assert_eq!(
                i.eval_str(
                    b"namespace eval nb [list variable [binary format H* ff] 1]\n\
                       namespace eval nb [list variable [binary format H* fe] 2]\n\
                       namespace eval nb {list [set [binary format H* ff]] \
                       [set [binary format H* fe]]}"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1 2");
        });
    }

    /// The same byte fidelity for the *command* half of the namespace surface:
    /// `namespace origin`, `namespace which -command`, and the TclOO
    /// object-name resolution behind them. These reach their tables through
    /// the shared core's byte-valued entry points, so an invalid-UTF-8 name is
    /// never routed through `str`. tclsh 9.0.4-pinned (`binary encode hex` of
    /// each answer).
    #[test]
    fn origin_and_which_command_preserve_byte_valued_names() {
        leak_free(|i| {
            // An imported command whose simple name is the single byte 0xFF.
            assert_eq!(
                i.eval_str(
                    b"set n [binary format H* ff]\n\
                      namespace eval src [list namespace export $n]\n\
                      namespace eval src [list proc $n {} {return P}]\n\
                      namespace eval dst [list namespace import ::src::$n]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"binary encode hex [namespace origin ::dst::$n]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3a3a7372633a3aff");
            assert_eq!(i.eval_str(b"::dst::$n"), Code::Ok);
            assert_eq!(i.result_bytes(), b"P");
            // Two distinct byte names must stay distinct through `origin` —
            // a lossy map would collapse both onto U+FFFD and collide them.
            assert_eq!(
                i.eval_str(
                    b"set a [binary format H* ff]; set b [binary format H* fe]\n\
                      namespace eval s2 [list namespace export $a $b]\n\
                      namespace eval s2 [list proc $a {} {return A}]\n\
                      namespace eval s2 [list proc $b {} {return B}]\n\
                      list [binary encode hex [namespace origin ::s2::$a]] \
                           [binary encode hex [namespace origin ::s2::$b]]"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3a3a73323a3aff 3a3a73323a3afe");
            // `namespace which -command` over the same name.
            assert_eq!(
                i.eval_str(b"binary encode hex [namespace which -command ::s2::$a]"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"3a3a73323a3aff");
            i.eval_str(b"unset -nocomplain n a b");
        });
    }

    /// The byte-valued `Namespaces` spellings resolve a name the `&str` ones
    /// cannot express at all.
    ///
    /// Everything reachable from a *script* arrives here as valid UTF-8 — a
    /// byte array's string rep is one U+00XX per byte (`bytearray.rs`), so the
    /// old `from_utf8_lossy` hop was a no-op on every scripted path, which is
    /// why the surface test above passes either way. The hole it leaves is the
    /// one `binary_bytes` already documents: an embedder may hand the C ABI a
    /// plain string that is not UTF-8. This exercises that seam directly, so it
    /// fails if the byte-valued entry points are ever routed back through
    /// `str`.
    #[test]
    fn byte_valued_command_names_resolve_without_a_utf8_round_trip() {
        use tcl_runtime_api::Namespaces;

        leak_free(|i| {
            // A command name that is not valid UTF-8 in any encoding.
            let raw: &[u8] = b"::raw\xff\xfename";
            i.ns_register(
                raw,
                crate::interp::Command::Builtin(|interp, _| {
                    interp.set_result_bytes(b"RAW");
                    Code::Ok
                }),
            );
            let global = tcl_runtime_api::NsId(crate::namespace::GLOBAL as u32);
            let id = i
                .find_command_bytes(global, raw)
                .expect("a byte-valued name resolves through the byte-valued spelling");
            assert_eq!(i.command_name_bytes(id).as_deref(), Some(raw));
            // The shared cores reach the same answer.
            assert_eq!(
                tcl_cmd_core::namespace::origin_bytes(i, raw).as_deref(),
                Some(raw)
            );
            assert_eq!(
                tcl_cmd_core::namespace::which_command_bytes(i, raw).as_deref(),
                Some(raw)
            );
            // The lossy `&str` spelling genuinely cannot: the replacement
            // characters name a different (absent) command. This is the
            // divergence the byte-valued entry points exist to remove.
            assert_eq!(
                tcl_cmd_core::namespace::origin(i, &String::from_utf8_lossy(raw)),
                None
            );
        });
    }

    /// An embedder can supply a plain string whose bytes are not UTF-8.
    /// Drive the real `namespace` adapter with such objects rather than using a
    /// script-created byte array (whose string shimmer is valid UTF-8), so any
    /// lossy `&str` hop makes these two distinct namespace names disappear or
    /// collide.
    #[test]
    fn byte_valued_namespace_navigation_uses_the_embedder_bytes_verbatim() {
        fn invoke(interp: &mut Interp, words: &[&[u8]]) -> Code {
            let argv: Vec<*mut crate::obj::TclObj> = words
                .iter()
                .map(|word| crate::obj::new_string_bytes(word))
                .collect();
            let code = super::namespace_cmd(interp, &argv);
            for object in argv {
                super::drop_fresh(object);
            }
            code
        }

        leak_free(|interp| {
            let parent = b"::raw\xff";
            let sibling = b"::raw\xfe";
            let child = b"::raw\xff::child\xfd";
            {
                let mut namespaces = interp.namespaces_mut();
                namespaces.ensure_namespace(crate::namespace::GLOBAL, parent);
                namespaces.ensure_namespace(crate::namespace::GLOBAL, sibling);
                namespaces.ensure_namespace(crate::namespace::GLOBAL, child);
            }

            assert_eq!(invoke(interp, &[b"namespace", b"exists", parent]), Code::Ok);
            assert_eq!(interp.result_bytes(), b"1");
            assert_eq!(
                invoke(interp, &[b"namespace", b"exists", sibling]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"1");

            assert_eq!(invoke(interp, &[b"namespace", b"parent", child]), Code::Ok);
            assert_eq!(interp.result_bytes(), parent);

            assert_eq!(
                invoke(interp, &[b"namespace", b"children", parent]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), child);
            assert_eq!(
                invoke(interp, &[b"namespace", b"children", parent, child]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), child);

            // Native namespace child matching scans opaque CString bytes.
            assert_eq!(
                invoke(interp, &[b"namespace", b"children", parent, b"*"]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), child);
            assert_eq!(
                invoke(interp, &[b"namespace", b"children", parent, b"*other*"]),
                Code::Ok
            );
            assert_eq!(interp.result_bytes(), b"");

            let missing = b"::missing\xff";
            assert_eq!(
                invoke(interp, &[b"namespace", b"parent", missing]),
                Code::Error
            );
            let mut message = b"namespace \"".to_vec();
            message.extend_from_slice(missing);
            message.extend_from_slice(b"\" not found");
            assert_eq!(interp.result_bytes(), message);
        });
    }

    /// `-map` is a dict: insertion order round-trips through the read-back and
    /// a repeated key keeps its first position while taking the last value
    /// (tclsh 9.0.4-pinned).
    #[test]
    fn ensemble_map_preserves_dict_order_and_collapses_repeats() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval M {namespace export *\n\
                       proc zeta {} {return Z}\n\
                       proc alpha {} {return A}\n\
                       proc mid {} {return M}\n\
                       namespace ensemble create -command ::E -map {zz zeta aa alpha}}"
                ),
                Code::Ok
            );
            // Insertion order, not sorted (`aa` would sort first).
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::E -map"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"zz ::M::zeta aa ::M::alpha");
            // A repeated key: last value wins, first position kept.
            assert_eq!(
                i.eval_str(
                    b"namespace eval M {namespace ensemble configure ::E \
                       -map {zz zeta aa alpha zz mid}}"
                ),
                Code::Ok
            );
            assert_eq!(
                i.eval_str(b"namespace ensemble configure ::E -map"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"zz ::M::mid aa ::M::alpha");
            // Dispatch follows the collapsed entry, not the stale first one.
            assert_eq!(i.eval_str(b"::E zz"), Code::Ok);
            assert_eq!(i.result_bytes(), b"M");
        });
    }
}

#[cfg(test)]
mod native_ensemble_tests;

#[cfg(test)]
mod native_upvar_tests;

#[cfg(test)]
mod native_holder_routing;

#[cfg(test)]
mod native_map_prefix_tests;
