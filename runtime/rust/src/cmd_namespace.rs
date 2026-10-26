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
mod tests {
    use crate::interp::{Code, Command, Interp};
    use crate::{counters, list, obj};

    thread_local! {
        static WHICH_NAMESPACE: std::cell::Cell<*mut obj::TclObj> = const {
            std::cell::Cell::new(std::ptr::null_mut())
        };
        static WHICH_NAME_OBSERVATIONS: std::cell::RefCell<Vec<bool>> = const {
            std::cell::RefCell::new(Vec::new())
        };
    }

    extern "C" fn update_which_probe_name(value: *mut obj::TclObj) {
        let resident = WHICH_NAMESPACE.with(|namespace| obj::has_string_rep(namespace.get()));
        WHICH_NAME_OBSERVATIONS.with(|observations| observations.borrow_mut().push(resident));
        let bytes: &[u8] = match obj::internal_rep(value) {
            0 => b"::absent",
            1 => b"absent",
            2 => b"::return",
            3 => b"return",
            _ => unreachable!("fixed original name control"),
        };
        // SAFETY: the updater owns the live original object's String slot.
        unsafe { obj::set_string_rep(value, bytes) };
    }

    static WHICH_PROBE_NAME: obj::TclObjType = obj::TclObjType {
        name: c"which-name-probe".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update_which_probe_name),
        set_from_any_proc: None,
    };

    #[test]
    fn original_jim_which_helper_preserves_public_and_direct_cold_and_hot_getter_order() {
        use tcl_runtime_api::Namespaces;
        let evidence =
            include_str!("../tests/data/native_jim_namespace_canonical/which-helper-20261015.tsv");
        let row = |kind: &str, label: &str| {
            evidence
                .lines()
                .find(|line| {
                    let mut fields = line.split('|');
                    fields.next() == Some(kind) && fields.next() == Some(label)
                })
                .unwrap()
                .split('|')
                .collect::<Vec<_>>()
        };
        for (label, option, case, direct) in [
            (
                "PUBLIC_VARIABLE_ABSOLUTE",
                b"-variable".as_slice(),
                0,
                false,
            ),
            (
                "PUBLIC_VARIABLE_RELATIVE",
                b"-variable".as_slice(),
                1,
                false,
            ),
            ("PUBLIC_COMMAND_ABSOLUTE", b"-command".as_slice(), 2, false),
            ("PUBLIC_COMMAND_RELATIVE", b"-command".as_slice(), 3, false),
            ("DIRECT_VARIABLE_ABSOLUTE", b"-variable".as_slice(), 0, true),
            ("DIRECT_VARIABLE_RELATIVE", b"-variable".as_slice(), 1, true),
            ("DIRECT_COMMAND_ABSOLUTE", b"-command".as_slice(), 2, true),
            ("DIRECT_COMMAND_RELATIVE", b"-command".as_slice(), 3, true),
        ] {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jimtcl"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            crate::cmd_proc::install_stock_scripted_library(
                &mut interp,
                tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers,
            );
            let context = interp.native_jim_object_context().unwrap();
            let namespace = obj::Owned::fresh(obj::new_wide_int_obj(42));
            crate::native_source::bind_context(namespace.as_ptr(), &context).unwrap();
            let name = obj::Owned::fresh(obj::alloc_typed(&WHICH_PROBE_NAME, case));
            crate::native_source::bind_context(name.as_ptr(), &context).unwrap();
            let head = obj::Owned::fresh(obj::new_string_bytes(if direct {
                b"namespace which"
            } else {
                b"namespace"
            }));
            let which = obj::Owned::fresh(obj::new_string_bytes(b"which"));
            let option = obj::Owned::fresh(obj::new_string_bytes(option));
            let argv = if direct {
                vec![head.as_ptr(), option.as_ptr(), name.as_ptr()]
            } else {
                vec![
                    head.as_ptr(),
                    which.as_ptr(),
                    option.as_ptr(),
                    name.as_ptr(),
                ]
            };
            let body = obj::Owned::fresh(interp.new_list_object(&argv));
            WHICH_NAMESPACE.with(|slot| slot.set(namespace.as_ptr()));
            for hot in [false, true] {
                let phase = format!("{label}_{}", if hot { "HOT" } else { "COLD" });
                if hot {
                    assert_eq!(
                        obj::with_jim_command_cache(head.as_ptr(), |cache| cache
                            .namespace
                            .as_ptr()),
                        Some(namespace.as_ptr()),
                    );
                    obj::invalidate_string(namespace.as_ptr());
                    obj::invalidate_string(name.as_ptr());
                }
                let current = interp
                    .namespaces_mut()
                    .retain_unmaterialized_jim_namespace(obj::Owned::retain(namespace.as_ptr()));
                assert_eq!(
                    obj::has_string_rep(namespace.as_ptr()),
                    row("NAMESPACE_BEFORE", &phase)[2] == "1"
                );
                assert!(interp.namespaces().jim_namespace_bytes(current).is_none());
                assert!(interp
                    .namespace_variable_name_bytes_checked(
                        tcl_runtime_api::NsId(current as u32),
                        b"absent"
                    )
                    .is_err());
                assert!(!obj::has_string_rep(namespace.as_ptr()));
                WHICH_NAME_OBSERVATIONS.with(|observations| observations.borrow_mut().clear());
                assert_eq!(
                    interp.ns_eval_in_token(
                        current,
                        &[],
                        None,
                        false,
                        Some((
                            tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                            body.as_ptr()
                        )),
                        None
                    ),
                    Code::Ok,
                    "{phase}"
                );
                assert_eq!(
                    WHICH_NAME_OBSERVATIONS.with(|observations| observations.borrow().clone()),
                    vec![row("NAMESPACE_AT_NAME_GETTER", &phase)[2] == "1"]
                );
                assert_eq!(
                    obj::has_string_rep(namespace.as_ptr()),
                    row("NAMESPACE_AFTER", &phase)[2] == "1"
                );
                assert_eq!(
                    obj::obj_type_ptr(name.as_ptr()),
                    &WHICH_PROBE_NAME as *const obj::TclObjType
                );

                let expected = row("RESULT", &phase)[3]
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(interp.result_bytes(), expected, "{phase}");
                assert!(interp.native_execution_refusal().is_none());
            }
            interp.set_current_ns(crate::namespace::GLOBAL);
            WHICH_NAMESPACE.with(|slot| slot.set(std::ptr::null_mut()));
        }
    }

    #[test]
    fn jim_namespace_which_dispatches_the_actual_replaceable_helper_allocation() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        crate::cmd_proc::install_stock_scripted_library(
            &mut interp,
            tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers,
        );
        let helper = interp
            .resolve_cmd_token(b"namespace which")
            .expect("actual helper allocation");
        assert!(matches!(
            interp
                .namespaces()
                .command_by_generation(helper)
                .map(|(_, command)| command),
            Some(Command::Proc(_))
        ));
        assert_eq!(
            interp.eval_str(b"namespace which -variable absent"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"::absent");
        assert_eq!(interp.eval_str(b"rename {namespace which} saved"), Code::Ok);
        assert_eq!(interp.resolve_cmd_token(b"saved"), Some(helper));
        assert_eq!(
            interp.eval_str(
                b"proc {namespace which} args {return OVERRIDE}; namespace which -variable absent"
            ),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"OVERRIDE");
        let replacement = interp.resolve_cmd_token(b"namespace which").unwrap();
        assert_ne!(replacement, helper);
        interp.install_stock_jim_namespace_helpers();
        assert_eq!(
            interp.resolve_cmd_token(b"namespace which"),
            Some(replacement)
        );
        assert_eq!(
            interp.eval_str(b"{namespace which} -variable absent"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"OVERRIDE");
        assert_eq!(
            interp.eval_str(b"rename {namespace which} {}; namespace which -variable absent"),
            Code::Error
        );
        assert!(interp.resolve_cmd_token(b"namespace which").is_none());
        assert!(interp.native_execution_refusal().is_none());
    }

    #[test]
    fn jim_namespace_which_refresh_preserves_the_actual_replacement() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interp.eval_str(b"proc {namespace which} args {return SELECTED}"),
            Code::Ok
        );
        let selected = interp.resolve_cmd_token(b"namespace which").unwrap();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.6"));
        assert_eq!(interp.resolve_cmd_token(b"namespace which"), Some(selected));
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jimtcl"));
        assert_eq!(interp.resolve_cmd_token(b"namespace which"), Some(selected));
        assert_eq!(
            interp.eval_str(b"namespace which -variable ignored"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"SELECTED");
        assert!(interp.native_execution_refusal().is_none());
    }

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
                assert!(crate::ensemble::NativeEnsembleObjects::pointer(
                    &another_query.originals.map
                )
                .is_none());
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
                        assert_eq!(crate::interp::obj_bytes(elements[0]), b"::namespace");
                        assert_eq!(crate::interp::obj_bytes(elements[1]), b"inscope");
                        assert_eq!(crate::interp::obj_bytes(elements[2]), b"::");
                        assert_eq!(elements[3], script.as_ptr(), "{}: {index}", profile.name);
                    }
                    assert_eq!(crate::interp::obj_bytes(script.as_ptr()), bytes);
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
                assert_eq!(crate::interp::obj_bytes(retained), bytes);
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
            assert!(interp
                .native_invocation_dialect()
                .native_namespace_code_policy()
                .is_none());
        });
    }

    #[test]
    fn jim_namespace_ensemble_uses_the_mutable_helper_and_original_arguments() {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jimtcl"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(
            interp.eval_str(br#"proc {namespace ensemble} {subcommand args} {return "[llength $args]:[lindex $args 0]:[lindex $args 1]"}"#),
            Code::Ok
        );
        assert_eq!(
            interp.eval_str(b"namespace ensemble custom {two words} tail"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"2:two words:tail");
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
            leak_free_with(
                || {
                    Interp::with_native_core(
                        crate::interp::default_host(),
                        profile,
                        tcl_registry::special_vars::NativeBootstrapInputs::default(),
                    )
                    .unwrap()
                },
                |interp| {
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
                },
            );
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
        leak_free_with(
            || {
                Interp::with_native_core(
                    crate::interp::default_host(),
                    profile,
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap()
            },
            |interp| {
                let script = br#"namespace eval S {proc p {} {return SOURCE}}; namespace eval D {proc p {} {return DESTINATION}; namespace import -force ::S::*; set imported [namespace import]}; rename S::p {}; set result [list [llength [info commands D::p]] [catch {D::p}] [catch {namespace origin D::p}] [namespace eval D {namespace import ::missing::*}] [namespace eval D {namespace export}]]; proc S::p {} {return REVIVED}; lappend result [D::p] [namespace origin D::p] [catch {namespace path}]; set result"#;
                assert_eq!(
                    interp.eval_str(script),
                    Code::Ok,
                    "{:?}",
                    interp.result_bytes()
                );
                assert_eq!(interp.result_bytes(), b"1 1 1 {} {} REVIVED ::S::p 1");
            },
        );
    }

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        leak_free_with(Interp::new, body);
    }

    fn leak_free_with(create: impl FnOnce() -> Interp, body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = create();
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

    fn forget_native_fields(
        stdout: &'static str,
    ) -> std::collections::BTreeMap<&'static str, Vec<u8>> {
        stdout
            .lines()
            .map(|line| {
                let (name, hex) = line
                    .strip_prefix("FIELD ")
                    .unwrap()
                    .split_once(' ')
                    .unwrap();
                let bytes = hex
                    .as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                (name, bytes)
            })
            .collect()
    }

    #[test]
    fn native_c86_forget_global_origin_matches_original_issue_witness() {
        let fields = forget_native_fields(include_str!(
            "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/namespace-forget-global-origin/results/tcl8.6.18/stdout.txt"
        ));
        leak_free_with(
            || native_namespace_interp(tcl_dialect::TclVersion::V8_6),
            |interp| {
                assert_eq!(fields["setup_code"], b"0");
                let code = interp.eval_str(include_bytes!("../../../rust/tcl-registry/tests/data/source_contract_vectors/results/namespace-forget-global-origin/issue-witness.tcl"));
                assert_eq!(
                    code.as_int().to_string().as_bytes(),
                    fields["positive_code"]
                );
                assert_eq!(interp.result_bytes(), fields["positive_result"]);
                assert!(!interp.host_refusal_pending());
            },
        );
    }

    #[test]
    fn native_c86_forget_global_origin_matches_original_adverse_controls() {
        let fields = forget_native_fields(include_str!(
            "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/namespace-forget-global-origin/results/tcl8.6.18/stdout.txt"
        ));
        let probe = include_str!(
            "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/namespace-forget-global-origin/probe.tcl"
        );
        let block = |start: &str, end: &str| {
            probe
                .split_once(start)
                .unwrap()
                .1
                .split_once(end)
                .unwrap()
                .0
                .as_bytes()
        };
        let line = |prefix: &str| {
            probe
                .lines()
                .find(|line| line.starts_with(prefix))
                .unwrap()
                .as_bytes()
        };
        leak_free_with(
            || native_namespace_interp(tcl_dialect::TclVersion::V8_6),
            |interp| {
                assert_eq!(
                    interp.eval_str(block("set setup_code [catch {\n", "} setup_result]")),
                    Code::Ok
                );
                assert_eq!(interp.result_bytes(), fields["setup_result"]);
                assert_eq!(
                    interp.eval_str(block("set positive_code [catch {\n", "} positive_result]")),
                    Code::Ok
                );
                assert_eq!(interp.result_bytes(), fields["positive_result"]);
                assert_eq!(
                    interp.eval_str(block(
                        "set adverse_setup_code [catch {\n",
                        "} adverse_setup_result]"
                    )),
                    Code::Ok
                );
                assert_eq!(interp.result_bytes(), fields["adverse_setup_result"]);
                assert_eq!(interp.eval_str(line("set adverse_code ")), Code::Ok);
                assert_eq!(interp.result_bytes(), fields["adverse_code"]);
                assert_eq!(interp.eval_str(b"set adverse_result"), Code::Ok);
                assert_eq!(interp.result_bytes(), fields["adverse_result"]);
                assert_eq!(interp.eval_str(b"set ::errorCode"), Code::Ok);
                assert_eq!(interp.result_bytes(), fields["adverse_error_code"]);
                assert_eq!(interp.eval_str(b"llength [info commands ::n::h]"), Code::Ok);
                assert_eq!(interp.result_bytes(), fields["imported_other_present"]);
                let local_forget = line("field local_forget_code ")
                    .strip_prefix(b"field local_forget_code [")
                    .unwrap()
                    .strip_suffix(b"]")
                    .unwrap();
                assert_eq!(interp.eval_str(local_forget), Code::Ok);
                assert_eq!(interp.result_bytes(), fields["local_forget_code"]);
                assert_eq!(interp.eval_str(b"set local_forget_result"), Code::Ok);
                assert_eq!(interp.result_bytes(), fields["local_forget_result"]);
                for (query, field) in [
                    (
                        b"llength [info commands ::n::local]".as_slice(),
                        "local_present",
                    ),
                    (
                        b"llength [info commands ::n::h]".as_slice(),
                        "imported_other_after_local_forget",
                    ),
                    (
                        b"llength [info commands ::g]".as_slice(),
                        "global_source_present",
                    ),
                ] {
                    assert_eq!(interp.eval_str(query), Code::Ok);
                    assert_eq!(interp.result_bytes(), fields[field]);
                }
                assert!(!interp.host_refusal_pending());
            },
        );
    }

    #[test]
    fn native_c86_forget_matches_original_import_chain_controls() {
        let fields = forget_native_fields(include_str!(
            "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/namespace-forget-import-chain/results/tcl8.6.18/stdout.txt"
        ));
        let probe = include_str!(
            "../../../rust/tcl-registry/tests/data/source_contract_vectors/results/namespace-forget-import-chain/probe.tcl"
        );
        let block = |start: &str, end: &str| {
            probe
                .split_once(start)
                .unwrap()
                .1
                .split_once(end)
                .unwrap()
                .0
                .as_bytes()
        };
        leak_free_with(
            || native_namespace_interp(tcl_dialect::TclVersion::V8_6),
            |interp| {
                let setup = interp.eval_str(block("set setup_code [catch {\n", "} setup_result]"));
                assert_eq!(setup.as_int().to_string().as_bytes(), fields["setup_code"]);
                assert_eq!(interp.result_bytes(), fields["setup_result"]);
                for case in [
                    "ultimate",
                    "immediate",
                    "middle",
                    "tail_precedence",
                    "own_name",
                ] {
                    let start = format!("set {case}_code [catch {{\n");
                    let end = format!("}} {case}_result]");
                    let code = interp.eval_str(block(&start, &end));
                    assert_eq!(
                        code.as_int().to_string().as_bytes(),
                        fields[format!("{case}_code").as_str()],
                        "{case}"
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        fields[format!("{case}_result").as_str()],
                        "{case}"
                    );
                    assert!(!interp.host_refusal_pending(), "{case}");
                }
            },
        );
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

    #[test]
    fn mutually_entangled_deletion_preserves_the_global_namespace() {
        for version in tcl_dialect::TclVersion::ALL {
            leak_free_with(
                || native_namespace_interp(version),
                |interp| {
                    assert_eq!(
                    interp.eval_str(
                        br#"namespace eval ::testing {
                            namespace eval abc {proc xyz {} {}}
                            namespace eval def {proc xyz {} {}}
                            trace add command abc::xyz delete "namespace delete ::testing::def {}; #"
                            trace add command def::xyz delete "namespace delete ::testing::abc {}; #"
                        }
                        namespace delete ::testing
                        list [namespace exists ::testing] [info commands set]"#,
                    ),
                    Code::Ok,
                    "{version:?}: {:?}",
                    interp.result_bytes(),
                );
                    assert_eq!(interp.result_bytes(), b"0 set", "{version:?}");
                },
            );
        }
    }

    #[test]
    fn child_teardown_distinguishes_cached_names_from_fresh_lookup() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            leak_free_with(
                || native_namespace_interp(version),
                |interp| {
                    assert_eq!(
                        interp.eval_str(
                            br#"set seen {}
                        proc observe {tag old new op} {
                            set fresh ::P
                            append fresh ::C
                            lappend ::seen [list $tag [namespace exists ::P::C] \
                                [namespace exists $fresh]]
                        }
                        namespace eval ::P {
                            namespace ensemble create -command ::PE
                            namespace eval C {namespace ensemble create -command ::CE}
                        }
                        trace add command ::PE delete [list observe parent]
                        trace add command ::CE delete [list observe child]
                        namespace delete ::P
                        set seen"#,
                        ),
                        Code::Ok,
                        "{version:?}: {:?}",
                        interp.result_bytes(),
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        b"{parent 1 1} {child 1 0}",
                        "{version:?}",
                    );
                },
            );
        }
    }

    #[test]
    fn implicit_ensemble_targets_preserve_the_selected_namespace_context() {
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
        {
            leak_free_with(
                || native_namespace_interp(version),
                |interp| {
                    assert_eq!(
                        interp.eval_str(br#"namespace eval ::M {
                            namespace ensemble create -command path -subcommands {add remove}
                            proc add {path} {return [list ADD [namespace current] $path]}
                        }
                        proc ::add {path} {return [list GLOBAL $path]}
                        proc ::remove {path} {return [list GLOBAL_REMOVE $path]}
                        namespace eval ::Caller {proc add {path} {return [list CALLER $path]}}
                        list [::M::path add x] [namespace eval ::Caller {::M::path add y}] [::add z]"#),
                        Code::Ok,
                        "{version:?}: {:?}",
                        interp.result_bytes(),
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        b"{ADD ::M x} {ADD ::M y} {GLOBAL z}",
                        "{version:?}"
                    );
                    assert_eq!(interp.eval_str(br#"set code [catch {namespace eval ::Caller {::M::path remove q}} result]; list $code $result"#), Code::Ok);
                    let expected = if version == tcl_dialect::TclVersion::V8_5 {
                        b"1 {invalid command name \"::M::remove\"}".as_slice()
                    } else {
                        b"0 {GLOBAL_REMOVE q}".as_slice()
                    };
                    assert_eq!(interp.result_bytes(), expected, "{version:?}");
                    assert!(interp.native_execution_refusal().is_none());
                },
            );
        }
    }

    fn native_namespace_interp(version: tcl_dialect::TclVersion) -> Interp {
        Interp::with_native_core(
            std::rc::Rc::new(tcl_host_native::NativeHost::new()),
            tcl_registry::model::resolve_environment(version.dialect_profile_name()).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("selected native namespace core")
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

    #[test]
    fn namespace_import_cycle_reports_the_root_command_from_its_slot() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"proc p {} {return P}
                      namespace export p
                      namespace eval S {
                          namespace import ::p
                          namespace export p
                      }
                      set code [catch {namespace import -force ::S::p} message options]
                      list $code $message [dict get $options -errorcode] \
                           [namespace origin ::p] [namespace origin ::S::p]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {import pattern \"::S::p\" would create a loop containing command \"::p\"} \
                  {TCL IMPORT LOOP} ::p ::p"
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
                tcl_cmd_core::namespace::origin_bytes_checked(i, raw)
                    .unwrap()
                    .as_deref(),
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
                tcl_cmd_core::namespace::origin_bytes_checked(
                    i,
                    String::from_utf8_lossy(raw).as_bytes()
                )
                .unwrap(),
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

#[cfg(test)]
pub(crate) mod original_argument_tests;

#[cfg(test)]
mod original_import_alias_tests {
    use super::*;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};
    use tcl_runtime_api::NativeExecutionError;

    thread_local! {
        static OWNER: RefCell<Option<Interp>> = const { RefCell::new(None) };
        static READS: Cell<usize> = const { Cell::new(0) };
        static MODE: Cell<u8> = const { Cell::new(0) };
        static FIRST: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
        static DESTINATION: Cell<NsId> = const { Cell::new(GLOBAL) };
        static GETTER_RESULT: RefCell<Option<obj::NativeObjectLifetime>> = const { RefCell::new(None) };
    }

    extern "C" fn update(original: *mut TclObj) {
        READS.with(|reads| reads.set(reads.get() + 1));
        let mode = MODE.with(Cell::get);
        let bytes: &[u8] = if mode >= 3 {
            b"::src::branch"
        } else {
            b"MEMBER"
        };
        unsafe { obj::set_native_updater_string_rep(original, bytes, false) };
        if mode == 0 || mode == 4 {
            return;
        }
        let mut interp = OWNER.with(|owner| owner.borrow().as_ref().unwrap().clone());
        let result = GETTER_RESULT.with(|result| {
            result
                .borrow()
                .as_ref()
                .map(|original| original.checked_ptr().unwrap())
        });
        if let Some(result) = result {
            interp.set_result(result);
        }
        match mode {
            1 => {
                interp.refuse_host_command("original Jim import member first cause");
                FIRST.with(|first| *first.borrow_mut() = interp.native_execution_refusal());
            }
            2 => {
                let current = interp.runtime_context();
                let mut changed = current.clone();
                changed.packages = vec![("original-import-member".to_owned(), "1.0".to_owned())];
                interp.pin_context(&changed).unwrap();
                interp.pin_context(&current).unwrap();
            }
            3 => {
                let destination = DESTINATION.with(Cell::get);
                assert_ne!(interp.current_ns(), destination);
                interp.set_current_ns(destination);
            }
            _ => unreachable!(),
        }
    }

    static TYPE: obj::TclObjType = obj::TclObjType {
        name: c"originalJimImportMember".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };

    fn native() -> Interp {
        OWNER.with(|owner| owner.borrow_mut().take());
        READS.with(|reads| reads.set(0));
        MODE.with(|mode| mode.set(0));
        FIRST.with(|first| first.borrow_mut().take());
        DESTINATION.with(|destination| destination.set(GLOBAL));
        GETTER_RESULT.with(|result| result.borrow_mut().take());
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            tcl_registry::model::resolve_environment("jim").unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        crate::cmd_proc::install_stock_scripted_library(
            &mut interp,
            tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers,
        );
        assert_eq!(
            interp.eval_str(b"namespace eval src {}; namespace eval dest {}"),
            Code::Ok
        );
        OWNER.with(|owner| *owner.borrow_mut() = Some(interp.clone()));
        interp
    }

    fn define(interp: &mut Interp, name: &[u8], members: &[*mut TclObj]) -> Owned {
        let head = Owned::fresh(obj::new_string_bytes(b"alias"));
        let name = Owned::fresh(obj::new_string_bytes(name));
        let mut argv = vec![head.as_ptr(), name.as_ptr()];
        argv.extend_from_slice(members);
        assert_eq!(interp.dispatch(&argv), Code::Ok);
        let query = tcl_runtime_api::Aliases::alias_prefix_original_value_at(
            interp,
            tcl_runtime_api::NsId(GLOBAL as u32),
            &name.as_ptr(),
        )
        .unwrap();
        let tcl_runtime_api::AliasPrefixLookup::Prefix(original) = query else {
            panic!("actual just-published alias prefix");
        };
        let stored = Owned::retain(original);
        assert_eq!(
            ValueOps::list_elements(interp, &stored.as_ptr()).unwrap(),
            members
        );
        interp.set_result_bytes(b"");
        stored
    }

    fn import(interp: &mut Interp, destination: NsId, pattern: &[u8]) -> Code {
        interp.set_current_ns(destination);
        let words = [b"namespace".as_slice(), b"import", pattern]
            .map(|word| Owned::fresh(obj::new_string_bytes(word)));
        let argv = words.each_ref().map(Owned::as_ptr);
        import_arguments(interp, destination, &argv)
    }

    fn import_arguments(interp: &mut Interp, destination: NsId, argv: &[*mut TclObj]) -> Code {
        DESTINATION.with(|selected| selected.set(destination));
        // The original helper reads its caller with uplevel. Enter the retained
        // namespace token through the existing namespace activation owner.
        let body = Owned::fresh(interp.new_list_object(argv));
        let code = interp.ns_eval_in_token(
            destination,
            b"",
            None,
            false,
            Some((
                tcl_registry::native_eval_object::EvalObjectPurpose::NamespaceBody,
                body.as_ptr(),
            )),
            None,
        );
        assert_eq!(interp.current_ns(), destination);
        assert_eq!(interp.entered_native_operation_count(), 0);
        code
    }

    fn destination(interp: &mut Interp) -> super::original_argument_tests::JimFixtureNamespace {
        super::original_argument_tests::jim_fixture_namespace(interp, b"dest")
    }

    // The pinned Jim helper publishes the generated destination at the flat
    // root table. Issue that exact slot before the tested getter/effect, then
    // inspect only retained storage after any operational refusal.
    fn destination_slot(interp: &Interp, destination: NsId) -> Vec<u8> {
        let namespaces = interp.namespaces();
        let mut generated = namespaces
            .jim_namespace_bytes(destination)
            .expect("retained reached destination namespace")
            .to_vec();
        generated.extend_from_slice(b"::branch");
        let slot = interp
            .name_policy_protocol()
            .unwrap()
            .recipe()
            .alias_publication_slot(tcl_syntax::naming::NativeNameContext::root(), &generated)
            .unwrap();
        assert!(slot.namespace.is_root());
        slot.simple.as_bytes().to_vec()
    }

    #[test]
    fn original_jim_import_refuses_a_pattern_getter_that_switches_the_current_namespace() {
        for change in [false, true] {
            let mut interp = native();
            let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
                OWNER.with(|owner| owner.borrow_mut().take());
            });
            assert_eq!(
                interp.eval_str(b"proc src::branch {} {return TARGET}"),
                Code::Ok
            );
            let destination_owner = destination(&mut interp);
            let destination = destination_owner.token;
            let slot = destination_slot(&interp, destination);
            interp.set_current_ns(destination);
            MODE.with(|mode| mode.set(if change { 3 } else { 4 }));
            let pattern = Owned::fresh(obj::alloc_typed(&TYPE, 0));
            let head = Owned::fresh(obj::new_string_bytes(b"namespace"));
            let selector = Owned::fresh(obj::new_string_bytes(b"import"));
            let code = import_arguments(
                &mut interp,
                destination,
                &[head.as_ptr(), selector.as_ptr(), pattern.as_ptr()],
            );
            assert_eq!(code, if change { Code::Error } else { Code::Ok });
            assert_eq!(READS.with(Cell::get), 1);
            assert_eq!(
                interp.namespaces().command_in(GLOBAL, &slot).is_some(),
                !change
            );
            assert_eq!(interp.native_execution_refusal().is_some(), change);
            assert_eq!(interp.entered_native_operation_count(), 0);
            interp.set_current_ns(GLOBAL);
        }
    }

    #[test]
    fn original_jim_import_prefix_keeps_the_enumerated_hash_key_after_source_deletion() {
        let mut interp = native();
        assert_eq!(
            interp.eval_str(b"namespace eval src {proc x {} {return OLD}}"),
            Code::Ok,
            "Jim source setup: native refusal {:?}; result {:?}",
            interp.native_execution_refusal(),
            interp.result_bytes()
        );
        let key = interp
            .namespaces()
            .original_jim_command_key(GLOBAL, b"src::x")
            .unwrap();
        let before = unsafe { (*key).ref_count };
        let imported = interp.eval_str(b"namespace eval dest {namespace import ::src::x}");
        assert_eq!(
            imported,
            Code::Ok,
            "Jim import: native refusal before projection {:?}; result {:?}; actual completion {:?}",
            interp.native_execution_refusal(),
            interp.result_bytes(),
            crate::completion::capture_bytes(&mut interp, imported)
        );
        let name = Owned::fresh(obj::new_string_bytes(b"dest::x"));
        let query = tcl_runtime_api::Aliases::alias_prefix_original_value_at(
            &mut interp,
            tcl_runtime_api::NsId(GLOBAL as u32),
            &name.as_ptr(),
        )
        .unwrap();
        let tcl_runtime_api::AliasPrefixLookup::Prefix(prefix) = query else {
            panic!("actual imported alias prefix");
        };
        assert_eq!(
            ValueOps::list_elements(&mut interp, &prefix).unwrap(),
            vec![key]
        );
        assert_eq!(unsafe { (*key).ref_count }, before + 1);
        assert_eq!(interp.eval_str(b"rename src::x {}"), Code::Ok);
        assert!(obj::check_native_liveness(key).is_ok());
        assert_eq!(unsafe { (*key).ref_count }, 1);
        assert!(interp
            .namespaces()
            .original_jim_command_enumeration_keys(&[b"src::x".to_vec()])
            .is_err());
        assert_eq!(
            interp.eval_str(b"namespace eval src {proc x {} {return NEW}}"),
            Code::Ok
        );
        let replacement = interp
            .namespaces()
            .original_jim_command_key(GLOBAL, b"src::x")
            .unwrap();
        assert_ne!(replacement, key);
        assert_eq!(
            ValueOps::list_elements(&mut interp, &prefix).unwrap(),
            vec![key]
        );
        assert!(!interp.host_refusal_pending());
        OWNER.with(|owner| owner.borrow_mut().take());
    }

    #[test]
    fn original_jim_import_uses_rooted_hash_key_spelling_in_helper_operations() {
        let mut interp = native();
        assert_eq!(
            interp.eval_str(b"namespace eval src {proc ::src::x {} {return OLD}}; namespace eval dest {namespace import ::src::x}; info alias dest::x"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"::src::x");
        assert_eq!(
            interp.eval_str(b"namespace eval src {namespace import ::src::x}"),
            Code::Ok
        );
        assert!(!interp.host_refusal_pending());
        OWNER.with(|owner| owner.borrow_mut().take());
    }

    #[test]
    fn original_jim_import_uses_the_cached_alias_list_without_member_getters() {

        // Software helper control: cached List and original member identity;
        // no original Enum name header or C import equivalence is inferred.
        let mut interp = native();
        let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
            OWNER.with(|owner| owner.borrow_mut().take());
        });
        let target = Owned::fresh(obj::new_string_bytes(b"list"));
        let member = Owned::fresh(obj::alloc_typed(&TYPE, 0));
        let stored = define(
            &mut interp,
            b"src::branch",
            &[target.as_ptr(), member.as_ptr()],
        );
        assert_eq!(READS.with(Cell::get), 0);
        assert_eq!(
            ValueOps::native_string_bytes(&mut interp, &stored.as_ptr())
                .unwrap()
                .as_ref(),
            b"list MEMBER"
        );
        assert_eq!(READS.with(Cell::get), 1);
        obj::invalidate_string(member.as_ptr());
        READS.with(|reads| reads.set(0));
        MODE.with(|mode| mode.set(1));
        let refs = unsafe { (*member.as_ptr()).ref_count };
        let destination_owner = destination(&mut interp);
        let destination = destination_owner.token;
        let destination_slot = destination_slot(&interp, destination);
        assert_eq!(import(&mut interp, destination, b"::src::branch"), Code::Ok);
        assert_eq!(READS.with(Cell::get), 0);
        assert!(!obj::has_string_rep(member.as_ptr()));
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, refs);
        let name = Owned::fresh(obj::new_string_bytes(b"src::branch"));
        let tcl_runtime_api::AliasPrefixLookup::Prefix(after) =
            tcl_runtime_api::Aliases::alias_prefix_original_value_at(
                &mut interp,
                tcl_runtime_api::NsId(GLOBAL as u32),
                &name.as_ptr(),
            )
            .unwrap()
        else {
            panic!("retained source alias");
        };
        assert_eq!(after, stored.as_ptr());
        assert_eq!(
            ValueOps::list_elements(&mut interp, &stored.as_ptr()).unwrap(),
            vec![target.as_ptr(), member.as_ptr()]
        );
        assert!(matches!(
            interp.namespaces().command_in(GLOBAL, &destination_slot),
            Some(Command::Alias { .. })
        ));
        assert!(interp.native_execution_refusal().is_none());
        OWNER.with(|owner| owner.borrow_mut().take());
    }

    #[test]
    fn original_jim_import_retains_reached_list_getter_first_cause_before_publication() {

        for mode in [1, 2] {
            let mut interp = native();
            let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
                GETTER_RESULT.with(|result| result.borrow_mut().take());
                OWNER.with(|owner| owner.borrow_mut().take());
            });
            let target = Owned::fresh(obj::new_string_bytes(b"list"));
            let member = Owned::fresh(obj::alloc_typed(&TYPE, 0));
            let stored = define(
                &mut interp,
                b"src::branch",
                &[target.as_ptr(), member.as_ptr()],
            );
            let backing = list::native_list_backing(stored.as_ptr()).unwrap();
            let destination_owner = destination(&mut interp);
            let destination = destination_owner.token;
            let destination_slot = destination_slot(&interp, destination);
            let prior = Owned::fresh(obj::new_string_bytes(b"PRIOR\0\xff"));
            interp.set_result(prior.as_ptr());
            // Successful helper commands precede this List getter. The updater
            // installs the retained result immediately before its Host effect.
            GETTER_RESULT.with(|result| {
                *result.borrow_mut() = Some(obj::NativeObjectLifetime::retain(prior.as_ptr()));
            });
            let refs = unsafe { (*member.as_ptr()).ref_count };
            let current = interp.runtime_context();
            MODE.with(|selected| selected.set(mode));
            assert_eq!(
                import(&mut interp, destination, b"::src::branch"),
                Code::Error
            );
            assert_eq!(READS.with(Cell::get), 1);
            assert!(interp
                .namespaces()
                .command_in(GLOBAL, &destination_slot)
                .is_none());
            assert_eq!(interp.result_obj(), prior.as_ptr());
            assert_eq!(obj::bytes_of(prior.as_ptr()), b"PRIOR\0\xff");
            assert_eq!(interp.runtime_context(), current);
            assert_eq!(unsafe { (*member.as_ptr()).ref_count }, refs);
            assert_eq!(
                &*backing.elements().unwrap(),
                &[target.as_ptr(), member.as_ptr()]
            );
            let cause = interp
                .native_execution_refusal()
                .expect("first original operational cause");
            if mode == 1 {
                assert_eq!(Some(cause), FIRST.with(|first| first.borrow().clone()));
            } else {
                assert_eq!(cause, NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation")));
            }
            OWNER.with(|owner| owner.borrow_mut().take());
        }
    }

    #[test]
    fn original_jim_import_follows_original_alias_lists_and_refuses_destination_loops() {

        for closes_loop in [false, true] {
            let mut interp = native();
            let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
                OWNER.with(|owner| owner.borrow_mut().take());
            });
            let destination_owner = destination(&mut interp);
            let destination = destination_owner.token;
            let destination_slot = destination_slot(&interp, destination);
            let end = Owned::fresh(obj::new_string_bytes(if closes_loop {
                b"dest::branch"
            } else {
                b"list"
            }));
            let middle = define(&mut interp, b"chain", &[end.as_ptr()]);
            let chain = Owned::fresh(obj::new_string_bytes(b"chain"));
            let source = define(&mut interp, b"src::branch", &[chain.as_ptr()]);
            assert_eq!(
                import(&mut interp, destination, b"::src::branch"),
                if closes_loop { Code::Error } else { Code::Ok }
            );
            assert_eq!(
                interp
                    .namespaces()
                    .command_in(GLOBAL, &destination_slot)
                    .is_some(),
                !closes_loop
            );
            if closes_loop {
                assert_eq!(
                    interp.result_bytes(),
                    b"import pattern \"::src::branch\" would create a loop"
                );
            }
            assert!(interp.native_execution_refusal().is_none());
            assert!(obj::has_string_rep(source.as_ptr()));
            assert!(obj::has_string_rep(middle.as_ptr()));
            OWNER.with(|owner| owner.borrow_mut().take());
        }
    }

    #[test]
    fn original_jim_import_keeps_root_alias_lookup_separate_from_destination_collisions() {

        let mut interp = native();
        let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
            OWNER.with(|owner| owner.borrow_mut().take());
        });
        assert_eq!(interp.eval_str(b"namespace eval dest::src {}"), Code::Ok);
        let destination_owner = destination(&mut interp);
        let destination = destination_owner.token;
        let destination_slot = destination_slot(&interp, destination);
        let list = Owned::fresh(obj::new_string_bytes(b"list"));
        let source = define(&mut interp, b"src::branch", &[list.as_ptr()]);
        let loop_destination = Owned::fresh(obj::new_string_bytes(b"dest::branch"));
        let collision = define(
            &mut interp,
            b"dest::src::branch",
            &[loop_destination.as_ptr()],
        );
        assert_eq!(import(&mut interp, destination, b"::src::branch"), Code::Ok);
        assert!(interp
            .namespaces()
            .command_in(GLOBAL, &destination_slot)
            .is_some());
        assert!(obj::has_string_rep(source.as_ptr()));
        assert!(!obj::has_string_rep(collision.as_ptr()));
        assert!(interp.native_execution_refusal().is_none());
        let name = Owned::fresh(obj::new_string_bytes(b"src::branch"));
        assert!(tcl_runtime_api::Aliases::alias_prefix_original_value_at(
            &mut interp,
            tcl_runtime_api::NsId(u32::MAX),
            &name.as_ptr()
        )
        .is_err());
        assert_eq!(interp.current_ns(), destination);
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}

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
mod original_namespace_continuation_tests {
    use super::*;
    use crate::obj::Owned;
    use std::cell::{Cell, RefCell};

