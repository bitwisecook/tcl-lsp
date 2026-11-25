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

//! `interp` subcommands that create, address, configure, evaluate in and delete child interpreters.

use crate::interp::native_arguments::OriginalArguments;
use crate::interp::{drop_fresh, Code, Interp};
use crate::obj;
use crate::obj::TclObj;
use tcl_syntax::value::ValueOps;

/// `interp target path alias` — the interp-path (from this interp) to the
/// target interpreter of `alias`, as installed in the interpreter addressed
/// by `path`. Cheap given the two alias shapes this runtime supports
/// (same-interp, or child-to-immediate-parent) — see
/// [`Interp::alias_target_path`]. `cancel`/`share`/`transfer` are the other
/// three subcommands tclsh advertises here that this runtime does not
/// implement; unlike `target` they need infrastructure (script cancellation,
/// cross-interp channel sharing) this runtime has none of.
pub(super) fn interp_target(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_target_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_target_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"interp target path alias");
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    let alias = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    match interp.alias_target_path(&path, &alias) {
        Ok(Some(target_path)) => {
            let elems: Vec<*mut TclObj> = target_path
                .iter()
                .map(|n| obj::new_string_bytes(n))
                .collect();
            interp.set_result(interp.new_list_object(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error.into()),
        Ok(None) => {
            let mut m = b"alias \"".to_vec();
            m.extend_from_slice(&alias);
            m.extend_from_slice(b"\" in path \"");
            let original_path = match arguments.bytes(2) {
                Ok(bytes) => bytes,
                Err(error) => return interp.report_cmd_error(error),
            };
            m.extend_from_slice(&original_path);
            m.extend_from_slice(b"\" not found");
            let code = crate::interp::error_code_list(&[b"TCL", b"LOOKUP", b"ALIAS", &alias]);
            interp.error_with_code(&m, &code)
        }
    }
}

/// `interp create`'s option words (`createOptions[]`, `tclInterp.c`), resolved
/// with `Tcl_GetIndexFromObj(…, "option", 0)`: `-s` abbreviates `-safe` and the
/// lone `-` — a prefix of both entries — is `ambiguous option "-"`. Only a word
/// starting with `-` reaches the table, so an empty word is a path, not a miss.
const CREATE_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &[b"-safe", b"--"]);

/// `interp create ?-safe? ?--? ?path?` — create a child interpreter, returning
/// its name (auto-generated `interpN` when omitted). `-safe` hides the
/// host-touching commands; it does not re-alias `source`/`load`/`file`
/// through the Safe Base.
pub(super) fn interp_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_create_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_create_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    // C's "weird historical rule": `-safe` is accepted anywhere before `--`
    // (`interp create a -safe` is valid), and the path is the lone non-option
    // word — so scan all args rather than stopping at the first non-flag.
    let mut name_obj: Option<*mut TclObj> = None;
    let mut safe = false;
    let mut last = false;
    let mut i = 2;
    while i < argv.len() {
        let a = match arguments.bytes(i) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error),
        };
        if !last && a.first() == Some(&b'-') {
            let selected =
                interp.native_static_option_index(argv[i], CREATE_OPTIONS.names(), false, "option");
            if let Err(error) = arguments.check() {
                return interp.report_cmd_error(error);
            }
            match selected {
                Ok(0) => {
                    safe = true;
                    i += 1;
                    continue;
                }
                Ok(_) => {
                    i += 1;
                    last = true;
                }
                Err(m) => {
                    return interp.report_cmd_error(m);
                }
            }
        }
        if name_obj.is_some() {
            return interp.wrong_args(b"interp create ?-safe? ?--? ?path?");
        }
        if i < argv.len() {
            name_obj = Some(argv[i]);
        }
        i += 1;
    }
    // A safe parent makes every child safe, with or without `-safe`
    // (`Tcl_CreateChild`).
    let Some(original) = name_obj else {
        let created = match interp.create_tcl_child(None, safe) {
            Ok(created) => created,
            Err(code) => return code,
        };
        interp.set_result_bytes(&created);
        return Code::Ok;
    };
    let path = match interp_path(interp, original) {
        Ok(path) => path,
        Err(code) => return code,
    };
    interp_create_from_original_path(interp, original, &path, safe)
}

