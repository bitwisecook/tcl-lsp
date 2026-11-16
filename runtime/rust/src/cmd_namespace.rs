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
//! retaining the actual source generation; `forget` selects those redirects
//! through the shared qualifier, tail and namespace resolver.
//!
//! See `docs/design/runtime/namespace-tree.md` for the model.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::ensemble::EnsembleConfig;
use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{error_code_list, Code, Command, Interp};
use crate::list;
use crate::namespace::{NsId, GLOBAL};
use crate::obj::{self, TclObj};
use tcl_syntax::value::ValueOps;

/// Register the `namespace` command.
pub fn install(interp: &mut Interp) {
    install_core(interp);
    interp.install_stock_jim_namespace_helpers();
}

/// Register primitive namespace commands without loading source libraries.
pub(crate) fn install_core(interp: &mut Interp) {
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
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = namespace_cmd_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn namespace_cmd_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "namespace subcommand issuer",
            )
            .into(),
        );
    };
    let sub = if policy.recipe().is_jim084() {
        match interp.original_jim_namespace_subcommand(argv, arguments) {
            Ok(crate::interp::OriginalJimNamespaceSubcommandOutcome::Invoke(entry)) => {
                entry.name().as_bytes()
            }
            Ok(crate::interp::OriginalJimNamespaceSubcommandOutcome::Complete(code)) => {
                return code
            }
            Err(error) => return interp.report_cmd_error(error),
        }
    } else {
        if argv.len() < 2 {
            return interp.wrong_args_for_invocation(argv, b"subcommand ?arg ...?");
        }
        let original = match arguments.bytes(1) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error),
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
        let Some(subcommand) = spec.resolve_native_namespace_subcommand_bytes_for_dialect(
            policy.recipe(),
            &original,
            dialect,
        ) else {
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
        subcommand.name.as_bytes()
    };
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
        b"parent" => ns_parent(interp, argv, arguments),
        b"children" => ns_children(interp, argv, arguments),
        b"qualifiers" => ns_qualifiers(interp, argv, arguments),
        b"tail" => ns_tail(interp, argv, arguments),
        b"which" => ns_which(interp, argv),
        b"origin" => ns_origin(interp, argv),
        b"export" => {
            if policy.recipe().is_jim084() {
                interp.set_result_bytes(b"");
                Code::Ok
            } else {
                ns_export(interp, argv, arguments)
            }
        }
        b"import" => ns_import(interp, argv, arguments),
        b"forget" => ns_forget(interp, argv, arguments),
        b"path" => ns_path(interp, argv, arguments),
        b"ensemble" => ns_ensemble(interp, argv),
        b"inscope" => ns_inscope(interp, argv),
        b"code" => ns_code(interp, argv),
        b"unknown" => ns_unknown_original(interp, argv, arguments),
        b"upvar" => ns_upvar(interp, argv, arguments),
        _ => unreachable!("subcommand resolved to a canonical name above"),
    }
}

// current / eval / exists / parent / children

/// `namespace unknown ?handler?` — get or set the current namespace's
/// unknown-command handler. The global namespace's default is `::unknown`; a
/// sub-namespace with no handler reports the empty string (and falls back to
/// `::unknown` at dispatch). Mirrors `NamespaceUnknownCmd` (`tclNamesp.c`).
#[cfg(test)]
fn ns_unknown(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = ns_unknown_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn ns_unknown_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"?script?");
    }
    let owner = match interp.original_namespace_consumer(arguments) {
        Ok(owner) => owner,
        Err(error) => return interp.report_cmd_error(error),
    };
    let check = || owner.check(arguments);
    let cur = interp.current_ns();
    if argv.len() == 3 {
        let length = match crate::value_ops::list_length_using(
            interp,
            &argv[2],
            &mut |actual, original| {
                arguments.check_original_for(actual, 2, *original)?;
                check()
            },
            |actual, original| namespace_list_elements_using(actual, *original, &check, arguments),
        ) {
            Ok(length) => length,
            Err(error) => return interp.report_cmd_error_using(error, Some(&check)),
        };
        let handler = (length != 0).then(|| obj::Owned::retain(argv[2]));
        let retired = interp.namespaces_mut().set_unknown_handler(cur, None);
        drop(retired);
        if let Err(error) = check() {
            return interp.report_cmd_error(error);
        }
        let retired = interp.namespaces_mut().set_unknown_handler(cur, handler);
        drop(retired);
        if let Err(error) = check() {
            return interp.report_cmd_error(error);
        }
        interp.set_result(argv[2]);
        return match check() {
            Ok(()) => Code::Ok,
            Err(error) => interp.report_cmd_error_using(error, Some(&check)),
        };
    }
    match interp.namespace_unknown_root(cur, cur == GLOBAL) {
        Some(handler) => interp.set_result(handler.as_ptr()),
        None => interp.set_result_bytes(b""),
    }
    match check() {
        Ok(()) => Code::Ok,
        Err(error) => interp.report_cmd_error_using(error, Some(&check)),
    }
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
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Delete,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
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

