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

//! `global` / `variable` / `upvar` — the variable-namespace side.
//!
//! All three install variable [`Link`](crate::frame::Link)s through the one
//! variable resolver ([`crate::vars`]) — the variable parallel of `rename`/
//! `interp alias` on the command side:
//!
//! - `global name…` links each `name`'s tail to the global var of that name
//!   (qualified names resolve in the global context, so `global ::a::x` links
//!   `x` → `::a::x`).
//! - `variable name ?value?…` links each tail to the *current* namespace's var
//!   (or the named one if qualified), optionally initialising it.
//! - `upvar ?level? otherVar localVar …` links `localVar` to `otherVar` in a
//!   caller frame (`#N` absolute / `N` relative) or, when `otherVar` is
//!   namespace-qualified, to that namespace var.
//!
//! In C Tcl, `global` has no effect outside a procedure activation; namespace
//! evaluation still uses its ordinary dialect-specific variable lookup.
//! `variable` and `upvar` can install namespace links in those activations.
//! Jim selects links by name in its activation-local variable table. See `tclVar.c`
//! (`Tcl_GlobalObjCmd` / `Tcl_VariableObjCmd` / `Tcl_UpvarObjCmd`) and
//! `namespace-tree.md` §5.3 for the modelled semantics.

use crate::interp::{Code, Interp};
use crate::obj::TclObj;
use tcl_syntax::value::ValueOps;

/// Register `global`, `variable`, and `upvar`; also re-registers `set` (and,
/// where the numeric tower is linked, `incr`) to fix their return value after
/// a write trace runs — see [`set_cmd`] and [`installed_incr`].
/// The override pattern mirrors TclOO's own `variable` override in
/// `builtins::install` (installed later still wins; nothing registered
/// after this module re-registers `set`/`incr`).
pub fn install(interp: &mut Interp) {
    interp.register_stock_builtin(b"global", global);
    interp.register_stock_builtin(b"variable", variable);
    interp.register_stock_builtin(b"upvar", upvar);
    interp.register_stock_builtin(b"set", set_cmd);
    interp.register_stock_builtin(b"incr", installed_incr());
}

// set / incr: return-after-trace
//
// C's `TclPtrSetVarIdx` (tclVar.c 9.0.4:2050-2065) stores the value, fires the
// write traces, and only *then* decides what to return: the cell's *current*
// value if it is still a defined scalar (a trace may have rewritten it), or an
// empty-string object if a trace changed the variable "in some gross way" —
// unset it, or turned it into an array. `set`/`incr` in `builtins.rs` instead
// echoed back the value they had just stored, which is also a use-after-free
// once a write trace replaces that same cell (the stored object's only
// reference is dropped from under it) — `store_var_result` (`interp.rs`,
// already used by `append`/`lappend`/`string insert` for the identical
// reason) closes both bugs at once by holding a protective reference across
// the store and reading the cell back afterward.
//
// Pinned against `tclsh9.0`/`tclsh8.6`:
//   proc w {n1 n2 op} {set ::x mangled}; trace add variable x write w
//   set x orig                                -> mangled
//   proc u {n1 n2 op} {unset ::x}; trace add variable y write u
//   set y orig                                -> "" (the trace unset it)
//   proc w {n1 n2 op} {set ::z mangled}; trace add variable z write w
//   incr z                                    -> mangled

/// `set varName ?newValue?` — overrides `builtins::set`'s write arm only; the
/// read arm is unchanged.
fn set_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        2 => match interp.read_original_named_variable(argv[1]) {
            Ok(value) => {
                interp.set_result(value);
                Code::Ok
            }
            Err(code) => code,
        },
        3 => match interp.store_original_named_variable(argv[1], argv[2]) {
            Ok(()) => Code::Ok,
            Err(code) => code,
        },
        _ => interp.wrong_args_for_invocation(argv, b"varName ?newValue?"),
    }
}

/// The installed integer increment shares the physical receiver protocol with
/// direct runtime and compiled-ABI calls, including trace-safe result retention.
pub(crate) fn installed_incr() -> fn(&mut Interp, &[*mut TclObj]) -> Code {
    crate::builtins::incr
}