    thread_local! {
        static ACTUAL: RefCell<Option<Interp>> = const { RefCell::new(None) };
        static PAYLOADS: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
        static EVENTS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
        static ACTION: Cell<u8> = const { Cell::new(0) };
        static AT: Cell<usize> = const { Cell::new(0) };
        static TARGET: Cell<NsId> = const { Cell::new(GLOBAL) };
        static PARENT: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
        static FIRST: RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { RefCell::new(None) };
    }

    fn callback() {
        let mut actual = ACTUAL.with(|slot| slot.borrow().as_ref().unwrap().clone());
        match ACTION.with(Cell::get) {
            0 => {}
            1 => {
                actual.refuse_host_command("original namespace continuation first cause");
                FIRST.with(|slot| *slot.borrow_mut() = actual.native_execution_refusal());
            }
            2 => {
                let original = actual.runtime_context();
                let mut changed = original.clone();
                changed.packages = vec![("namespace-continuation-control".into(), "1.0".into())];
                actual.pin_context(&changed).unwrap();
                actual.pin_context(&original).unwrap();
            }
            3 => actual.set_current_ns(TARGET.with(Cell::get)),
            4 => {
                let old = TARGET.with(Cell::get);
                actual.delete_namespace_by_id(old);
                let replacement = actual.ensure_global_namespace(b"::tree");
                assert_ne!(replacement, old);
            }
            5 => obj::change_type(PARENT.with(Cell::get), std::ptr::null(), 0),
            6 => {
                let namespace = actual.current_ns();
                actual.frames.borrow_mut().push(namespace);
            }
            _ => unreachable!(),
        }
    }

