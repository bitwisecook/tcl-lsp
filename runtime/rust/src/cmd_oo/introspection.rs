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

//! `info object` and `info class`, called from `cmd_info`.

use super::{err, native_context, wrong_args, Method};
use crate::interp::{Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use tcl_core_types::OoId;
use tcl_syntax::value::ValueOps;

mod call_chain_reports;
mod class_info;
mod object_info;

pub(super) use call_chain_reports::{
    original_core_report_step, render_call_chain, retain_original_terminal_report,
};
pub(crate) use class_info::info_class;
pub(crate) use object_info::info_object;
use object_info::original_info_target;

/// `info object|class forward objName methodName` — the forward prefix list.
fn info_forward(interp: &mut Interp, id: OoId, argv: &[*mut TclObj], class: bool) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() != 5 {
            let u: &[u8] = if class {
                b"info class forward className methodName"
            } else {
                b"info object forward objName methodName"
            };
            return wrong_args(interp, u);
        }
        let target_owner = match original_info_target(interp, originals, id, class) {
            Ok(target) => target,
            Err(code) => return code,
        };
        let name = oo_original_bytes!(interp, originals, 4);
        oo_checked!(interp, target_owner.check(interp));
        let method_owner = {
            let oo = interp.oo.borrow();
            if class {
                oo.classes
                    .get(&id)
                    .and_then(|c| c.methods.owner(name.as_ref()))
            } else {
                oo.objects
                    .get(&id)
                    .and_then(|o| o.methods.owner(name.as_ref()))
            }
        };
        let method = method_owner.as_ref().map(|owner| owner.borrow().clone());
        let code = match method {
            Some(Method::Forward { prefix }) => {
                let members = interp.list_elements(&prefix.as_ptr());
                oo_original_check!(interp, originals);
                match members {
                    Ok(_) => {
                        interp.set_result(prefix.as_ptr());
                        Code::Ok
                    }
                    Err(error) => interp.report_cmd_error(error.into()),
                }
            }
            Some(_) => err(
                interp,
                b"prefix argument list not available for this kind of method",
            ),
            None => unknown_method_named(interp, &name),
        };
        drop(method_owner);
        code
    })
}

/// `info object|class definition fqn methodName` — `{params} {body}`.
fn info_definition(interp: &mut Interp, id: OoId, argv: &[*mut TclObj], class: bool) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() != 5 {
            let u: &[u8] = if class {
                b"info class definition className methodName"
            } else {
                b"info object definition objName methodName"
            };
            return wrong_args(interp, u);
        }
        let target_owner = match original_info_target(interp, originals, id, class) {
            Ok(target) => target,
            Err(code) => return code,
        };
        let name = oo_original_bytes!(interp, originals, 4);
        oo_checked!(interp, target_owner.check(interp));
        let m = {
            let oo = interp.oo.borrow();
            if class {
                oo.classes.get(&id).and_then(|c| c.methods.get(&name))
            } else {
                oo.objects.get(&id).and_then(|o| o.methods.get(&name))
            }
        };
        match m {
            Some(Method::Body { procedure, .. }) => {
                let out = list_procedure_body(interp, &procedure.declaration());
                interp.set_result(out);
                Code::Ok
            }
            Some(_) => err(interp, b"definition not available for this kind of method"),
            None => unknown_method_named(interp, &name),
        }
    })
}

/// `info object|class methodtype fqn methodName` — `method` or `forward`.
fn info_methodtype(interp: &mut Interp, id: OoId, argv: &[*mut TclObj], class: bool) -> Code {
    native_context::with_original_arguments(interp, argv, |interp, originals| {
        if argv.len() != 5 {
            let u: &[u8] = if class {
                b"info class methodtype className methodName"
            } else {
                b"info object methodtype objName methodName"
            };
            return wrong_args(interp, u);
        }
        let target_owner = match original_info_target(interp, originals, id, class) {
            Ok(target) => target,
            Err(code) => return code,
        };
        let name = oo_original_bytes!(interp, originals, 4);
        oo_checked!(interp, target_owner.check(interp));
        let record = {
            let oo = interp.oo.borrow();
            if class {
                oo.classes.get(&id).and_then(|c| c.methods.owner(&name))
            } else {
                oo.objects.get(&id).and_then(|o| o.methods.owner(&name))
            }
        };
        let Some(record) = record else {
            return unknown_method_named(interp, &name);
        };
        let descriptor = record.borrow().report_type().name();
        drop(record);
        oo_original_check!(interp, originals);
        oo_checked!(interp, target_owner.check(interp));
        interp.set_result_bytes(&descriptor);
        Code::Ok
    })
}

/// The `unknown method "X"` error (no method-list suffix — used by the `info`
/// introspection subcommands that name a specific method).
fn unknown_method_named(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"unknown method \"".to_vec();
    m.extend_from_slice(name);
    m.push(b'"');
    interp.set_error(&m)
}

/// Whether `name` passes an optional introspection glob `pattern` (matching the
/// fully-qualified name). `None` (no pattern) always passes.
fn fqn_glob_ok(pattern: Option<&[u8]>, name: &[u8]) -> bool {
    match pattern {
        None => true,
        Some(p) => match (core::str::from_utf8(p), core::str::from_utf8(name)) {
            (Ok(p), Ok(n)) => tcl_syntax::glob::string_match(p, n),
            _ => false,
        },
    }
}

pub(super) fn not_object(interp: &mut Interp, name: &[u8]) -> Code {
    // C (`Tcl_GetObjectFromObj`) reports without surrounding quotes.
    let mut m = name.to_vec();
    m.extend_from_slice(b" does not refer to an object");
    interp.set_error(&m)
}

/// `{params} body` as a 2-element list (for `info class constructor`).
pub(super) fn list_procedure_body(
    interp: &Interp,
    procedure: &crate::interp::ProcDef,
) -> *mut TclObj {
    // The argument spec is a list whose elements are either a bare parameter
    // name or a `{name default}` pair — so `{a {b c} args}` round-trips.
    let param_objs: Vec<*mut TclObj> = procedure
        .params
        .iter()
        .map(|p| match &p.default {
            Some(d) => {
                let pair = [
                    obj::new_string_bytes(&p.name),
                    d.checked_ptr().expect("live native method default"),
                ];
                interp.new_list_object(&pair)
            }
            None => obj::new_string_bytes(&p.name),
        })
        .collect();
    let plist = interp.new_list_object(&param_objs);
    let elems = [
        plist,
        procedure
            .body
            .checked_ptr()
            .expect("live native method source"),
    ];
    interp.new_list_object(&elems)
}

pub(super) fn set_list(interp: &mut Interp, names: &[Vec<u8>]) {
    let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
    // The contextual constructor retains each element; set_result retains
    // the list. Fresh elements therefore need no separate release.
    interp.set_result(interp.new_list_object(&elems));
}

fn set_oo_list(interp: &mut Interp, ids: &[OoId]) {
    let names: Vec<Vec<u8>> = ids.iter().map(|&id| interp.oo_name(id)).collect();
    set_list(interp, &names);
}