/// `can't <verb> "<name>": parent namespace doesn't exist` — the qualified-into-
/// a-missing-namespace error (verb is `define` for `variable`, `access` for
/// `global`/`upvar`).
fn no_namespace(interp: &mut Interp, verb: &[u8], name: &[u8]) -> Code {
    let mut m = b"can't ".to_vec();
    m.extend_from_slice(verb);
    m.extend_from_slice(b" \"");
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": parent namespace doesn't exist");
    let mut error_code = b"TCL LOOKUP VARNAME ".to_vec();
    error_code.extend_from_slice(name);
    interp.error_with_code(&m, &error_code)
}

fn inverted_upvar(interp: &mut Interp, local: &[u8]) -> Code {
    let mut message = b"bad variable name \"".to_vec();
    message.extend_from_slice(local);
    message.extend_from_slice(
        b"\": can't create namespace variable that refers to procedure variable",
    );
    interp.error_with_code(&message, b"TCL UPVAR INVERTED")
}

// global

/// `global varName ?varName ...?` — link each name's tail to the global of that
/// name (resolved in the global namespace context).
fn global(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let frame = if interp.frames.borrow().current_level() == 0 {
        tcl_registry::VariableAliasFrame::Global
    } else if interp.in_proc() {
        tcl_registry::VariableAliasFrame::Procedure
    } else {
        tcl_registry::VariableAliasFrame::Namespace
    };
    if tcl_registry::VariableAliasDestination::ProcedureLocal
        .is_active_in_frame(frame, interp.native_invocation_dialect())
        == Some(false)
    {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    let protocol = match interp.require_variable_name_protocol() {
        Ok(protocol) => protocol,
        Err(error) => return crate::builtins::var_error(interp, b"", error),
    };
    if protocol.is_jim084() {
        if interp.current_level() == 0 {
            return Code::Ok;
        }
        let arguments =
            match crate::interp::native_arguments::OriginalArguments::enter(interp, argv) {
                Ok(arguments) => arguments,
                Err(error) => return interp.report_cmd_error(error),
            };
        for (index, &argument) in argv.iter().enumerate().skip(1) {
            let name = match arguments.bytes(index) {
                Ok(name) => name,
                Err(error) => return interp.report_cmd_error(error),
            };
            if protocol.variable_root_input(&name).qualification()
                == tcl_syntax::naming::NativeNameQualification::Absolute
            {
                continue;
            }
            let code = interp.link_original_jim_upvar_objects(argument, 0, argument);
            if code != Code::Ok {
                return arguments.finish(code);
            }
        }
        let code = arguments.finish(Code::Ok);
        if code == Code::Ok {
            interp.set_result_bytes(b"");
        }
        return arguments.finish(code);
    }
    // `global` with no names is a no-op (TIP 323).
    for &a in &argv[1..] {
        let name = match interp.native_string_bytes(&a) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let Some(local) = tcl_syntax::naming::global_local_name_bytes(protocol, &name) else {
            continue;
        };
        let qualified = protocol.variable_alias_local_input(&name).qualification()
            != tcl_syntax::naming::NativeNameQualification::Unqualified;
        let tail =
            qualified.then(|| crate::obj::Owned::fresh(crate::obj::new_string_bytes(&local)));
        let local_original = tail.as_ref().map_or(a, crate::obj::Owned::as_ptr);
        let code = bind_upvar_at_original(interp, 0, &name, &local, Some(a), Some(local_original));
        if code != Code::Ok {
            return code;
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// variable

/// `variable ?name value ...? name ?value?` — declare/link namespace variables,
/// initialising those given a value. The trailing name may omit its value.
pub(crate) fn variable(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_none()
    {
        return variable_entered(interp, argv, None);
    }
    let arguments = match crate::interp::native_arguments::OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let reached = variable_entered(interp, argv, Some(&arguments));
    arguments.finish(reached)
}

fn variable_entered(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: Option<&crate::interp::native_arguments::OriginalArguments>,
) -> Code {
    let Some(grammar) = interp
        .native_invocation_dialect()
        .native_variable_declaration_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "namespace variable declaration grammar",
            )
            .into(),
        );
    };
    if let Err(usage) = grammar.arguments(argv.len().saturating_sub(1)) {
        return interp.wrong_args_for_invocation(argv, usage);
    }
    if interp.native_c_variable_name_protocol().is_some() {
        for pair in argv[1..].chunks(2) {
            let code = interp.define_original_c_namespace_variable(pair[0], pair.get(1).copied());
            if code != Code::Ok {
                return code;
            }
        }
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    if let Some(arguments) = arguments {
        if interp.current_ns() == crate::namespace::GLOBAL {
            if let Err(error) = interp.native_jim_object_context() {
                return interp.report_cmd_error(error.into());
            }
        }
        let namespace =
            match crate::interp::OriginalJimLookupNamespace::capture(interp, interp.current_ns()) {
                Ok(namespace) => namespace,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
        let reached = (|| {
            for (pair_index, pair) in argv[1..].chunks(2).enumerate() {
                let ordinal = 1 + pair_index * 2;
                let check = |actual: &Interp| {
                    namespace.check_current_arguments(actual, arguments)?;
                    arguments.check_original_for(actual, ordinal, pair[0])
                };
                let canonical = match interp.jim_canonical_namespace_object_using(
                    namespace.original_ptr(),
                    pair[0],
                    &check,
                ) {
                    Ok(canonical) => canonical,
                    Err(error) => return interp.report_cmd_error(error),
                };
                if let Err(error) = check(interp) {
                    return interp.report_cmd_error(error);
                }
                let local = match interp
                    .original_jim_namespace_text_object(pair[0], true, ordinal, arguments)
                {
                    Ok(local) => crate::obj::Owned::retain(local),
                    Err(error) => return interp.report_cmd_error(error),
                };
                if interp.current_level() != 0 || interp.current_ns() != crate::namespace::GLOBAL {
                    let scope =
                        match crate::interp::native_operation_currency::NativeOperationScope::enter(
                            interp,
                        ) {
                            Ok(scope) => scope,
                            Err(first) => return interp.refuse_native_execution(first),
                        };
                    if let Err(error) = arguments.check() {
                        return interp.report_cmd_error(error);
                    }
                    let target = match canonical.checked_ptr() {
                        Ok(target) => target,
                        Err(error) => return interp.report_cmd_error(error.into()),
                    };
                    let local_temporary = crate::obj::Owned::retain(local.as_ptr());
                    let target = crate::obj::Owned::retain(target);
                    let code = interp.link_original_jim_upvar_objects(
                        target.as_ptr(),
                        0,
                        local_temporary.as_ptr(),
                    );
                    if let Err(error) = check(interp) {
                        return interp.report_cmd_error(error);
                    }
                    let code = if code == Code::Error {
                        let Some(protocol) = interp
                            .native_invocation_dialect()
                            .native_jim_lookup_protocol()
                        else {
                            return interp.report_cmd_error(
                                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                    "Jim namespace variable failure issuer",
                                )
                                .into(),
                            );
                        };
                        match interp.publish_original_jim_variable_error(
                            local_temporary.as_ptr(),
                            &check,
                            |bytes| {
                                tcl_cmd_core::var::native_jim_namespace_variable_error(
                                    protocol, bytes,
                                )
                            },
                        ) {
                            Ok(code) => code,
                            Err(error) => return interp.report_cmd_error(error),
                        }
                    } else {
                        code
                    };
                    drop(local_temporary);
                    drop(target);
                    if let Err(first) = scope.currency().ensure_current_or_refuse() {
                        return interp.refuse_native_execution(first);
                    }
                    if code != Code::Ok {
                        return code;
                    }
                }
                if let Some(&value) = pair.get(1) {
                    if let Err(code) = interp.assign_original_named_variable(local.as_ptr(), value)
                    {
                        return code;
                    }
                }
                drop(local);
                drop(canonical);
                if let Err(error) = check(interp) {
                    return interp.report_cmd_error(error);
                }
            }
            Code::Ok
        })();
        if let Err(error) = namespace.check_current_arguments(interp, arguments) {
            return interp.report_cmd_error(error);
        }
        if reached != Code::Ok {
            return reached;
        }
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    let protocol = match interp.require_variable_name_protocol() {
        Ok(protocol) => protocol,
        Err(error) => return crate::builtins::var_error(interp, b"", error),
    };
    let current = interp.current_ns();
    let mut i = 1;
    while i < argv.len() {
        let name = match interp.native_string_bytes(&argv[i]) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let Some((ns, tail)) = interp.resolve_var_target(current, &name) else {
            return no_namespace(interp, b"define", &name);
        };
        // A `variable` may not name an array element (C's `TclObjLookupVarEx`
        // rejects an `arr(elem)` target).
        if protocol.combined_variable_input(&tail).element().is_some() {
            let mut m = b"can't define \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b"\": name refers to an element in an array");
            return interp.set_error(&m);
        }
        // Install the link first; a following `var_set(tail, …)` then writes
        // *through* it into the target namespace (the link makes the unqualified
        // tail resolve there, in a proc or at namespace scope alike).
        let local = tcl_syntax::naming::variable_local_name_bytes(protocol, &name);
        let code = interp.make_variable_mapped(ns, &local, &tail);
        if code != Code::Ok {
            return code;
        }
        if i + 1 < argv.len() {
            let value = argv[i + 1];
            if let Err(e) = interp.var_set(&local, value) {
                return crate::builtins::var_error(interp, &name, e);
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// upvar

/// `upvar ?level? otherVar localVar ?otherVar localVar ...?` — link each
/// `localVar` in the current frame to `otherVar`. The optional level is `#N`
/// (absolute) or `N` (relative, default `1`); a namespace-qualified `otherVar`
/// links to that namespace var regardless of level.
fn upvar(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let usage = interp.native_invocation_dialect().native_namespace_upvar_protocol()
        .and_then(tcl_native_model::native_namespace_upvar::NativeNamespaceUpvarProtocol::forwarded_wrong_arguments_usage)
        .unwrap_or(b"upvar ?level? otherVar localVar ?otherVar localVar ...?");
    if argv.len() < 3 {
        return interp.wrong_args(usage);
    }
    let (width, target_level) = match crate::cmd_eval::select_frame(
        interp,
        &argv[1..],
        tcl_registry::FrameEffectSpec::UPVAR,
    ) {
        Ok(selected) => selected,
        Err(code) => {
            #[cfg(test)]
            crate::env_trace::remember_upvar_failure("frame-selection", code, None);
            return code;
        }
    };
    let pairs_start = 1 + width;
    if argv.len() - pairs_start < 2 || (argv.len() - pairs_start) % 2 != 0 {
        return interp.wrong_args(usage);
    }
    let mut i = pairs_start;
    while i + 1 < argv.len() {
        if interp.native_c_variable_name_protocol().is_some() {
            let code = interp.link_original_c_upvar_objects(argv[i], target_level, argv[i + 1]);
            if code != Code::Ok {
                return code;
            }
            i += 2;
            continue;
        }
        if interp
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .is_some()
        {
            let code = interp.link_original_jim_upvar_objects(argv[i], target_level, argv[i + 1]);
            if code != Code::Ok {
                return code;
            }
            i += 2;
            continue;
        }
        let other = match interp.native_string_bytes(&argv[i]) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let local = match interp.native_string_bytes(&argv[i + 1]) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let code = bind_upvar_at_original(
            interp,
            target_level,
            &other,
            &local,
            Some(argv[i]),
            Some(argv[i + 1]),
        );
        if code != Code::Ok {
            return code;
        }
        i += 2;
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Install one alias through the same physical resolver used by upvar and
/// reference-formal activation. No command lookup participates in binding.
pub(crate) fn bind_upvar_at(
    interp: &mut Interp,
    target_level: usize,
    other: &[u8],
    local: &[u8],
) -> Code {
    bind_upvar_at_original(interp, target_level, other, local, None, None)
}

fn bind_upvar_at_original(
    interp: &mut Interp,
    target_level: usize,
    other: &[u8],
    local: &[u8],
    original: Option<*mut TclObj>,
    local_original: Option<*mut TclObj>,
) -> Code {
    if interp
        .native_invocation_dialect()
        .native_jim_lookup_protocol()
        .is_some()
    {
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(interp) {
                Ok(scope) => scope,
                Err(first) => return interp.refuse_native_execution(first),
            };
        let fresh_target = original
            .is_none()
            .then(|| crate::obj::Owned::fresh(crate::obj::new_string_bytes(other)));
        let fresh_local = local_original
            .is_none()
            .then(|| crate::obj::Owned::fresh(crate::obj::new_string_bytes(local)));
        let target = original.unwrap_or_else(|| {
            fresh_target
                .as_ref()
                .expect("fresh compatibility target")
                .as_ptr()
        });
        let local = local_original.unwrap_or_else(|| {
            fresh_local
                .as_ref()
                .expect("fresh compatibility local")
                .as_ptr()
        });
        let reached = interp.link_original_jim_upvar_objects(target, target_level, local);
        drop(fresh_local);
        drop(fresh_target);
        return match scope.currency().ensure_current_or_refuse() {
            Ok(()) => reached,
            Err(first) => interp.refuse_native_execution(first),
        };
    }
    let protocol = match interp.require_variable_name_protocol() {
        Ok(protocol) => protocol,
        Err(error) => return crate::builtins::var_error(interp, local, error),
    };
    let selected_local = protocol.variable_root_input(local);
    let local_key = selected_local.selected();

    let fresh;
    let original = if let Some(original) = original {
        original
    } else {
        fresh = crate::obj::Owned::fresh(crate::obj::new_string_bytes(other));
        fresh.as_ptr()
    };
    if interp.native_c_variable_name_protocol().is_some() {
        let target = match interp.prepare_selected_c_link_target(original, target_level) {
            Ok(Some(target)) => target,
            Ok(None) => return no_namespace(interp, b"access", other),
            Err(code) => return code,
        };
        let fresh_local;
        let local_original = if let Some(original) = local_original {
            original
        } else {
            fresh_local = crate::obj::Owned::fresh(crate::obj::new_string_bytes(local));
            fresh_local.as_ptr()
        };
        return interp.bind_selected_c_alias_local(local_original, target);
    }
    let prepared = None;
    let target = match prepared {
        Some(target) => Some(target),
        None => {
            let (base, elem) = match interp.variable_name_parts(other) {
                Ok(parts) => parts,
                Err(error) => return crate::builtins::var_error(interp, other, error),
            };
            crate::vars::link_target_at(
                &interp.frames.borrow(),
                &interp.namespaces(),
                &base,
                elem,
                target_level,
            )
        }
    };
    let Some(mut link) = target else {
        return no_namespace(interp, b"access", other);
    };
    if let Err(error) = interp.prepare_upvar_target(&mut link) {
        let reason = match error {
            crate::frame::VarError::IsScalar => b"variable isn't array".as_slice(),
            crate::frame::VarError::DeletedArray => {
                b"upvar refers to element in deleted array".as_slice()
            }
            _ => b"no such variable".as_slice(),
        };
        let mut message = b"can't access \"".to_vec();
        message.extend_from_slice(other);
        message.extend_from_slice(b"\": ");
        message.extend_from_slice(reason);
        return interp.set_error(&message);
    }
    // C resolves the other-variable first, then rejects a namespace alias
    // to a procedure cell before inspecting the alias's element shape or
    // parent namespace (`TCL UPVAR INVERTED`).
    if interp.upvar_would_invert(&link, local) {
        return inverted_upvar(interp, local);
    }
    if protocol.combined_variable_input(local).element().is_some() {
        let mut m = b"bad variable name \"".to_vec();
        m.extend_from_slice(local);
        m.extend_from_slice(b"\": can't create a scalar variable that looks like an array element");
        return interp.error_with_code(&m, b"TCL UPVAR LOCAL_ELEMENT");
    }
    // A qualified local name (`ns::lnk`) creates a namespace link variable
    // rather than a frame local; its namespace must exist.
    if interp.variable_is_qualified(local) {
        match interp.resolve_var_target(interp.current_ns(), local) {
            Some((target_ns, tail)) => {
                let code = interp.make_upvar_in(target_ns, &tail, link);
                if code != Code::Ok {
                    return code;
                }
            }
            None => {
                let mut m = b"can't create \"".to_vec();
                m.extend_from_slice(local);
                m.extend_from_slice(b"\": parent namespace doesn't exist");
                let mut error_code = b"TCL LOOKUP VARNAME ".to_vec();
                error_code.extend_from_slice(local);
                return interp.error_with_code(&m, &error_code);
            }
        }
    } else {
        let code = interp.make_upvar(link, local_key);
        if code != Code::Ok {
            return code;
        }
    }
    Code::Ok
}

#[cfg(test)]
mod member_trace_horizon;

#[cfg(test)]
mod tests;