    extern "C" fn update(original: *mut TclObj) {
        let index = usize::try_from(obj::internal_rep(original)).unwrap();
        EVENTS.with(|events| events.borrow_mut().push(index));
        let bytes = PAYLOADS.with(|payloads| payloads.borrow()[index].clone());
        unsafe { obj::set_native_updater_string_rep(original, &bytes, false) };
        if AT.with(Cell::get) == index {
            callback();
        }
    }
    extern "C" fn free(_: *mut TclObj) {
        EVENTS.with(|events| events.borrow_mut().push(99));
        callback();
    }
    static LAZY: obj::TclObjType = obj::TclObjType {
        name: c"originalNamespaceContinuation".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    static RETIRED: obj::TclObjType = obj::TclObjType {
        name: c"originalNamespaceHandlerRetirement".as_ptr(),
        free_int_rep_proc: Some(free),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };

    struct Cleanup;
    impl Drop for Cleanup {
        fn drop(&mut self) {
            ACTION.with(|action| action.set(0));
            ACTUAL.with(|slot| slot.borrow_mut().take());
            PARENT.with(|parent| parent.set(std::ptr::null_mut()));
        }
    }
    fn actual(engine: &str) -> (Interp, Cleanup, NsId, NsId) {
        ACTUAL.with(|slot| slot.borrow_mut().take());
        PAYLOADS.with(|values| values.borrow_mut().clear());
        EVENTS.with(|values| values.borrow_mut().clear());
        ACTION.with(|value| value.set(0));
        AT.with(|value| value.set(0));
        FIRST.with(|value| value.borrow_mut().take());
        let mut interp = original_argument_tests::instance(engine);
        assert_eq!(interp.eval_str(b"namespace eval tree {namespace eval child {}; variable value VALUE}; namespace eval dest {}"), Code::Ok);
        let tree = interp.ensure_global_namespace(b"::tree");
        let dest = interp.ensure_global_namespace(b"::dest");
        interp.set_current_ns(dest);
        ACTUAL.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
        (interp, Cleanup, tree, dest)
    }
    fn lazy(bytes: &[u8]) -> Owned {
        obj::register_legacy_extension_type(&LAZY);
        let index = PAYLOADS.with(|payloads| {
            let mut payloads = payloads.borrow_mut();
            let index = payloads.len();
            payloads.push(bytes.to_vec());
            index
        });
        Owned::fresh(obj::alloc_typed(&LAZY, u64::try_from(index).unwrap()))
    }
    fn text(bytes: &[u8]) -> Owned {
        Owned::fresh(obj::new_string_bytes(bytes))
    }
    fn call(actual: &mut Interp, subcommand: &[u8], operands: &[&Owned]) -> Code {
        let head = text(b"namespace");
        let member = text(subcommand);
        let mut argv = vec![head.as_ptr(), member.as_ptr()];
        argv.extend(operands.iter().map(|word| word.as_ptr()));
        let code = namespace_cmd(actual, &argv);
        assert_eq!(actual.entered_native_operation_count(), 0);
        code
    }
    fn events() -> Vec<usize> {
        EVENTS.with(|events| events.borrow().clone())
    }
    fn action(mode: u8, at: usize, target: NsId) {
        ACTION.with(|action| action.set(mode));
        AT.with(|selected| selected.set(at));
        TARGET.with(|slot| slot.set(target));
    }

    #[test]
    fn original_namespace_children_path_unknown_upvar_keep_successful_getter_order_and_objects() {
        // Software extension controls preserve the existing native success paths.
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let (mut actual, _cleanup, tree, dest) = actual(engine);
            let target = lazy(b"::tree");
            let pattern = lazy(b"*");
            assert_eq!(
                call(&mut actual, b"children", &[&target, &pattern]),
                Code::Ok,
                "{engine}"
            );
            assert_eq!(actual.result_bytes(), b"::tree::child", "{engine}");
            assert_eq!(events(), vec![0, 1]);

            if engine == "tcl8.4" {
                continue;
            }
            let first = lazy(b"::tree");
            let second = lazy(b"::dest");
            let path = Owned::fresh(actual.new_list_object(&[first.as_ptr(), second.as_ptr()]));
            assert_eq!(call(&mut actual, b"path", &[&path]), Code::Ok, "{engine}");
            assert_eq!(actual.namespaces().path(dest), &[tree, dest]);
            assert_eq!(events(), vec![0, 1, 2, 3]);
            assert_eq!(call(&mut actual, b"path", &[]), Code::Ok);
            assert_eq!(actual.result_bytes(), b"::tree ::dest");

            let handler = lazy(b"::handler retained\0counted");
            assert_eq!(call(&mut actual, b"unknown", &[&handler]), Code::Ok);
            assert_eq!(actual.get_obj_result(), handler.as_ptr());
            assert_eq!(call(&mut actual, b"unknown", &[]), Code::Ok);
            assert_eq!(actual.get_obj_result(), handler.as_ptr());
            assert_eq!(
                obj::bytes_of(handler.as_ptr()),
                b"::handler retained\0counted"
            );

            let namespace = lazy(b"::tree");
            let variable = lazy(b"value");
            let local = lazy(b"linked\0discarded");
            let start = events().len();
            let first_index = PAYLOADS.with(|payloads| payloads.borrow().len()) - 3;
            assert_eq!(
                call(&mut actual, b"upvar", &[&namespace, &variable, &local]),
                Code::Ok,
                "{engine}"
            );
            assert_eq!(
                &events()[start..],
                &[first_index, first_index + 2, first_index + 1]
            );
            assert_eq!(
                actual.eval_str(b"set linked CHANGED; set ::tree::value"),
                Code::Ok
            );
            assert_eq!(actual.result_bytes(), b"CHANGED");
            assert_eq!(obj::bytes_of(local.as_ptr()), b"linked\0discarded");
            assert!(actual.native_execution_refusal().is_none());
        }
    }