/// `namespace eval name arg ?arg ...?` — evaluate a body in `name` (multiple
/// `arg`s are concatenated with spaces, like `eval`).
fn ns_eval(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"name arg ?arg...?");
    }
    interp.with_script_evaluation("namespace eval", argv.len() - 3, |interp| {
        if argv.len() == 4 {
            return interp.ns_eval_objects_with_arguments(
                argv[2],
                argv[3],
                tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                Some(argv),
            );
        }
        let body = match interp.concat_original_arguments(&argv[3..]) {
            Ok(body) => obj::Owned::fresh(body),
            Err(error) => return interp.report_cmd_error(error),
        };
        interp.ns_eval_objects_with_arguments(
            argv[2],
            body.as_ptr(),
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceConcat,
            Some(argv),
        )
    })
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

/// `namespace parent ?name?` selects the C namespace token or Jim textual parent.
/// Jim reads the original operand or the reached current namespace holder.
fn ns_parent(interp: &mut Interp, argv: &[*mut TclObj], arguments: &OriginalArguments) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"?name?");
    }
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        return interp.original_jim_namespace_parent(argv, arguments);
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
fn ns_children(interp: &mut Interp, argv: &[*mut TclObj], arguments: &OriginalArguments) -> Code {
    if argv.len() > 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"?name? ?pattern?");
    }
    let owner = match interp.original_namespace_consumer(arguments) {
        Ok(owner) => owner,
        Err(error) => return interp.report_cmd_error(error),
    };
    let check = || owner.check(arguments);
    let current = interp.current_ns();
    let target = if let Some(&original) = argv.get(2) {
        match interp.native_namespace_object_lookup_at_using(current, original, &check, &|error| {
            arguments.retain_refusal(error.into())
        }) {
            Ok(Some(target)) => target,
            Ok(None) => {
                return ns_operation_not_found_using(
                    interp,
                    original,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::Children,
                    Some(&check),
                );
            }
            Err(error) => return interp.report_cmd_error_using(error, Some(&check)),
        }
    } else {
        interp.current_ns()
    };
    if let Err(error) = owner.observe(target, arguments) {
        return interp.report_cmd_error_using(error, Some(&check));
    }
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
    let pattern = if argv.get(3).is_some() {
        match arguments.bytes_using(3, &check) {
            Ok(bytes) => Some(bytes),
            Err(error) => return interp.report_cmd_error_using(error, Some(&check)),
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
        Err(error) => return interp.report_cmd_error_using(error.into(), Some(&check)),
    };
    if let Err(error) = check() {
        return interp.report_cmd_error_using(error, Some(&check));
    }
    for child in &children {
        if let Err(error) = owner.observe(child.0 as usize, arguments) {
            return interp.report_cmd_error_using(error, Some(&check));
        }
    }
    match tcl_cmd_core::namespace::children_original(interp, &children) {
        Ok(v) => {
            if let Err(error) = check() {
                crate::interp::drop_fresh(v);
                return interp.report_cmd_error_using(error, Some(&check));
            }
            interp.set_result(v);
            match check() {
                Ok(()) => Code::Ok,
                Err(error) => interp.report_cmd_error_using(error, Some(&check)),
            }
        }
        Err(error) => interp.report_cmd_error_using(error.into(), Some(&check)),
    }
}

