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
    interp.register_builtin(b"global", global);
    interp.register_builtin(b"variable", variable);
    interp.register_builtin(b"upvar", upvar);
    interp.register_builtin(b"set", set_cmd);
    interp.register_builtin(b"incr", installed_incr());
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
    let protocol = match interp.require_variable_name_protocol() {
        Ok(protocol) => protocol,
        Err(error) => return crate::builtins::var_error(interp, b"", error),
    };
    // `variable` with no names is a no-op (TIP 323).
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
        if protocol.is_jim084() {
            if interp.current_level() != 0 || current != crate::namespace::GLOBAL {
                let namespace = match interp.jim_current_namespace_object() {
                    Ok(namespace) => namespace,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                let canonical = match interp.jim_canonical_namespace_object(&namespace, argv[i]) {
                    Ok(canonical) => canonical,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                let target_bytes = match interp.native_string_bytes(&canonical.as_ptr()) {
                    Ok(bytes) => bytes,
                    Err(error) => return interp.report_cmd_error(error.into()),
                };
                let local_original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(&local));
                let code = bind_upvar_at_original(
                    interp,
                    0,
                    &target_bytes,
                    &local,
                    Some(canonical.as_ptr()),
                    Some(local_original.as_ptr()),
                );
                if code != Code::Ok {
                    return code;
                }
            }
        } else {
            interp.make_variable_mapped(ns, &local, &tail);
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
        .and_then(tcl_registry::native_namespace_upvar::NativeNamespaceUpvarProtocol::forwarded_wrong_arguments_usage)
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
        Err(code) => return code,
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
    let prepared = match interp.prepare_original_c_link_target(original, target_level) {
        Ok(prepared) => prepared,
        Err(code) => return code,
    };
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
    if let Err(error) = interp.retain_original_jim_link_target(&mut link, original, target_level) {
        return interp.report_cmd_error(error.into());
    }
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
    if interp.native_c_variable_name_protocol().is_some() {
        let fresh_local;
        let local_original = if let Some(original) = local_original {
            original
        } else {
            fresh_local = crate::obj::Owned::fresh(crate::obj::new_string_bytes(local));
            fresh_local.as_ptr()
        };
        return interp.bind_original_c_alias_local(local_original, link);
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
            Some((target_ns, tail)) => interp.make_upvar_in(target_ns, &tail, link),
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
        interp.make_upvar(link, local_key);
    }
    if let Some(original) = local_original {
        if let Err(error) = interp.retain_original_jim_link_local(original, local) {
            return interp.report_cmd_error(error.into());
        }
    }
    Code::Ok
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

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

    #[test]
    fn whole_array_destruction_preserves_staged_native_member_lifetimes() {
        use tcl_syntax::execution_conformance::{vectors, ExecutionDomain};
        let cases: Vec<_> = vectors(ExecutionDomain::CommandBinding)
            .into_iter()
            .filter(|case| case.id.starts_with("variable_array_unset_"))
            .collect();
        assert!(cases.len() >= 5);
        for reference in tcl_test_support::available_tclshs() {
            for case in &cases {
                let expected =
                    tcl_test_support::run_script(&reference.path, case.script().as_bytes())
                        .expect("native staged destruction")
                        .strict_text()
                        .expect("native result");
                let script = format!("list [catch {{{}}} value] $value", case.source);
                leak_free(|interp| {
                    interp.set_runtime_version(reference.version);
                    assert_eq!(
                        interp.eval_str(script.as_bytes()),
                        Code::Ok,
                        "host refusal: {:?}; case {}",
                        interp.native_access_refusal(),
                        case.id
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        expected.as_bytes(),
                        "{:?}: {}",
                        reference.version,
                        case.id
                    );
                });
            }
        }
    }

    #[test]
    fn captured_increment_receivers_match_actual_native_releases() {
        use tcl_syntax::execution_conformance::{vectors, ExecutionDomain};
        let cases: Vec<_> = vectors(ExecutionDomain::CommandBinding)
            .into_iter()
            .filter(|case| {
                case.id.starts_with("variable_rmw_")
                    || case.id == "variable_unset_recreated_root_survives_old_members"
            })
            .collect();
        assert!(cases.len() >= 8);
        for reference in tcl_test_support::available_tclshs() {
            for case in &cases {
                let expected =
                    tcl_test_support::run_script(&reference.path, case.script().as_bytes())
                        .expect("native captured receiver observation")
                        .strict_text()
                        .expect("native result");
                let script = format!("list [catch {{{}}} value] $value", case.source);
                leak_free(|interp| {
                    interp.set_runtime_version(reference.version);
                    assert_eq!(
                        interp.eval_str(script.as_bytes()),
                        Code::Ok,
                        "host refusal: {:?}; case {}",
                        interp.native_access_refusal(),
                        case.id
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        expected.as_bytes(),
                        "{:?}: {}",
                        reference.version,
                        case.id
                    );
                });
            }
        }
        if let Some(reference) = tcl_test_support::locate_jimsh().expect("Jim discovery") {
            let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
            for case in cases
                .iter()
                .filter(|case| case.jim_want != tcl_syntax::execution_conformance::UNSUPPORTED)
            {
                let expected =
                    tcl_test_support::run_script(&reference.path, case.script().as_bytes())
                        .expect("native Jim increment observation")
                        .strict_text()
                        .expect("native result");
                let script = format!("list [catch {{{}}} value] $value", case.source);
                leak_free(|interp| {
                    interp.set_dialect_profile(profile);
                    assert_eq!(
                        interp.eval_str(script.as_bytes()),
                        Code::Ok,
                        "host refusal: {:?}; case {}",
                        interp.native_access_refusal(),
                        case.id
                    );
                    assert_eq!(
                        interp.result_bytes(),
                        expected.as_bytes(),
                        "Jim: {}",
                        case.id
                    );
                });
            }
        }
    }

    #[test]
    fn legacy_upvar_level_presence_rejects_numeric_variable_pair() {
        for (dialect, expected) in [
            ("tcl8.4", b"1".as_slice()),
            ("tcl8.5", b"1".as_slice()),
            ("tcl8.6", b"0".as_slice()),
            ("tcl9.0", b"0".as_slice()),
            ("tcl9.1", b"0".as_slice()),
        ] {
            leak_free(|i| {
                i.set_dialect_profile(tcl_dialect::DialectProfile::find(dialect).expect("profile"));
                assert_eq!(i.eval_str(b"proc probe {} {upvar 1 local}; proc caller {} {set 1 3; catch probe}; caller"), Code::Ok);
                assert_eq!(i.result_bytes(), expected, "{dialect}");
            });
        }
    }

    #[test]
    fn upvar_qualified_scalar_and_element_use_selected_frame() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            leak_free(|i| {
                let expected = if dialect == "jim" {
                    let profile =
                        Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                            "jim",
                            &[],
                            "Jim",
                            tcl_dialect::model::DialectPoint::canonical(
                                tcl_dialect::model::Release::JIM_0_84,
                            ),
                        )));
                    i.set_dialect_profile(profile);
                    b"11 12 4 5".as_slice()
                } else {
                    i.set_dialect_profile(
                        tcl_dialect::DialectProfile::find(dialect).expect("profile"),
                    );
                    b"11 12 11 12".as_slice()
                };
                assert_eq!(i.eval_str(br#"namespace eval N {namespace eval R {variable x 4; variable a; set a(k) 5}; proc outer {} {inner::scalar; inner::array; list $R::x $R::a(k) $::N::R::x $::N::R::a(k)}; namespace eval inner {namespace eval R {variable x 99; variable a; set a(k) 98}; proc scalar {} {upvar 1 R::x alias; set alias 11}; proc array {} {upvar 1 R::a(k) alias; set alias 12}}}; N::outer"#), Code::Ok);
                assert_eq!(i.result_bytes(), expected, "{dialect}");
                assert_eq!(i.eval_str(b"proc global_alias {} {upvar #0 ::N::R::a(k) alias; incr alias}; global_alias"), Code::Ok, "{dialect}: host refusal {:?}", i.native_access_refusal());
                assert_eq!(
                    i.result_bytes(),
                    if dialect == "jim" {
                        b"6".as_slice()
                    } else {
                        b"13".as_slice()
                    }
                );
            });
        }
    }

    #[test]
    fn global_namespace_destinations_follow_real_engine_activation_rules() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            leak_free(|i| {
                i.set_dialect_profile(
                    tcl_registry::model::ingress::resolve_environment(dialect).unit_profile(),
                );
                assert_eq!(i.eval_str(b"set x ROOT; namespace eval N {global x; set x INSIDE}; list $::x [info exists ::N::x]"), Code::Ok);
                assert_eq!(
                    i.result_bytes(),
                    if dialect.starts_with("tcl9") {
                        b"ROOT 1".as_slice()
                    } else {
                        b"INSIDE 0".as_slice()
                    },
                    "{dialect}"
                );
                assert_eq!(i.eval_str(b"namespace eval Target {variable y START}; namespace eval N {upvar #0 ::Target::y link; set link CHANGED}; list $::Target::y [info exists ::N::link]"), Code::Ok);
                assert_eq!(
                    i.result_bytes(),
                    if dialect == "jim" {
                        b"CHANGED 0".as_slice()
                    } else {
                        b"CHANGED 1".as_slice()
                    },
                    "{dialect}"
                );
                assert_eq!(i.eval_str(b"namespace eval R {variable fresh QUALIFIED}; global R::fresh; info exists fresh"), Code::Ok);
                assert_eq!(i.result_bytes(), b"0", "{dialect}");
            });
        }
    }

    #[test]
    fn alias_cell_lifetimes_follow_real_c_and_jim_policies() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            leak_free(|i| {
                let profile = if dialect == "jim" {
                    Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                        "jim",
                        &[],
                        "Jim",
                        tcl_dialect::model::DialectPoint::canonical(
                            tcl_dialect::model::Release::JIM_0_84,
                        ),
                    )))
                } else {
                    tcl_dialect::DialectProfile::find(dialect).expect("profile")
                };
                i.set_dialect_profile(profile);
                for (script, expected) in [
                    (b"proc lifetime {} {set x OLD; upvar 0 x y; unset x; set y NEW; list $x $y}; lifetime".as_slice(), b"NEW NEW".as_slice()),
                    (b"proc lifetime {} {set x OLD; upvar 0 x y; unset x; set x NEW; list $x $y}; lifetime", b"NEW NEW"),
                    (b"proc lifetime {} {set results {}; upvar 0 x y; foreach n {1 2 3} {set x $n; lappend results $y; unset x}; set results}; lifetime", b"1 2 3"),
                    (b"proc lifetime {} {set a(k) OLD; upvar 0 a(k) y; unset a; set a(k) NEW; list [catch {set y} error] $error $a(k)}; lifetime", if dialect == "jim" { b"0 NEW NEW".as_slice() } else { b"1 {can't read \"y\": no such variable} NEW".as_slice() }),
                    (b"proc lifetime {} {set a(k) OLD; upvar 0 a(k) y; unset a(k); set a(k) NEW; list $y $a(k)}; lifetime", b"NEW NEW"),
                    (b"proc lifetime {} {upvar 0 missing(k) y; list [array exists missing] [info exists y]}; lifetime", if dialect == "jim" { b"0 0".as_slice() } else { b"1 0".as_slice() }),
                    (b"proc lifetime {} {set a SCALAR; list [catch {upvar 0 a(k) y} msg] $msg [info exists y]}; lifetime", if dialect == "jim" { b"0 {} 0".as_slice() } else { b"1 {can't access \"a(k)\": variable isn't array} 0".as_slice() }),
                    (b"proc lifetime {} {namespace eval ::life {variable x OLD}; upvar #0 ::life::x y; namespace delete ::life; namespace eval ::life {variable x NEW}; list [catch {set y} error] $error $::life::x}; lifetime", if dialect == "jim" { b"0 NEW NEW".as_slice() } else { b"1 {can't read \"y\": no such variable} NEW".as_slice() }),
                    (b"proc lifetime {} {namespace eval ::life2 {variable x OLD}; upvar #0 ::life2::x y; unset ::life2::x; set y NEW; list $y $::life2::x}; lifetime", b"NEW NEW"),
                    (b"proc lifetime {} {namespace eval ::write_life {variable x OLD};upvar #0 ::write_life::x y;namespace delete ::write_life;namespace eval ::write_life {variable x NEW};list [catch {set y NEXT} msg] $msg $::write_life::x}; lifetime", if dialect == "jim" { b"0 NEXT NEXT".as_slice() } else { b"1 {can't set \"y\": upvar refers to variable in deleted namespace} NEW".as_slice() }),
                    (b"proc lifetime {} {set a(k) OLD;upvar 0 a(k) y;unset a;set a(k) NEW;list [catch {set y NEXT} msg] $msg $a(k)}; lifetime", if dialect == "jim" { b"0 NEXT NEXT".as_slice() } else { b"1 {can't set \"y\": upvar refers to element in deleted array} NEW".as_slice() }),
                ] {
                    assert_eq!(i.eval_str(script), Code::Ok, "{dialect}: {}", String::from_utf8_lossy(&i.result_bytes()));
                    assert_eq!(i.result_bytes(), expected, "{dialect}: {}", String::from_utf8_lossy(script));
                }
            });
        }
    }

    #[test]
    fn qualified_variable_fallback_depends_on_cell_and_release() {
        for (dialect, expected) in [
            ("tcl8.4", b"11 0".as_slice()),
            ("tcl8.6", b"11 0".as_slice()),
            ("tcl9.0", b"9 1".as_slice()),
        ] {
            leak_free(|i| {
                i.set_dialect_profile(tcl_dialect::DialectProfile::find(dialect).expect("profile"));
                assert_eq!(i.eval_str(b"namespace eval R {variable x 9}; namespace eval N {namespace eval R {}; proc p {} {set R::x 11}}; N::p; list $::R::x [info exists ::N::R::x]"), Code::Ok);
                assert_eq!(i.result_bytes(), expected, "{dialect}");
            });
        }
    }

    #[test]
    fn jim_namespace_delete_retires_flat_variable_cells() {
        leak_free(|i| {
            let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
            i.set_dialect_profile(profile);
            assert_eq!(i.eval_str(b"namespace eval N {variable v 4; namespace eval C {variable x 3}}; namespace delete N; list [info exists ::N::v] [info exists ::N::C::x]"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0 0");
        });
    }

    #[test]
    fn jim_activation_variables() {
        leak_free(|i| {
            let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
            i.set_dialect_profile(profile);
            assert_eq!(i.eval_str(br#"namespace eval N {set ephemeral 5; variable v 7; list $ephemeral $v [info locals] [info vars]}; list [info exists ::N::ephemeral] [set ::N::v]"#), Code::Ok);
            assert_eq!(i.result_bytes(), b"0 7");
        });
    }

    #[test]
    fn jim_relative_qualified_proc_variable() {
        leak_free(|i| {
            let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
            i.set_dialect_profile(profile);
            assert_eq!(i.eval_str(br#"namespace eval N {variable R::x 9; proc p {} {set R::x 10; list $R::x $::N::R::x}}; N::p"#), Code::Ok);
            assert_eq!(i.result_bytes(), b"10 9");
        });
    }

    #[test]
    fn jim_flat_absolute_variable_names() {
        leak_free(|i| {
            let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
            i.set_dialect_profile(profile);
            assert_eq!(i.eval_str(br#"set ::Missing::x 3; set ::N:::x 4; set ::N::x 5; list $::::Missing::x $::N:::x $::N::x"#), Code::Ok);
            assert_eq!(i.result_bytes(), b"3 4 5");
        });
    }

    #[test]
    fn jim_namespace_root_is_local_activation() {
        leak_free(|i| {
            let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
            i.set_dialect_profile(profile);
            assert_eq!(
                i.eval_str(
                    br#"namespace eval :: {set ephemeralRoot 11}; info exists ::ephemeralRoot"#
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"0");
        });
    }

    #[test]
    fn jim_upvar_targets_selected_activation() {
        leak_free(|i| {
            let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
            i.set_dialect_profile(profile);
            assert_eq!(i.eval_str(br#"proc inner {} {upvar 1 R::x link; set link 12}; proc outer {} {set R::x 3; inner; set R::x}; outer"#), Code::Ok);
            assert_eq!(i.result_bytes(), b"12");
        });
    }

    #[test]
    fn jim_variable_declaration_preserves_colon_runs() {
        leak_free(|i| {
            let profile = Box::leak(Box::new(tcl_dialect::DialectProfile::projected_from_point(
                "jim",
                &[],
                "Jim",
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
            )));
            i.set_dialect_profile(profile);
            assert_eq!(i.eval_str(br#"namespace eval N {variable R:::v 6; incr v}; list [set ::N::R:::v] [info exists ::N::R::v]"#), Code::Ok);
            assert_eq!(i.result_bytes(), b"7 0");
        });
    }

    #[test]
    fn variable_in_namespace_eval_initialises_ns_var() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"namespace eval a { variable v 10 }"), Code::Ok);
            assert_eq!(i.eval_str(b"set ::a::v"), Code::Ok);
            assert_eq!(i.result_bytes(), b"10");
            // multiple name/value pairs + a trailing declare-only name.
            assert_eq!(
                i.eval_str(b"namespace eval a { variable m1 1 m2 2 q }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"set ::a::m1"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            assert_eq!(i.eval_str(b"set ::a::m2"), Code::Ok);
            assert_eq!(i.result_bytes(), b"2");
            // `q` was declared without a value → still unset.
            assert_eq!(i.eval_str(b"set ::a::q"), Code::Error);
            i.eval_str(b"unset ::a::v ::a::m1 ::a::m2");
        });
    }

    #[test]
    fn upvar_relative_zero_in_namespace_eval_aliases_ns_var() {
        // `upvar 0 Option(-x) alias` inside `namespace eval` links the alias to
        // the *namespace* array element (not the global frame). tcltest relies
        // on this for its option/accessor machinery.
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"namespace eval foo { variable Opt; set Opt(-x) 5; upvar 0 Opt(-x) a; set a }"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"5");
            // a write through the alias reaches the array element.
            i.eval_str(b"namespace eval foo { set a 9 }");
            assert_eq!(i.eval_str(b"set ::foo::Opt(-x)"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9");
            // a scalar alias too.
            assert_eq!(
                i.eval_str(b"namespace eval bar { variable r 1; upvar 0 r s; set s }"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1");
            i.eval_str(b"unset -nocomplain ::foo::Opt ::bar::r");
        });
    }

    #[test]
    fn variable_qualified_target() {
        leak_free(|i| {
            i.eval_str(b"namespace eval a {}");
            // `variable ::a::x 5` from the global scope declares in ::a.
            assert_eq!(i.eval_str(b"variable ::a::x 5"), Code::Ok);
            assert_eq!(i.eval_str(b"set ::a::x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            // C top-level declarations do not create a procedure-local tail link.
            // Native rows: tests/data/native_variable_qualified_target.
            assert_eq!(i.eval_str(b"set x"), Code::Error);
            assert_eq!(i.result_bytes(), b"can't read \"x\": no such variable");
            i.eval_str(b"unset ::a::x");
        });
    }

    #[test]
    fn procedure_qualified_target_links_the_actual_local_tail() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            counters::reset();
            {
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs::default(),
                )
                .unwrap();
                assert_eq!(interp.eval_str(b"namespace eval a {}; proc qualified {} {variable ::a::x 5; set x 7; set ::a::x}; qualified"), Code::Ok, "{engine}: {:?}", interp.result_bytes());
                assert_eq!(interp.result_bytes(), b"7", "{engine}");
                assert!(!interp.host_refusal_pending(), "{engine}");
            }
            assert_eq!(counters::finalize(), 0, "{engine}");
            assert_eq!(counters::double_free_count(), 0, "{engine}");
        }
    }

    #[test]
    fn variable_into_missing_namespace_errors() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"variable ::nosuch::x 1"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't define \"::nosuch::x\": parent namespace doesn't exist"
            );
        });
    }

    #[test]
    fn global_is_noop_at_top_level() {
        leak_free(|i| {
            // `global g` at global scope must not loop / error; the var is plain.
            assert_eq!(i.eval_str(b"global g"), Code::Ok);
            assert_eq!(i.eval_str(b"set g 1"), Code::Ok);
            assert_eq!(i.eval_str(b"set ::g"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            i.eval_str(b"unset ::g");
        });
    }

    #[test]
    fn upvar_at_global_links_to_namespace_var() {
        leak_free(|i| {
            i.eval_str(b"namespace eval a { variable x 5 }");
            // `upvar #0 ::a::x y` at global scope, no proc frame.
            assert_eq!(i.eval_str(b"upvar #0 ::a::x y"), Code::Ok);
            assert_eq!(i.result_bytes(), b""); // upvar returns empty
            assert_eq!(i.eval_str(b"set y"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            // write through the link reaches the namespace var
            assert_eq!(i.eval_str(b"set y 99"), Code::Ok);
            assert_eq!(i.eval_str(b"set ::a::x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"99");
            i.eval_str(b"unset ::a::x");
        });
    }

    #[test]
    fn upvar_errors() {
        leak_free(|i| {
            // bad level (no caller above global).
            assert_eq!(i.eval_str(b"upvar #5 foo bar"), Code::Error);
            assert_eq!(i.result_bytes(), b"bad level \"#5\"");
            // qualified other-var into a missing namespace.
            assert_eq!(i.eval_str(b"upvar #0 ::nosuch::z w"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't access \"::nosuch::z\": parent namespace doesn't exist"
            );
        });
    }

    /// The resolver classifies both homes before command-level validation:
    /// namespace aliases may not retain procedure cells, while the missing
    /// parent and element-name paths retain Tcl's distinct error codes.
    #[test]
    fn upvar_validates_semantic_homes_and_error_precedence() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(b"namespace eval x { variable ok READY }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"upvar #0 ::x::ok top"), Code::Ok);
            assert_eq!(i.eval_str(b"set top"), Code::Ok);
            assert_eq!(i.result_bytes(), b"READY");

            assert_eq!(
                i.eval_str(
                    b"proc inverted {} { set proc_local 1; upvar 0 proc_local ::x::link(k) }"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"inverted"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"bad variable name \"::x::link(k)\": can't create namespace variable that refers to procedure variable"
            );
            assert_eq!(i.eval_str(b"set ::errorCode"), Code::Ok);
            assert_eq!(i.result_bytes(), b"TCL UPVAR INVERTED");

            for (script, message, code) in [
                (
                    &b"upvar #0 ::missing::x local"[..],
                    &b"can't access \"::missing::x\": parent namespace doesn't exist"[..],
                    &b"TCL LOOKUP VARNAME ::missing::x"[..],
                ),
                (
                    b"upvar #0 x ::missing::local",
                    b"can't create \"::missing::local\": parent namespace doesn't exist",
                    b"TCL LOOKUP VARNAME ::missing::local",
                ),
                (
                    b"upvar #0 x local(k)",
                    b"bad variable name \"local(k)\": can't create a scalar variable that looks like an array element",
                    b"TCL UPVAR LOCAL_ELEMENT",
                ),
            ] {
                assert_eq!(i.eval_str(script), Code::Error, "{script:?}");
                assert_eq!(i.result_bytes(), message, "{script:?}");
                assert_eq!(i.eval_str(b"set ::errorCode"), Code::Ok, "{script:?}");
                assert_eq!(i.result_bytes(), code, "{script:?}");
            }
            i.eval_str(b"unset -nocomplain ::x::ok ::x::link top");
        });
    }

    /// The 8.x namespace-scope fallback to global, off by default (9.0 /
    /// TIP 278) and on for an 8.x runtime version — tclsh 8.6/9.0-pinned
    /// (reads fall back, writes hit the global, a `variable` declaration
    /// blocks it, and `info exists` / `unset` agree).
    #[test]
    fn ns_scope_unqualified_falls_back_to_global_only_under_8x() {
        // Default (9.0): no fallback anywhere.
        leak_free(|i| {
            i.eval_str(b"set g GLOBAL");
            assert_eq!(i.eval_str(b"namespace eval foo { set g }"), Code::Error);
            assert_eq!(
                i.eval_str(b"namespace eval foo { set g WRITTEN }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"set ::foo::g"), Code::Ok);
            assert_eq!(i.result_bytes(), b"WRITTEN");
            assert_eq!(i.eval_str(b"set ::g"), Code::Ok);
            assert_eq!(i.result_bytes(), b"GLOBAL");
            assert_eq!(i.eval_str(b"namespace eval q { info exists g }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            i.eval_str(b"unset -nocomplain ::g ::foo::g");
        });
        // 8.x: reads fall back, writes reach the global, declared names block.
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            i.eval_str(b"set g GLOBAL");
            assert_eq!(i.eval_str(b"namespace eval foo { set g }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"GLOBAL");
            assert_eq!(
                i.eval_str(b"namespace eval foo { set g WRITTEN }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"info exists ::foo::g"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0", "the write must reach the global");
            assert_eq!(i.eval_str(b"set ::g"), Code::Ok);
            assert_eq!(i.result_bytes(), b"WRITTEN");
            assert_eq!(i.eval_str(b"namespace eval q { info exists g }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // A declared-but-unset `variable` blocks the fallback.
            i.eval_str(b"set v GLOBALV");
            assert_eq!(
                i.eval_str(b"namespace eval bar { variable v; set v }"),
                Code::Error,
                "a declared-but-unset `variable` blocks the fallback"
            );
            // With neither cell present, a write creates in the namespace.
            assert_eq!(i.eval_str(b"namespace eval foo { set fresh NS }"), Code::Ok);
            assert_eq!(i.eval_str(b"info exists ::foo::fresh"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            i.eval_str(b"unset -nocomplain ::g ::v ::foo::fresh ::bar::v");
        });
    }

    /// The gate is selected by the emulated *release*, not by a caller
    /// remembering which side of 9.0 the fallback lives on
    /// (`set_runtime_version`).
    ///
    /// Both directions, for every modelled release: 8.4/8.5/8.6 fall back and
    /// 9.0/9.1 do not, and each release's *opposite* behaviour must not hold.
    #[test]
    fn the_fallback_is_selected_by_the_emulated_release() {
        use tcl_dialect::TclVersion as V;
        for (version, falls_back) in [
            (V::V8_4, true),
            (V::V8_5, true),
            (V::V8_6, true),
            (V::V9_0, false),
            (V::V9_1, false),
        ] {
            leak_free(|i| {
                i.set_runtime_version(version);
                i.eval_str(b"set g GLOBAL");
                let read = i.eval_str(b"namespace eval foo { set g }");
                if falls_back {
                    assert_eq!(read, Code::Ok, "{version:?} must fall back on read");
                    assert_eq!(i.result_bytes(), b"GLOBAL", "{version:?}");
                } else {
                    assert_eq!(read, Code::Error, "{version:?} must NOT fall back");
                }
                // A write either reaches the global (8.x) or creates a fresh
                // namespace variable and leaves the global alone (9.x).
                assert_eq!(i.eval_str(b"namespace eval foo { set g W }"), Code::Ok);
                assert_eq!(i.eval_str(b"info exists ::foo::g"), Code::Ok);
                assert_eq!(
                    i.result_bytes(),
                    if falls_back { b"0" } else { b"1" },
                    "{version:?}: namespace variable creation"
                );
                assert_eq!(i.eval_str(b"set ::g"), Code::Ok);
                assert_eq!(
                    i.result_bytes(),
                    if falls_back {
                        &b"W"[..]
                    } else {
                        &b"GLOBAL"[..]
                    },
                    "{version:?}: the global's value"
                );
                i.eval_str(b"unset -nocomplain ::g ::foo::g");
            });
        }
    }

    /// `set_runtime_version` re-derives the release-reporting globals, so
    /// `info patchlevel` answers for the emulated release — and does so
    /// without `--init`, because C sets them in `Tcl_CreateInterp`
    /// (`generic/tclBasic.c:1346`), not `Tcl_Init`.
    #[test]
    fn the_emulated_release_is_reported_without_init() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"info patchlevel"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9.0.4", "the 9.0 default");
            assert_eq!(i.eval_str(b"set ::tcl_version"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9.0");
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            assert_eq!(i.eval_str(b"info patchlevel"), Code::Ok);
            assert_eq!(i.result_bytes(), b"8.6.18");
            assert_eq!(i.eval_str(b"set ::tcl_version"), Code::Ok);
            assert_eq!(i.result_bytes(), b"8.6");
        });
    }

    /// The rule governs *relative variable resolution*, so it reaches every
    /// command that resolves a relative name — not just the `append` the
    /// issue was filed on.  tclsh 8.6.16/9.0.4-pinned.
    #[test]
    fn the_fallback_governs_every_relative_name_resolution() {
        // 8.x: each command reaches the global.
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            i.eval_str(b"set a foo; set l a; set c 1; array set A {k v}");
            i.eval_str(b"namespace eval n { append a baz; lappend l b; incr c; set A(k) NEW }");
            for (expr, want) in [
                (&b"set ::a"[..], &b"foobaz"[..]),
                (b"set ::l", b"a b"),
                (b"set ::c", b"2"),
                (b"array get ::A", b"k NEW"),
            ] {
                assert_eq!(i.eval_str(expr), Code::Ok);
                assert_eq!(
                    i.result_bytes(),
                    want,
                    "8.6: {}",
                    String::from_utf8_lossy(expr)
                );
            }
            // `unset` and `$`-substitution follow the same rule.
            assert_eq!(i.eval_str(b"namespace eval n { unset a }"), Code::Ok);
            assert_eq!(i.eval_str(b"info exists ::a"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0", "8.6: unset reaches the global");
            assert_eq!(i.eval_str(b"namespace eval n { set x \"<$l>\" }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"<a b>");
            i.eval_str(b"unset -nocomplain ::a ::l ::c ::A ::n::x");
        });
        // 9.0: none of them do — each binds in the namespace instead.
        leak_free(|i| {
            i.eval_str(b"set a foo; set l a; set c 1");
            i.eval_str(b"namespace eval n { append a baz; lappend l b; incr c }");
            for (expr, want) in [
                (&b"set ::a"[..], &b"foo"[..]),
                (b"set ::l", b"a"),
                (b"set ::c", b"1"),
                (b"set ::n::a", b"baz"),
            ] {
                assert_eq!(i.eval_str(expr), Code::Ok);
                assert_eq!(
                    i.result_bytes(),
                    want,
                    "9.0: {}",
                    String::from_utf8_lossy(expr)
                );
            }
            assert_eq!(
                i.eval_str(b"namespace eval n { unset nosuch }"),
                Code::Error,
                "9.0: no fallback for unset"
            );
            assert_eq!(
                i.eval_str(b"namespace eval n2 { set x \"<$l>\" }"),
                Code::Error,
                "9.0: no fallback for $-substitution"
            );
            i.eval_str(b"unset -nocomplain ::a ::l ::c");
        });
    }

    /// "Current, then global" — never the intermediate parents — and an
    /// existing namespace variable shadows the global in *both* releases.
    #[test]
    fn the_fallback_never_walks_intermediate_parents() {
        for version in [tcl_dialect::TclVersion::V8_6, tcl_dialect::TclVersion::V9_0] {
            leak_free(|i| {
                i.set_runtime_version(version);
                // A parent namespace's variable is never found from a child.
                i.eval_str(b"namespace eval P { variable pv PARENT }");
                assert_eq!(
                    i.eval_str(b"namespace eval P { namespace eval C { set pv } }"),
                    Code::Error,
                    "{version:?}: parents are never searched"
                );
                // An existing namespace variable shadows the global.
                i.eval_str(b"set sh GLOBAL");
                i.eval_str(b"namespace eval S { variable sh NS }");
                assert_eq!(i.eval_str(b"namespace eval S { set sh }"), Code::Ok);
                assert_eq!(i.result_bytes(), b"NS", "{version:?}: shadowing");
                i.eval_str(b"unset -nocomplain ::sh ::S::sh ::P::pv");
            });
        }
        // Nesting depth is irrelevant to the fallback: a deeply nested
        // namespace still reaches the *global* under 8.x.
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            i.eval_str(b"set deep G");
            assert_eq!(
                i.eval_str(
                    b"namespace eval a { namespace eval b { namespace eval c { set deep } } }"
                ),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"G");
            i.eval_str(b"unset -nocomplain ::deep");
        });
    }

    /// `namespace upvar` is an explicit link, so the fallback never applies to
    /// it in either release.  (Covered here rather than in `tcl-vm`'s
    /// cross-version suite: that VM's `namespace` ensemble has no `upvar`
    /// subcommand yet.)
    #[test]
    fn namespace_upvar_is_explicit_in_both_releases() {
        for version in [tcl_dialect::TclVersion::V8_6, tcl_dialect::TclVersion::V9_0] {
            leak_free(|i| {
                i.set_runtime_version(version);
                i.eval_str(b"set g G");
                assert_eq!(
                    i.eval_str(b"namespace eval n { namespace upvar :: g z; set z }"),
                    Code::Ok
                );
                assert_eq!(i.result_bytes(), b"G", "{version:?}");
                i.eval_str(b"unset -nocomplain ::g ::n::z");
            });
        }
    }

    /// Each interpreter owns its own global namespace, so the fallback
    /// resolves per-interp — a child never reaches its parent's global.
    #[test]
    fn the_fallback_resolves_per_interpreter() {
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            i.eval_str(b"set g PARENT");
            i.eval_str(b"interp create kid");
            i.eval_str(b"kid eval {set g KID}");
            assert_eq!(
                i.eval_str(b"kid eval {namespace eval n { set g }}"),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"KID",
                "the child falls back to its OWN global, not the parent's"
            );
            assert_eq!(i.eval_str(b"set ::g"), Code::Ok);
            assert_eq!(
                i.result_bytes(),
                b"PARENT",
                "the parent's global is untouched"
            );
            i.eval_str(b"interp delete kid");
            i.eval_str(b"unset -nocomplain ::g");
        });
    }

    /// The user-visible consequence of getting resolution wrong: a trace must
    /// fire on the variable an access *resolves to*, whatever spelling either
    /// side used.
    #[test]
    fn traces_fire_on_the_resolved_variable_not_the_spelling() {
        // A qualified registration must catch an unqualified access, and vice
        // versa — independent of the fallback (this is the 9.0 default).
        leak_free(|i| {
            i.eval_str(b"proc log {n1 n2 op} { lappend ::fired $n1:$op }");
            i.eval_str(b"set ::fired {}");
            i.eval_str(b"set a1 foo; trace add variable ::a1 write log; set a1 X");
            i.eval_str(b"set a2 foo; trace add variable a2 write log; set ::a2 X");
            assert_eq!(i.eval_str(b"set ::fired"), Code::Ok);
            // `name1` is the *access* spelling, not the resolved name: C
            // hands the callback the caller's `part1` unchanged
            // (`TclCallVarTraces`). tclsh 8.6.16/9.0.4 both print
            // `a1:write ::a2:write` for this sheet.
            assert_eq!(
                i.result_bytes(),
                b"a1:write ::a2:write",
                "both spellings must fire"
            );
            i.eval_str(b"unset -nocomplain ::fired ::a1 ::a2");
        });
        // Under the 8.x fallback the write reaches the GLOBAL, so the global's
        // trace must fire — and so must its unset trace, which needs the key
        // resolved *before* the variable is removed.
        leak_free(|i| {
            i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
            i.eval_str(b"proc log {n1 n2 op} { lappend ::fired $n1:$op }");
            i.eval_str(b"set ::fired {}");
            i.eval_str(b"set g foo");
            i.eval_str(b"trace add variable g write log");
            i.eval_str(b"trace add variable g unset log");
            i.eval_str(b"namespace eval n { set g X }");
            i.eval_str(b"namespace eval n { unset g }");
            assert_eq!(i.eval_str(b"set ::fired"), Code::Ok);
            assert_eq!(
                i.result_bytes(),
                b"g:write g:unset",
                "a write and an unset through the fallback fire the global's traces"
            );
            i.eval_str(b"unset -nocomplain ::fired");
        });
    }
}