    #[test]
    fn original_namespace_children_keeps_selected_target_birth_across_pattern_callback() {
        let (mut actual, _cleanup, tree, _) = actual("tcl8.6");
        let target = text(b"::tree");
        let pattern = lazy(b"*");
        actual.set_result_bytes(b"PRIOR");
        let prior = actual.get_obj_result();
        action(4, 0, tree);
        assert_eq!(
            call(&mut actual, b"children", &[&target, &pattern]),
            Code::Error
        );
        assert_eq!(events(), vec![0]);
        assert_eq!(actual.get_obj_result(), prior);
        assert!(actual.native_execution_refusal().is_some());
        assert_ne!(
            actual.namespaces().find_namespace(GLOBAL, b"::tree"),
            Some(tree)
        );
    }

    #[test]
    fn original_namespace_path_checks_prior_targets_and_original_member_backing_before_commit() {
        for mode in [1, 2, 3, 4, 5] {
            let (mut actual, _cleanup, tree, dest) = actual("tcl9.1");
            actual.namespaces_mut().set_path(dest, vec![dest]);
            let first = lazy(b"::tree");
            let second = lazy(b"::dest");
            let later = lazy(b"::tree");
            let path = Owned::fresh(actual.new_list_object(&[
                first.as_ptr(),
                second.as_ptr(),
                later.as_ptr(),
            ]));
            PARENT.with(|parent| parent.set(path.as_ptr()));
            actual.set_result_bytes(b"PRIOR");
            let prior = actual.get_obj_result();
            action(mode, 1, tree);
            assert_eq!(call(&mut actual, b"path", &[&path]), Code::Error, "{mode}");
            assert_eq!(events(), vec![0, 1], "{mode}");
            assert!(!obj::has_string_rep(later.as_ptr()));
            assert_eq!(actual.namespaces().path(dest), &[dest]);
            assert_eq!(actual.get_obj_result(), prior);
            let refusal = actual.native_execution_refusal().unwrap();
            if mode == 1 {
                assert_eq!(FIRST.with(|first| first.borrow().clone()), Some(refusal));
            }
        }
    }

