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

use crate::interp::native_arguments::OriginalArguments;
#[cfg(test)]
use crate::interp::Command;
use crate::interp::{error_code_list, Code, Interp};
use crate::list;
use crate::namespace::{NsId, GLOBAL};
use crate::obj::{self, TclObj};
use tcl_syntax::value::ValueOps;

mod ensemble_subcommand;
mod import_export;

use ensemble_subcommand::ns_ensemble;
#[cfg(test)]
use ensemble_subcommand::{ens_configure, ens_create, join_words};
use import_export::{ns_export, ns_forget, ns_import};

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