fn interp_create_from_original_path(
    interp: &mut Interp,
    original: *mut TclObj,
    path: &[Vec<u8>],
    safe: bool,
) -> Code {
    let (leaf, parent) = if path.len() < 2 {
        let bytes = match interp.native_string_bytes(&original) {
            Ok(bytes) => bytes,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        let policy = interp
            .name_policy_protocol()
            .expect("path selected the original protocol");
        let selected = match policy.recipe().interpreter_child_input(&bytes) {
            Ok(selected) => selected.selected().to_vec(),
            Err(_) => {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "interpreter creation key",
                    )
                    .into(),
                );
            }
        };
        (selected, &path[..0])
    } else {
        (path.last().unwrap().clone(), &path[..path.len() - 1])
    };
    let outcome = interp.with_child_path(parent, |owner| {
        if owner.child_exists(&leaf) {
            return Ok(false);
        }
        if let Err(code) = owner.create_tcl_child(Some(leaf.clone()), safe) {
            return Err((code, owner.clone()));
        }
        Ok(true)
    });
    match outcome {
        Some(Err((_, owner))) if owner.host_refusal_pending() && !interp.host_refusal_pending() => {
            interp.transport_host_refusal_from(&owner)
        }
        Some(Err((code, owner))) => {
            if !interp.host_refusal_pending() {
                interp.set_result(owner.get_obj_result());
            }
            code
        }
        Some(Ok(true)) => {
            interp.set_result(original);
            Code::Ok
        }
        Some(Ok(false)) => interp.set_error(
            &[
                b"interpreter named \"".as_slice(),
                &leaf,
                b"\" already exists, cannot create",
            ]
            .concat(),
        ),
        None => not_found_path(interp, parent),
    }
}

/// Parse the original list object, then select each C child-table CString key.
/// A failed native list/getter operation never becomes a literal-name fallback.
pub(crate) fn interp_path(
    interp: &mut Interp,
    original: *mut TclObj,
) -> Result<Vec<Vec<u8>>, Code> {
    let arguments = OriginalArguments::enter(interp, &[original])
        .map_err(|error| interp.report_cmd_error(error))?;
    let policy = interp.name_policy_protocol().ok_or_else(|| {
        interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("interpreter path protocol")
                .into(),
        )
    })?;
    let elements = ValueOps::list_elements(interp, &original);
    arguments
        .check()
        .map_err(|error| interp.report_cmd_error(error))?;
    let elements = elements.map_err(|error| interp.report_cmd_error(error.into()))?;
    let members = OriginalArguments::enter(interp, &elements)
        .map_err(|error| interp.report_cmd_error(error))?;
    let selected = (0..elements.len())
        .map(|index| {
            let bytes = members.bytes(index);
            arguments
                .check()
                .map_err(|error| interp.report_cmd_error(error))?;
            let bytes = bytes.map_err(|error| interp.report_cmd_error(error))?;
            let selected = policy
                .recipe()
                .interpreter_child_input(&bytes)
                .map_err(|_| {
                    interp.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "interpreter child key",
                        )
                        .into(),
                    )
                })?;
            Ok(selected.selected().to_vec())
        })
        .collect::<Result<Vec<_>, Code>>()?;
    members
        .check()
        .map_err(|error| interp.report_cmd_error(error))?;
    arguments
        .check()
        .map_err(|error| interp.report_cmd_error(error))?;
    Ok(selected)
}

pub(crate) fn not_found_original_path(interp: &mut Interp, original: *mut TclObj) -> Code {
    let bytes = match interp.native_string_bytes(&original) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let Some(policy) = interp.name_policy_protocol() else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "interpreter diagnostic protocol",
            )
            .into(),
        );
    };
    let reported = match policy.recipe().interpreter_child_input(&bytes) {
        Ok(selected) => selected.selected().to_vec(),
        Err(_) => {
            return interp.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "interpreter diagnostic extent",
                )
                .into(),
            );
        }
    };
    let code = tcl_syntax::list_result::NativeListResultSerialization::for_string_protocol(
        policy.string_protocol(),
    )
    .render(&[b"TCL".as_slice(), b"LOOKUP", b"INTERP", &reported]);
    interp.error_with_code(
        &[
            b"could not find interpreter \"".as_slice(),
            &reported,
            b"\"",
        ]
        .concat(),
        &code,
    )
}

/// The `could not find interpreter "a b"` error for a path that failed to
/// resolve, rendering the path as a Tcl list.
pub(super) fn not_found_path(interp: &mut Interp, path: &[Vec<u8>]) -> Code {
    let elems: Vec<*mut TclObj> = path.iter().map(|n| obj::new_string_bytes(n)).collect();
    let joined = obj::Owned::fresh(interp.new_list_object(&elems));
    for e in elems {
        drop_fresh(e);
    }
    let arguments = match OriginalArguments::enter(interp, &[joined.as_ptr()]) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let rendered = match arguments.bytes(0) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let mut m = b"could not find interpreter \"".to_vec();
    m.extend_from_slice(&rendered);
    m.push(b'"');
    arguments.finish(interp.set_error(&m))
}