    #[test]
    fn original_namespace_unknown_validates_original_list_and_frame_before_replacing_handler() {
        for mode in [1, 2, 3, 6] {
            let (mut actual, _cleanup, tree, dest) = actual("tcl8.6");
            let old = text(b"::old handler");
            assert_eq!(call(&mut actual, b"unknown", &[&old]), Code::Ok);
            let replacement = lazy(b"::new handler");
            actual.set_result_bytes(b"PRIOR");
            let prior = actual.get_obj_result();
            action(mode, 0, tree);
            assert_eq!(call(&mut actual, b"unknown", &[&replacement]), Code::Error);
            assert_eq!(events(), vec![0]);
            assert_eq!(
                actual.namespace_unknown_root(dest, false).unwrap().as_ptr(),
                old.as_ptr()
            );
            assert_eq!(actual.get_obj_result(), prior);
            assert!(actual.native_execution_refusal().is_some());
            if mode == 6 {
                actual.frames.borrow_mut().pop();
            }
        }
    }

    #[test]
    fn original_namespace_unknown_checks_retired_handler_callback_before_new_publication() {
        for mode in [1, 2, 3, 6] {
            let (mut actual, _cleanup, tree, dest) = actual("tcl8.6");
            let retired = Owned::fresh(obj::alloc_typed(&RETIRED, 0));
            let old = Owned::fresh(actual.new_list_object(&[retired.as_ptr()]));
            drop(retired);
            assert!(actual
                .namespaces_mut()
                .set_unknown_handler(dest, Some(old))
                .is_none());
            let replacement = text(b"::new handler");
            actual.set_result_bytes(b"PRIOR");
            let prior = actual.get_obj_result();
            action(mode, 99, tree);
            assert_eq!(call(&mut actual, b"unknown", &[&replacement]), Code::Error);
            assert_eq!(events(), vec![99]);
            assert!(actual.namespace_unknown_root(dest, false).is_none());
            assert_eq!(actual.get_obj_result(), prior);
            assert!(actual.native_execution_refusal().is_some());
            if mode == 1 {
                assert_eq!(
                    FIRST.with(|first| first.borrow().clone()),
                    actual.native_execution_refusal()
                );
            }
            if mode == 6 {
                actual.frames.borrow_mut().pop();
            }
        }
    }