/// `namespace qualifiers string` — everything before the last `::` (pure text).
fn ns_qualifiers(interp: &mut Interp, argv: &[*mut TclObj], arguments: &OriginalArguments) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"string");
    }
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        return interp.original_jim_namespace_text(argv[2], false, arguments);
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
fn ns_tail(interp: &mut Interp, argv: &[*mut TclObj], arguments: &OriginalArguments) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"string");
    }
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        return interp.original_jim_namespace_text(argv[2], true, arguments);
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
        NativeNamespaceTextResult::SharedEmpty => {
            return interp.refuse_native_access(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "Jim shared empty outside original text owner",
                ),
            );
        }
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
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Which,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
    }
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
    let Some(protocol) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("namespace which protocol")
                .into(),
        );
    };
    let Some((kind, name_index)) =
        tcl_cmd_core::namespace::which_request_in(protocol.recipe(), &args)
    else {
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
        // C reports its selected token; Jim reports its successful fresh query candidate.
        match interp.native_namespace_which_command_bytes(name) {
            Ok(fqn) => interp.set_result_bytes(&fqn.unwrap_or_default()),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    }
    Code::Ok
}

// export / import / forget

/// `namespace export ?-clear? ?pattern ...?` — query / append (or clear+set)
/// the current namespace's export patterns.
fn namespace_pattern(
    interp: &Interp,
    arguments: &OriginalArguments,
    index: usize,
    purpose: tcl_syntax::naming::NativeNamePurpose,
) -> Result<Vec<u8>, tcl_cmd_core::CmdError> {
    let original = arguments.bytes(index)?;
    let policy = interp.name_policy_protocol().ok_or(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("namespace pattern issuer"),
    )?;
    policy
        .recipe()
        .namespace_pattern_input(&original, purpose)
        .map(|projection| projection.selected().to_vec())
        .map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("namespace pattern purpose")
                .into()
        })
}