/// `interp limit path limitType ?-option value …?` — query/configure the
/// `commands` or `time` limit on a child interp.
pub(super) fn interp_limit(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_limit_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_limit_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    // argv = [interp, limit, path, limitType, opts…]
    if argv.len() < 4 {
        return interp.wrong_args(b"interp limit path limitType ?-option value ...?");
    }
    // Validate the limit type before the current-interp guard so a bad type is
    // reported ahead of the inaccessibility error (interp-35.3 vs .23).
    if let Err(m) = interp.native_static_option_index(
        argv[3],
        crate::interp::LIMIT_TYPES.names(),
        false,
        "limit type",
    ) {
        return interp.report_cmd_error(m);
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    if path.is_empty() {
        return interp.set_error(b"limits on current interpreter inaccessible");
    }
    let ltype = match arguments.bytes(3) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error),
    };
    let opts: Vec<*mut TclObj> = argv[4..].to_vec();
    match interp.with_child_path(&path, |c| c.limit_apply(&ltype, &opts)) {
        Some(Ok(o)) => {
            interp.set_result(o);
            Code::Ok
        }
        Some(Err(error)) => interp.report_cmd_error(error),
        None => not_found_original_path(interp, argv[2]),
    }
}

/// `interp marktrusted path` — clear a child interp's safe flag (denied from a
/// safe interpreter).
pub(super) fn interp_marktrusted(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"interp marktrusted path");
    }
    if interp.is_safe() {
        return interp.set_error(b"permission denied: safe interpreter cannot mark trusted");
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    if path.is_empty() {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    match interp.with_child_path(&path, |c| c.mark_trusted()) {
        Some(()) => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        None => not_found_original_path(interp, argv[2]),
    }
}

/// `interp debug path ?-frame ?bool??` — the per-interp frame-debug switch.
pub(super) fn interp_debug(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 || argv.len() > 5 {
        return interp.wrong_args(b"interp debug path ?-frame ?bool??");
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    let opts: Vec<*mut TclObj> = argv[3..].to_vec();
    match interp.with_child_path(&path, |c| c.debug_apply(&opts)) {
        Some(Ok(o)) => {
            interp.set_result(o);
            Code::Ok
        }
        Some(Err(error)) => interp.report_cmd_error(error),
        None => not_found_original_path(interp, argv[2]),
    }
}

/// `interp eval path arg ?arg ...?` — evaluate a script in a child interpreter.
pub(super) fn interp_eval(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = interp_eval_original(interp, argv, &arguments);
    arguments.finish(code)
}

fn interp_eval_original(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &OriginalArguments,
) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"interp eval path arg ?arg ...?");
    }
    let path = match interp_path(interp, argv[2]) {
        Ok(path) => path,
        Err(code) => return code,
    };
    let script = if argv.len() == 4 {
        obj::Owned::retain(argv[3])
    } else {
        match interp.concat_original_arguments(&argv[3..]) {
            Ok(script) => obj::Owned::retain(script),
            Err(error) => return interp.report_cmd_error(error),
        }
    };
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    let outcome = interp.with_child_path(&path, |target| {
        let scope =
            match crate::interp::native_operation_currency::NativeOperationScope::enter(target) {
                Ok(scope) => scope,
                Err(first) => {
                    target.refuse_native_execution(first);
                    return Err(target.clone());
                }
            };
        let code = target.eval_body_obj(script.as_ptr());
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            target.refuse_native_execution(first);
            return Err(target.clone());
        }
        // Preserve the actual owned result, without an extra options getter or
        // a String reconstruction that would change this existing contract.
        let result = obj::Owned::retain(target.get_obj_result());
        if let Err(first) = scope.currency().ensure_current_or_refuse() {
            target.refuse_native_execution(first);
            return Err(target.clone());
        }
        Ok((code, result))
    });
    drop(script);
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    match outcome {
        Some(Ok((code, result))) => {
            interp.set_result(result.as_ptr());
            drop(result);
            code
        }
        Some(Err(child)) => interp.transport_host_refusal_from(&child),
        None => not_found_original_path(interp, argv[2]),
    }
}

/// `interp delete ?path ...?` — delete each named child interpreter.
pub(super) fn interp_delete(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    for &a in &argv[2..] {
        let path = match interp_path(interp, a) {
            Ok(path) => path,
            Err(code) => return code,
        };
        let Some((leaf, parent)) = path.split_last() else {
            return not_found_original_path(interp, a);
        };
        let leaf = leaf.clone();
        match interp.with_child_path(parent, |p| p.delete_child(&leaf)) {
            Some(true) => {}
            _ => return not_found_original_path(interp, a),
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}