    #[test]
    fn original_namespace_upvar_checks_namespace_before_target_getter_and_alias_publication() {
        for mode in [1, 2, 3, 4, 6] {
            let (mut actual, _cleanup, tree, dest) = actual("tcl8.6");
            let namespace = text(b"::tree");
            let target = lazy(b"value");
            let local = lazy(b"linked");
            actual.set_result_bytes(b"PRIOR");
            let prior = actual.get_obj_result();
            action(mode, 1, tree);
            assert_eq!(
                call(&mut actual, b"upvar", &[&namespace, &target, &local]),
                Code::Error,
                "{mode}"
            );
            assert_eq!(events(), vec![1], "{mode}");
            assert!(!obj::has_string_rep(target.as_ptr()));
            assert_eq!(actual.get_obj_result(), prior);
            assert!(actual
                .namespaces()
                .var_table(dest)
                .cell(b"linked")
                .is_none());
            assert!(actual.native_execution_refusal().is_some());
            if mode == 6 {
                actual.frames.borrow_mut().pop();
            }
        }
    }

    #[test]
    fn original_namespace_upvar_transports_outer_namespace_check_through_original_target_conversion(
    ) {
        let (mut actual, _cleanup, tree, dest) = actual("tcl8.6");
        let namespace = text(b"::tree");
        let target = lazy(b"value");
        let local = text(b"linked");
        actual.set_result_bytes(b"PRIOR");
        let prior = actual.get_obj_result();
        action(3, 0, tree);
        assert_eq!(
            call(&mut actual, b"upvar", &[&namespace, &target, &local]),
            Code::Error
        );
        assert_eq!(events(), vec![0]);
        assert_eq!(actual.get_obj_result(), prior);
        assert!(actual
            .namespaces()
            .var_table(dest)
            .cell(b"linked")
            .is_none());
        assert!(actual.native_execution_refusal().is_some());
    }
}