fn ns_export(interp: &mut Interp, argv: &[*mut TclObj], arguments: &OriginalArguments) -> Code {
    let cur = interp.current_ns();
    if argv.len() == 2 {
        let patterns = interp.namespaces().exports(cur).to_vec();
        set_list_bytes(interp, &patterns);
        return Code::Ok;
    }
    let mut first = match namespace_pattern(
        interp,
        arguments,
        2,
        tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
    ) {
        Ok(pattern) => Some(pattern),
        Err(error) => return interp.report_cmd_error(error),
    };
    let clear = first.as_deref() == Some(b"-clear");
    let start = if clear {
        interp.namespaces_mut().clear_exports(cur);
        first = None;
        3
    } else {
        2
    };
    // Original C drivers commit clear and each accepted pattern before the next
    // String getter. A failed pattern leaves earlier effects and later originals untouched.
    for index in start..argv.len() {
        let pattern = match first.take().map(Ok).unwrap_or_else(|| {
            namespace_pattern(
                interp,
                arguments,
                index,
                tcl_syntax::naming::NativeNamePurpose::NamespaceExportPattern,
            )
        }) {
            Ok(pattern) => pattern,
            Err(error) => return interp.report_cmd_error(error),
        };
        if tcl_syntax::naming::is_qualified(&pattern) {
            let mut message = b"invalid export pattern \"".to_vec();
            message.extend_from_slice(&pattern);
            message.extend_from_slice(b"\": pattern can't specify a namespace");
            return interp.set_error(&message);
        }
        interp.namespaces_mut().export(cur, &pattern);
        if let Err(error) = arguments.check() {
            return interp.report_cmd_error(error);
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace import ?-force? pattern ?pattern ...?` — install transparent
/// redirects in the current ns for the exported commands matching each pattern.
fn ns_import(interp: &mut Interp, argv: &[*mut TclObj], arguments: &OriginalArguments) -> Code {
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Import,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
    }
    if argv.len() > 2 {
        interp.invalidate_namespace_structure_guards();
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
    let mut first = match namespace_pattern(
        interp,
        arguments,
        2,
        tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
    ) {
        Ok(pattern) => Some(pattern),
        Err(error) => return interp.report_cmd_error(error),
    };
    let force = first.as_deref() == Some(b"-force");
    let start = if force {
        first = None;
        3
    } else {
        2
    };
    for index in start..argv.len() {
        let pattern = match first.take().map(Ok).unwrap_or_else(|| {
            namespace_pattern(
                interp,
                arguments,
                index,
                tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern,
            )
        }) {
            Ok(pattern) => pattern,
            Err(error) => return interp.report_cmd_error(error),
        };
        let pat = pattern.as_slice();
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
        let mut to_import: Vec<Vec<u8>> = Vec::new();
        for name in interp.visible_command_names_in(src_ns) {
            if glob_match_bytes(&tail_pat, &name) && interp.namespaces().is_exported(src_ns, &name)
            {
                to_import.push(name);
            }
        }
        for simple in to_import {
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
                let destination_fqn = interp.namespaces().command_fqn_at(dest, &simple);
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
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `namespace forget ?pattern ...?` — remove matching imported bindings.
fn ns_forget(interp: &mut Interp, argv: &[*mut TclObj], arguments: &OriginalArguments) -> Code {
    use tcl_syntax::naming::{NativeNameContext, NativeNamespacePatternSource};
    use tcl_syntax::value::ValueError;

    interp.invalidate_namespace_structure_guards();
    let dest = interp.current_ns();
    for index in 2..argv.len() {
        let original = match arguments.bytes(index) {
            Ok(original) => original,
            Err(error) => return interp.report_cmd_error(error),
        };
        let Some(policy) = interp.name_policy_protocol() else {
            return interp.report_cmd_error(
                ValueError::CommandProtocolUnavailable("namespace forget pattern issuer").into(),
            );
        };
        let path = interp.namespaces().native_context_path(dest);
        let Some(path) = path else {
            return interp.report_cmd_error(
                ValueError::CommandProtocolUnavailable("namespace forget current namespace").into(),
            );
        };
        let parts = match policy.recipe().namespace_pattern_parts(
            NativeNameContext::new(&path),
            &original,
            tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern,
        ) {
            Ok(parts) => parts,
            Err(_) => {
                return interp.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("namespace forget pattern purpose")
                        .into(),
                );
            }
        };
        let source = match &parts.source {
            Some(NativeNamespacePatternSource::C(path)) => {
                let source = interp.namespaces().namespace_at_path(path);
                let Some(source) = source else {
                    let mut message = b"unknown namespace in namespace forget pattern \"".to_vec();
                    message.extend_from_slice(parts.selected.as_bytes());
                    message.push(b'"');
                    if interp.runtime_version() >= tcl_dialect::TclVersion::V8_5 {
                        let error_code = error_code_list(&[
                            b"TCL",
                            b"LOOKUP",
                            b"NAMESPACE",
                            parts.selected.as_bytes(),
                        ]);
                        return interp.error_with_code(&message, &error_code);
                    }
                    return interp.set_error(&message);
                };
                Some(source)
            }
            None => None,
            Some(NativeNamespacePatternSource::Jim(_)) => {
                return interp.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("Jim namespace forget pattern").into(),
                );
            }
        };
        let victims = interp.namespaces().imported_forget_bindings(
            dest,
            source,
            parts.tail.as_bytes(),
            tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy),
        );
        let victims = match victims {
            Ok(victims) => victims,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        for (simple, generation) in victims {
            if interp.uses_original_c_command_retirement() {
                interp.finish_original_command_deletion(generation, true);
            } else if interp.namespaces().command_generation(dest, &simple) == Some(generation) {
                interp.namespaces_mut().remove_in(dest, &simple);
            }
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// path

/// `namespace path ?nsList?` — query (FQN list) or set the current ns's path.
fn ns_path(interp: &mut Interp, argv: &[*mut TclObj], arguments: &OriginalArguments) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"?nsList?");
    }
    let owner = match interp.original_namespace_consumer(arguments) {
        Ok(owner) => owner,
        Err(error) => return interp.report_cmd_error(error),
    };
    let check = || owner.check(arguments);
    let cur = interp.current_ns();
    if argv.len() == 2 {
        let path = interp.namespaces().path(cur).to_vec();
        for &namespace in &path {
            if let Err(error) = owner.observe(namespace, arguments) {
                return interp.report_cmd_error_using(error, Some(&check));
            }
        }
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
                if let Err(error) = check() {
                    crate::interp::drop_fresh(result);
                    return interp.report_cmd_error_using(error, Some(&check));
                }
                interp.set_result(result);
                match check() {
                    Ok(()) => Code::Ok,
                    Err(error) => interp.report_cmd_error_using(error, Some(&check)),
                }
            }
            Err(error) => interp.report_cmd_error_using(error.into(), Some(&check)),
        };
    }
    // Retain the original list members through namespace conversion.
    let original = obj::Owned::retain(argv[2]);
    let elems = match namespace_list_elements_using(interp, original.as_ptr(), &check, arguments) {
        Ok(elements) => elements,
        Err(error) => return interp.report_cmd_error_using(error, Some(&check)),
    };
    let primary = match obj::NativePrimaryReceipt::capture(original.as_ptr()) {
        Ok(primary) => primary,
        Err(error) => return interp.report_cmd_error_using(error.into(), Some(&check)),
    };
    let backing = list::native_list_backing(original.as_ptr());
    let members = elems
        .iter()
        .map(|&member| obj::NativeObjectLifetime::retain(member))
        .collect::<Vec<_>>();
    let member_check = || {
        check()?;
        primary
            .check_for(original.as_ptr(), "original namespace path List primary")
            .map_err(|error| arguments.retain_refusal(error.into()))?;
        if let Some(backing) = &backing {
            backing
                .check_original_header(original.as_ptr())
                .map_err(|error| arguments.retain_refusal(error.into()))?;
        }
        for member in &members {
            member
                .checked_ptr()
                .map_err(|error| arguments.retain_refusal(error.into()))?;
        }
        Ok(())
    };
    let mut path: Vec<NsId> = Vec::with_capacity(elems.len());
    for &element in &elems {
        let ns = match interp.native_namespace_object_lookup_at_using(
            cur,
            element,
            &member_check,
            &|error| arguments.retain_refusal(error.into()),
        ) {
            Ok(Some(namespace)) => namespace,
            Ok(None) => {
                return ns_operation_not_found_using(
                    interp,
                    element,
                    tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
                    Some(&member_check),
                );
            }
            Err(error) => return interp.report_cmd_error_using(error, Some(&check)),
        };
        if let Err(error) = owner.observe(ns, arguments) {
            return interp.report_cmd_error_using(error, Some(&member_check));
        }
        path.push(ns);
    }
    if let Err(error) = member_check() {
        return interp.report_cmd_error_using(error, Some(&member_check));
    }
    interp.namespaces_mut().set_path(cur, path);
    interp.set_result_bytes(b"");
    match check() {
        Ok(()) => Code::Ok,
        Err(error) => interp.report_cmd_error_using(error, Some(&check)),
    }
}

fn namespace_list_elements_using(
    interp: &mut Interp,
    original: *mut TclObj,
    check: &impl Fn() -> Result<(), tcl_cmd_core::CmdError>,
    arguments: &OriginalArguments,
) -> Result<Vec<*mut TclObj>, tcl_cmd_core::CmdError> {
    check()?;
    let protocol = interp
        .native_invocation_dialect()
        .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        ))
        .ok_or_else(|| {
            arguments.retain_refusal(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native object list conversion",
                )
                .into(),
            )
        })?
        .protocol();
    if protocol.is_jim084() {
        let context = interp
            .native_jim_object_context()
            .map_err(|error| arguments.retain_refusal(error.into()))?;
        check()?;
        crate::native_source::bind_context(original, &context)
            .map_err(|error| arguments.retain_refusal(error.into()))?;
        check()?;
    }
    list::list_elements_native_using(original, protocol, check, &|error| {
        arguments.retain_refusal(error.into())
    })
}

// helpers

/// Report the reached native operation's error without changing the name extent.
pub(crate) fn ns_operation_not_found(
    interp: &mut Interp,
    original: *mut TclObj,
    operation: tcl_syntax::naming::NativeNamespaceLookupOperation,
) -> Code {
    ns_operation_not_found_using(interp, original, operation, None)
}

pub(crate) fn ns_operation_not_found_using(
    interp: &mut Interp,
    original: *mut TclObj,
    operation: tcl_syntax::naming::NativeNamespaceLookupOperation,
    stage: Option<&dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>,
) -> Code {
    let bytes = match interp.original_c_name_bytes_using(original, stage) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error_using(error, stage),
    };
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error_using(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "namespace operation diagnostic issuer",
            )
            .into(),
            stage,
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
            return interp.report_cmd_error_using(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "namespace operation diagnostic purpose",
                )
                .into(),
                stage,
            );
        }
    };
    publish_namespace_error_using(interp, error, stage)
}

/// Publish the selected namespace diagnostic through the actual String/error owner.
pub(crate) fn publish_namespace_error_using(
    interp: &mut Interp,
    error: tcl_syntax::naming::NativeNamespaceOperationError,
    stage: Option<&dyn Fn() -> Result<(), tcl_cmd_core::CmdError>>,
) -> Code {
    let materialization = match interp
        .native_invocation_dialect()
        .native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )) {
        Some(materialization) if materialization.protocol() == error.string_protocol => {
            materialization
        }
        _ => {
            return interp.report_cmd_error_using(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "namespace diagnostic String producer issuer",
                )
                .into(),
                stage,
            );
        }
    };
    let code = match error.error_code {
        Some(code) => interp.error_with_code_using(&error.message, &code, stage),
        None => interp.error_using(&error.message, stage),
    };
    if let Some(check) = stage {
        if let Err(error) = check() {
            return interp.report_cmd_error(error);
        }
    }
    if let Err(error) =
        obj::retain_native_string_representation(interp.result_obj(), materialization)
    {
        return interp.report_cmd_error_using(error.into(), stage);
    }
    if let Some(check) = stage {
        if let Err(error) = check() {
            return interp.report_cmd_error(error);
        }
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
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Inscope,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
    }
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
        let body = match interp.concat_original_arguments(&[argv[3], tail.as_ptr()]) {
            Ok(body) => obj::Owned::fresh(body),
            Err(error) => return interp.report_cmd_error(error),
        };
        (
            body,
            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceConcat,
        )
    };
    let location = interp.arg_location(argv[3]);
    // An error leaving the script is framed `(in namespace inscope "<fqn>"
    // script line N)` (C's `NsEval_Callback` under `namespace inscope`).
    interp.with_namespace_frame_command("inscope", |interp| {
        interp.ns_eval_in_token(
            namespace,
            &[],
            location,
            true,
            Some((purpose, body.as_ptr())),
            Some(argv),
        )
    })
}

/// `namespace origin command` — the fully-qualified original name of `command`
/// (following `namespace import` chains to the source).
fn ns_origin(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Origin,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
    }
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"name");
    }
    let origin = if interp
        .native_invocation_dialect()
        .native_command_name_protocol()
        .is_some()
    {
        interp
            .native_namespace_origin(argv[2])
            .map_err(tcl_cmd_core::CmdError::from)
    } else {
        // Non-native introspection retains its separately selected byte query.
        let name = match interp.native_string_bytes(&argv[2]) {
            Ok(name) => name,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        tcl_cmd_core::namespace::origin_bytes_checked(interp, &name)
    };
    let origin = match origin {
        Ok(origin) => origin,
        Err(error) => return interp.report_cmd_error(error),
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
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Code,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
    }
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
fn ns_upvar(interp: &mut Interp, argv: &[*mut TclObj], originals: &OriginalArguments) -> Code {
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Upvar,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
    }
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
    let owner = match interp.original_namespace_consumer(originals) {
        Ok(owner) => owner,
        Err(error) => return interp.report_cmd_error(error),
    };
    let check = || owner.check(originals);
    let current = interp.current_ns();
    let protocol = match interp.require_variable_name_protocol() {
        Ok(protocol) => protocol,
        Err(error) => return crate::builtins::var_error(interp, b"", error),
    };
    let ns =
        match interp.native_namespace_object_lookup_at_using(current, argv[2], &check, &|error| {
            originals.retain_refusal(error.into())
        }) {
            Ok(Some(namespace)) => namespace,
            Ok(None) => {
                return ns_operation_not_found_using(
                    interp,
                    argv[2],
                    tcl_syntax::naming::NativeNamespaceLookupOperation::ObjectLookup,
                    Some(&check),
                );
            }
            Err(error) => return interp.report_cmd_error_using(error, Some(&check)),
        };
    if let Err(error) = owner.observe(ns, originals) {
        return interp.report_cmd_error_using(error, Some(&check));
    }
    for (target_word, local_word) in arguments.pairs() {
        let local_word = local_word.expect("selected C grammar has complete pairs") + 2;
        let bytes = match originals.bytes_using(local_word, &check) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error_using(error, Some(&check)),
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
        let code = interp.link_original_c_namespace_objects_using(
            argv[target_word + 2],
            ns,
            local.as_ptr(),
            Some(&check),
        );
        drop(local);
        if let Err(error) = check() {
            return interp.report_cmd_error_using(error, Some(&check));
        }
        if code != Code::Ok {
            return code;
        }
    }
    interp.set_result_bytes(b"");
    match check() {
        Ok(()) => Code::Ok,
        Err(error) => interp.report_cmd_error_using(error, Some(&check)),
    }
}

// ensemble

/// `namespace ensemble create|exists ...` — the canonical `ens sub`→target
/// redirect (the generalised `dict for`→`::tcl::dict::for` mechanism).
fn ns_ensemble(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if let Some(helper) = interp
        .native_invocation_dialect()
        .native_jim_namespace_helper(
            tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Ensemble,
        )
    {
        return interp.dispatch_stock_jim_namespace_helper(argv, helper);
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
        .invocation_name_protocol()
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
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
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
    let cmd = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
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
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    if argv.len() != 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"exists cmdname");
    }
    let name = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let exists = interp.is_ensemble(&name);
    interp.set_result_bytes(if exists { b"1" } else { b"0" });
    Code::Ok
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_ensemble_tests;

#[cfg(test)]
mod native_upvar_tests;

#[cfg(test)]
mod native_holder_routing;

#[cfg(test)]
mod native_map_prefix_tests;

#[cfg(test)]
pub(crate) mod original_argument_tests;

#[cfg(test)]
mod original_import_alias_tests;

#[cfg(test)]
#[path = "cmd_namespace/native_jim_code_tests.rs"]
mod native_jim_code_tests;

#[cfg(test)]
#[path = "cmd_namespace/native_jim_origin_tests.rs"]
mod native_jim_origin_tests;

#[cfg(test)]
#[path = "cmd_namespace/native_jim_inscope_tests.rs"]
mod native_jim_inscope_tests;

#[cfg(test)]
#[path = "cmd_namespace/native_jim_delete_tests.rs"]
mod native_jim_delete_tests;

#[cfg(test)]
#[path = "cmd_namespace/native_jim_upvar_tests.rs"]
mod native_jim_upvar_tests;

#[cfg(test)]
#[path = "cmd_namespace/native_jim_parent_tests.rs"]
mod native_jim_parent_tests;

#[cfg(test)]
#[path = "cmd_namespace/native_jim_text_result_tests.rs"]
mod native_jim_text_result_tests;

#[cfg(test)]
#[path = "cmd_namespace/native_jim_helper_literal_head_tests.rs"]
mod native_jim_helper_literal_head_tests;

#[cfg(test)]
mod original_namespace_continuation_tests;
