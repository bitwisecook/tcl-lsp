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

//! `rename` + `interp alias` (T1.5, the rename-alias wave).
//!
//! Both layer on the one command resolver in [`crate::namespace`]: `rename`
//! moves/deletes a binding in the table; `interp alias` installs a
//! [`Command::Alias`](crate::interp::Command) redirect that the dispatch
//! trampoline re-resolves *by name, anchored at global, on every call*. See
//! `docs/design/runtime/rename-alias.md` for the as-built contract and
//! `docs/design/contracts/command-alias-resolution.md` for the binding rules.
//!
//! Single-interp scope only: alias source/target interpreter paths must be `{}`
//! (the empty string). Child interpreters + cross-interp aliases are deferred.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::list;
use crate::namespace::RenameOutcome;
use crate::obj::{self, TclObj};

/// Register `rename` and `interp`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"rename", rename);
    interp.register_builtin(b"interp", interp_cmd);
    // `update` is registered by `cmd_event` (the real event loop).
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

// -- rename ----------------------------------------------------------------

/// `rename oldName newName` — move a command, or delete it when `newName` is the
/// empty string. Any command may be renamed, builtins included — C Tcl has no
/// protected list here (`rename ::return ::myreturn` succeeds on tclsh
/// 8.6.16 / 9.0.4).
fn rename(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"rename oldName newName");
    }
    let old = obj_bytes(argv[1]);
    let new = obj_bytes(argv[2]);
    match interp.rename_command(&old, &new) {
        RenameOutcome::Renamed | RenameOutcome::Deleted => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        RenameOutcome::NoSuchCommand => {
            let mut m = b"can't rename \"".to_vec();
            m.extend_from_slice(&old);
            m.extend_from_slice(b"\": command doesn't exist");
            interp.set_error(&m)
        }
    }
}

// -- interp ----------------------------------------------------------------

/// `interp alias` / `interp aliases` (single-interp forms). Other `interp`
/// subcommands (`create`, `eval`, `hide`, …) need child-interp infrastructure
/// and trap here.
fn interp_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"interp cmd ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        b"alias" => interp_alias(interp, argv),
        b"aliases" => interp_aliases(interp, argv),
        b"create" => interp_create(interp, argv),
        b"eval" => interp_eval(interp, argv),
        b"delete" => interp_delete(interp, argv),
        b"exists" => {
            // `interp exists ?path?`: the current interp ("") always exists; a
            // named one exists iff the whole path resolves.
            let path = argv.get(2).map(|&a| interp_path(a)).unwrap_or_default();
            let exists = interp.with_child_path(&path, |_| ()).is_some();
            interp.set_result_bytes(if exists { b"1" } else { b"0" });
            Code::Ok
        }
        b"children" | b"slaves" => {
            // Children of the interp addressed by the (possibly nested) path.
            let path = argv.get(2).map(|&a| interp_path(a)).unwrap_or_default();
            let names = interp
                .with_child_path(&path, |c| c.child_names())
                .unwrap_or_default();
            let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
            interp.set_result(list::new_list_obj(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        b"bgerror" => {
            // `interp bgerror path ?cmdPrefix?` — get/set the (possibly nested)
            // interp's background-error handler.
            if argv.len() < 3 || argv.len() > 4 {
                return wrong_args(interp, b"interp bgerror path ?cmdPrefix?");
            }
            let path = interp_path(argv[2]);
            let prefix = argv.get(3).copied();
            match interp.with_child_path(&path, |c| c.bgerror_apply(prefix)) {
                Some(Ok(h)) => {
                    interp.set_result_bytes(&h);
                    Code::Ok
                }
                Some(Err(m)) => interp.set_error(&m),
                None => not_found_path(interp, &path),
            }
        }
        b"hide" => interp_hidectl(interp, argv, HideOp::Hide),
        b"expose" => interp_hidectl(interp, argv, HideOp::Expose),
        b"invokehidden" => interp_invokehidden(interp, argv),
        b"limit" => interp_limit(interp, argv),
        b"marktrusted" => interp_marktrusted(interp, argv),
        b"debug" => interp_debug(interp, argv),
        b"hidden" => {
            // `interp hidden ?path?` — hidden command names in the interp
            // addressed by the (possibly nested) path.
            let path = argv.get(2).map(|&a| interp_path(a)).unwrap_or_default();
            let names = interp
                .with_child_path(&path, |c| c.hidden_names())
                .unwrap_or_default();
            let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
            interp.set_result(list::new_list_obj(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        b"issafe" => {
            // `interp issafe ?path?` — the current interp (no path) or a child
            // addressed by a (possibly nested) path.
            let path = argv.get(2).map(|&a| interp_path(a)).unwrap_or_default();
            let safe = interp
                .with_child_path(&path, |c| c.is_safe())
                .unwrap_or(false);
            interp.set_result_bytes(if safe { b"1" } else { b"0" });
            Code::Ok
        }
        b"recursionlimit" => {
            // `interp recursionlimit path ?newlimit?` — get/set a (possibly
            // nested) interp's recursion bound.
            if argv.len() < 3 || argv.len() > 4 {
                return wrong_args(interp, b"interp recursionlimit path ?newlimit?");
            }
            let path = interp_path(argv[2]);
            let newlimit = argv.get(3).map(|&a| obj_bytes(a));
            match interp.with_child_path(&path, |c| c.recursion_limit_apply(newlimit.as_deref())) {
                Some(Ok(n)) => {
                    interp.set_result_bytes(n.to_string().as_bytes());
                    Code::Ok
                }
                Some(Err(m)) => interp.set_error(&m),
                None => not_found_path(interp, &path),
            }
        }
        other => {
            let mut m = b"bad option \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(
                b"\": must be alias, aliases, bgerror, cancel, children, create, \
                  debug, delete, eval, exists, expose, hide, hidden, issafe, \
                  invokehidden, limit, marktrusted, recursionlimit, share, \
                  target, or transfer",
            );
            interp.set_error(&m)
        }
    }
}

/// `interp create ?-safe? ?--? ?path?` — create a child interpreter, returning
/// its name (auto-generated `interpN` when omitted). `-safe` hides the
/// host-touching commands (the Safe Base's re-aliasing is a follow-up).
fn interp_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // C's "weird historical rule": `-safe` is accepted anywhere before `--`
    // (`interp create a -safe` is valid), and the path is the lone non-option
    // word — so scan all args rather than stopping at the first non-flag.
    let mut name_obj: Option<*mut TclObj> = None;
    let mut safe = false;
    let mut last = false;
    let mut i = 2;
    while i < argv.len() {
        let a = obj_bytes(argv[i]);
        if !last && a.first() == Some(&b'-') {
            match a.as_slice() {
                b"-safe" => {
                    safe = true;
                    i += 1;
                    continue;
                }
                b"--" => {
                    i += 1;
                    last = true;
                }
                _ => {
                    let mut m = b"bad option \"".to_vec();
                    m.extend_from_slice(&a);
                    m.extend_from_slice(b"\": must be -safe or --");
                    return interp.set_error(&m);
                }
            }
        }
        if name_obj.is_some() {
            return wrong_args(interp, b"interp create ?-safe? ?--? ?path?");
        }
        if i < argv.len() {
            name_obj = Some(argv[i]);
        }
        i += 1;
    }
    // The path is a list of interp names; an empty/absent path auto-names a
    // child of this interp, otherwise the leaf is created inside the interp
    // addressed by the parent segments (`interp create {a b}`).
    let path = name_obj.map(interp_path).unwrap_or_default();
    let Some((leaf, parent)) = path.split_last() else {
        let created = interp.create_child(None);
        if safe {
            interp.with_child(&created, |c| c.make_safe());
        }
        interp.set_result(obj::new_string_bytes(&created));
        return Code::Ok;
    };
    let leaf = leaf.clone();
    let outcome: Option<Result<(), ()>> = interp.with_child_path(parent, |a| {
        if a.child_exists(&leaf) {
            return Err(()); // already exists
        }
        a.create_child(Some(leaf.clone()));
        if safe {
            a.with_child(&leaf, |c| c.make_safe());
        }
        Ok(())
    });
    match outcome {
        Some(Ok(())) => {
            // Result is the path as written (the original list object).
            interp.set_result(obj::new_string_bytes(&obj_bytes(name_obj.unwrap())));
            Code::Ok
        }
        Some(Err(_)) => {
            let mut m = b"interpreter named \"".to_vec();
            m.extend_from_slice(&obj_bytes(name_obj.unwrap()));
            m.extend_from_slice(b"\" already exists, cannot create");
            interp.set_error(&m)
        }
        None => not_found_path(interp, parent),
    }
}

/// Parse an interp path object into its list of names (`{a b}` → `["a","b"]`).
fn interp_path(obj: *mut TclObj) -> Vec<Vec<u8>> {
    match crate::list::list_elements(obj) {
        Ok(els) => els.iter().map(|&e| obj_bytes(e)).collect(),
        Err(_) => {
            let b = obj_bytes(obj);
            if b.is_empty() {
                Vec::new()
            } else {
                vec![b]
            }
        }
    }
}

/// The `could not find interpreter "a b"` error for a path that failed to
/// resolve, rendering the path as a Tcl list.
fn not_found_path(interp: &mut Interp, path: &[Vec<u8>]) -> Code {
    let elems: Vec<*mut TclObj> = path.iter().map(|n| obj::new_string_bytes(n)).collect();
    let joined = crate::list::new_list_obj(&elems);
    let rendered = obj_bytes(joined);
    for e in elems {
        crate::interp::drop_fresh(e);
    }
    crate::interp::drop_fresh(joined);
    let mut m = b"could not find interpreter \"".to_vec();
    m.extend_from_slice(&rendered);
    m.push(b'"');
    interp.set_error(&m)
}

enum HideOp {
    Hide,
    Expose,
}

/// `interp limit path limitType ?-option value …?` — query/configure the
/// `commands` or `time` limit on a child interp.
fn interp_limit(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // argv = [interp, limit, path, limitType, opts…]
    if argv.len() < 4 {
        return wrong_args(interp, b"interp limit path limitType ?-option value ...?");
    }
    let path = interp_path(argv[2]);
    let ltype = obj_bytes(argv[3]);
    // Validate the limit type before the current-interp guard so a bad type is
    // reported ahead of the inaccessibility error (interp-35.3 vs .23).
    if ltype.as_slice() != b"commands" && ltype.as_slice() != b"time" {
        let mut m = b"bad limit type \"".to_vec();
        m.extend_from_slice(&ltype);
        m.extend_from_slice(b"\": must be commands or time");
        return interp.set_error(&m);
    }
    if path.is_empty() {
        return interp.set_error(b"limits on current interpreter inaccessible");
    }
    let opts: Vec<*mut TclObj> = argv[4..].to_vec();
    match interp.with_child_path(&path, |c| c.limit_apply(&ltype, &opts)) {
        Some(Ok(o)) => {
            interp.set_result(o);
            Code::Ok
        }
        Some(Err(m)) => interp.set_error(&m),
        None => not_found_path(interp, &path),
    }
}

/// `interp marktrusted path` — clear a child interp's safe flag (denied from a
/// safe interpreter).
fn interp_marktrusted(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"interp marktrusted path");
    }
    if interp.is_safe() {
        return interp.set_error(b"permission denied: safe interpreter cannot mark trusted");
    }
    let path = interp_path(argv[2]);
    if path.is_empty() {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    match interp.with_child_path(&path, |c| c.mark_trusted()) {
        Some(()) => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        None => not_found_path(interp, &path),
    }
}

/// `interp debug path ?-frame ?bool??` — the per-interp frame-debug switch.
fn interp_debug(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 || argv.len() > 5 {
        return wrong_args(interp, b"interp debug path ?-frame ?bool??");
    }
    let path = interp_path(argv[2]);
    let opts: Vec<*mut TclObj> = argv[3..].to_vec();
    match interp.with_child_path(&path, |c| c.debug_apply(&opts)) {
        Some(Ok(o)) => {
            interp.set_result(o);
            Code::Ok
        }
        Some(Err(m)) => interp.set_error(&m),
        None => not_found_path(interp, &path),
    }
}

/// Whether `name` resolves in the global namespace — it carries no namespace
/// qualifiers beyond an optional leading `::`.
fn is_global_command(name: &[u8]) -> bool {
    let body = name.strip_prefix(b"::".as_slice()).unwrap_or(name);
    !body.windows(2).any(|w| w == b"::")
}

/// `interp hide|expose path cmdName` — move a command into/out of the hidden
/// table of the named (or current, when path is `{}`) interpreter.
fn interp_hidectl(interp: &mut Interp, argv: &[*mut TclObj], op: HideOp) -> Code {
    // `interp hide   path cmdName     ?hiddenCmdName?`
    // `interp expose path hiddenName  ?cmdName?`
    if argv.len() != 4 && argv.len() != 5 {
        return match op {
            HideOp::Hide => wrong_args(interp, b"interp hide path cmdName ?hiddenCmdName?"),
            HideOp::Expose => wrong_args(interp, b"interp expose path hiddenCmdName ?cmdName?"),
        };
    }
    // A safe interpreter may not touch the hidden-command table of itself or
    // any of its children (the check is on the *executing* interp).
    if interp.is_safe() {
        return interp.set_error(match op {
            HideOp::Hide => b"permission denied: safe interpreter cannot hide commands",
            HideOp::Expose => b"permission denied: safe interpreter cannot expose commands",
        });
    }
    let path = interp_path(argv[2]);
    let cmd = obj_bytes(argv[3]);
    let token = if argv.len() == 5 {
        obj_bytes(argv[4])
    } else {
        cmd.clone()
    };
    // The hidden-command token may never carry namespace qualifiers, and only
    // global-namespace commands can be hidden / exposed-from.
    match op {
        HideOp::Hide => {
            if token.windows(2).any(|w| w == b"::") {
                return interp.set_error(
                    b"cannot use namespace qualifiers in hidden command token (rename)",
                );
            }
            if !is_global_command(&cmd) {
                return interp
                    .set_error(b"can only hide global namespace commands (use rename then hide)");
            }
        }
        HideOp::Expose => {
            if cmd.windows(2).any(|w| w == b"::") {
                return interp.set_error(
                    b"cannot use namespace qualifiers in hidden command token (rename)",
                );
            }
            if !is_global_command(&token) {
                return interp.set_error(
                    b"cannot expose to a namespace (use expose to toplevel, then rename)",
                );
            }
        }
    }
    // Hiding a missing command, or exposing a non-hidden one, is a silent
    // no-op (as is a child path that does not resolve), matching the prior
    // single-level behaviour.
    interp.with_child_path(&path, |c| match op {
        HideOp::Hide => c.hide_command(&cmd),
        HideOp::Expose => c.expose_command(&cmd),
    });
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `interp invokehidden path ?-opt ...? cmdName ?arg ...?` — invoke a hidden
/// command in the named (or current) interpreter.
fn interp_invokehidden(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"interp invokehidden path ?-opt? cmd ?arg ...?");
    }
    let path = interp_path(argv[2]);
    // Skip leading option flags (`-namespace ns`, `-global`).
    let mut i = 3;
    while i < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-global" => i += 1,
            b"-namespace" => i += 2,
            b"--" => {
                i += 1;
                break;
            }
            s if s.starts_with(b"-") => i += 1,
            _ => break,
        }
    }
    if i >= argv.len() {
        return wrong_args(interp, b"interp invokehidden path ?-opt? cmd ?arg ...?");
    }
    if interp.is_safe() {
        return interp.set_error(b"not allowed to invoke hidden commands from safe interpreter");
    }
    let cmd = obj_bytes(argv[i]);
    // Build the hidden command's argv (cmd + remaining args).
    let mut hidden_argv: Vec<*mut TclObj> = Vec::with_capacity(argv.len() - i);
    for &a in &argv[i..] {
        unsafe { obj::incr_ref_count(a) };
        hidden_argv.push(a);
    }
    // Run in the addressed interp (the current one for an empty path), copying
    // its result back up the path.
    let code = match interp.with_child_path(&path, |c| {
        (c.invoke_hidden(&cmd, &hidden_argv), c.result_bytes())
    }) {
        Some((code, res)) => {
            interp.set_result_bytes(&res);
            code
        }
        None => not_found_path(interp, &path),
    };
    for a in hidden_argv {
        unsafe { obj::decr_ref_count(a) };
    }
    code
}

/// `interp eval path arg ?arg ...?` — evaluate a script in a child interpreter.
fn interp_eval(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"interp eval path arg ?arg ...?");
    }
    let path = interp_path(argv[2]);
    let mut script = Vec::new();
    for (k, &a) in argv[3..].iter().enumerate() {
        if k > 0 {
            script.push(b' ');
        }
        script.extend_from_slice(&obj_bytes(a));
    }
    // `interp eval {} script` runs in the current interp; otherwise descend the
    // path to the target's parent and eval in the leaf child.
    let Some((leaf, parent)) = path.split_last() else {
        return interp.eval_str(&script);
    };
    let leaf = leaf.clone();
    match interp.with_child_path(parent, |a| {
        let code = a.eval_in_child(&leaf, &script);
        (code, a.result_bytes())
    }) {
        Some((code, result)) => {
            interp.set_result_bytes(&result);
            code
        }
        None => not_found_path(interp, &path),
    }
}

/// `interp delete ?path ...?` — delete each named child interpreter.
fn interp_delete(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    for &a in &argv[2..] {
        let path = interp_path(a);
        let Some((leaf, parent)) = path.split_last() else {
            return not_found_path(interp, &path);
        };
        let leaf = leaf.clone();
        match interp.with_child_path(parent, |p| p.delete_child(&leaf)) {
            Some(true) => {}
            _ => return not_found_path(interp, &path),
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `interp alias {} aliasName ?{} target ?arg ...??` — create / query / delete.
fn interp_alias(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // argv: interp alias srcPath aliasName ?targetPath target ?arg ...??
    if argv.len() < 4 {
        return wrong_args(
            interp,
            b"interp alias srcPath srcCmd ?targetPath targetCmd? ?arg ...?",
        );
    }
    let src = obj_bytes(argv[2]);
    let name = obj_bytes(argv[3]);

    // -- alias in a child interp, delegating to the parent (this interp) -------
    if !src.is_empty() {
        if !interp.child_exists(&src) {
            let mut m = b"could not find interpreter \"".to_vec();
            m.extend_from_slice(&src);
            m.push(b'"');
            return interp.set_error(&m);
        }
        if argv.len() == 4 {
            return interp.set_error(b"querying a child alias is not yet supported");
        }
        if !obj_bytes(argv[4]).is_empty() {
            // Only a `{}` target path (the parent) is supported.
            return only_single_interp(interp);
        }
        if argv.len() == 5 {
            interp.with_child(&src, |c| c.delete_command(&name));
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        let target = obj_bytes(argv[5]);
        let prefix: Vec<Vec<u8>> = argv[6..].iter().map(|&a| obj_bytes(a)).collect();
        interp.install_parent_alias(&src, &name, target, prefix);
        interp.set_result(obj::new_string_bytes(&name));
        return Code::Ok;
    }

    // -- alias in the current interp (single-interp) --------------------------
    // Query: `interp alias {} aliasName`.
    if argv.len() == 4 {
        return match interp.alias_info(&name) {
            Some((target, prefix)) => {
                set_alias_list(interp, &target, &prefix);
                Code::Ok
            }
            None => {
                let mut m = b"alias \"".to_vec();
                m.extend_from_slice(&name);
                m.extend_from_slice(b"\" not found");
                interp.set_error(&m)
            }
        };
    }

    if !obj_bytes(argv[4]).is_empty() {
        return only_single_interp(interp);
    }

    // Delete: `interp alias {} aliasName {}`.
    if argv.len() == 5 {
        interp.delete_command(&name);
        interp.set_result_bytes(b"");
        return Code::Ok;
    }

    // Create: `interp alias {} aliasName {} target ?arg ...?`.
    let target = obj_bytes(argv[5]);
    let prefix: Vec<Vec<u8>> = argv[6..].iter().map(|&a| obj_bytes(a)).collect();
    interp.install_alias(&name, target, prefix);
    interp.set_result(obj::new_string_bytes(&name));
    Code::Ok
}

/// `interp aliases ?path?` — every alias command's name in the named interp (the
/// current one for an empty/missing path) as a Tcl list.
fn interp_aliases(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return wrong_args(interp, b"interp aliases ?path?");
    }
    let path = argv.get(2).map(|&a| obj_bytes(a)).unwrap_or_default();
    let names = if path.is_empty() {
        interp.alias_names()
    } else {
        interp
            .with_child(&path, |c| c.alias_names())
            .unwrap_or_default()
    };
    let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
    interp.set_result(list::new_list_obj(&elems));
    for e in elems {
        drop_fresh(e);
    }
    Code::Ok
}

// -- helpers ---------------------------------------------------------------

fn only_single_interp(interp: &mut Interp) -> Code {
    interp.set_error(b"only single-interp aliases (empty interpreter paths) are supported")
}

/// Set the result to the `target ?arg ...?` list (the alias query form).
fn set_alias_list(interp: &mut Interp, target: &[u8], prefix: &[Vec<u8>]) {
    let mut elems: Vec<*mut TclObj> = Vec::with_capacity(prefix.len() + 1);
    elems.push(obj::new_string_bytes(target));
    for p in prefix {
        elems.push(obj::new_string_bytes(p));
    }
    interp.set_result(list::new_list_obj(&elems));
    for e in elems {
        drop_fresh(e);
    }
}

/// Free a freshly created (`rc 0`) object once `new_list_obj` has taken its own
/// reference.
fn drop_fresh(obj: *mut TclObj) {
    // SAFETY: `obj` is a live rc-0 object; retain-then-release frees it cleanly.
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(obj);
    }
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
    fn child_interpreters() {
        // `interp create`/`eval`/`exists`/`children`/`delete` + the child as a
        // command (`$child eval …`). Verified vs tclsh 9.0.
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
            assert_eq!(i.result_bytes(), b"kid");
            assert_eq!(i.eval_str(b"interp exists kid"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // The child is isolated and addressable as a command.
            i.eval_str(b"kid eval {set x 42; proc dbl n {expr {$n*2}}}");
            assert_eq!(i.eval_str(b"kid eval {dbl $x}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"84");
            // ... and via `interp eval`.
            assert_eq!(i.eval_str(b"interp eval kid {set x}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            // The parent doesn't see the child's variable.
            assert_eq!(i.eval_str(b"info exists x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"interp children"), Code::Ok);
            assert_eq!(i.result_bytes(), b"kid");
            // Children get the predefined globals.
            assert_eq!(
                i.eval_str(b"kid eval {set ::tcl_platform(platform)}"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"unix");
            // Delete removes the child and its command.
            assert_eq!(i.eval_str(b"interp delete kid"), Code::Ok);
            assert_eq!(i.eval_str(b"interp exists kid"), Code::Ok);
            assert_eq!(i.result_bytes(), b"0");
            assert_eq!(i.eval_str(b"kid eval {set x}"), Code::Error);
        });
    }

    #[test]
    fn cross_interp_aliases() {
        // A child alias delegating to a parent command (both syntaxes), and
        // re-entrancy (a parent alias re-entering the *same* child mid-eval —
        // the Safe Base pattern — now recurses correctly, bounded not forbidden).
        leak_free(|i| {
            i.eval_str(b"proc padd {a b} {expr {$a+$b}}");
            i.eval_str(b"set c [interp create]");
            // `interp alias $c name {} target prefix...`
            i.eval_str(b"interp alias $c add {} padd 100");
            assert_eq!(i.eval_str(b"$c eval {add 5}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"105");
            // `$c alias name target prefix...`
            i.eval_str(b"$c alias mul ::tcl::mathop::* 3");
            assert_eq!(i.eval_str(b"$c eval {mul 4}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"12");
            // Re-entrancy: a parent alias target that evals back into the same
            // child while its outer eval is still on the stack. This recurses
            // (the child's `x` ends up set), it does not error.
            i.eval_str(b"proc reenter {} { $::c eval {set x 42} }");
            i.eval_str(b"interp alias $c cb {} reenter");
            assert_eq!(i.eval_str(b"$c eval {cb}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"$c eval {set x}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            i.eval_str(b"interp delete $c; unset -nocomplain c");
        });
    }

    #[test]
    fn hidden_commands_and_safe() {
        // `interp hide`/`expose`/`invokehidden` + `interp create -safe`.
        leak_free(|i| {
            i.eval_str(b"set c [interp create]");
            i.eval_str(b"$c hide set");
            // hidden `set` is gone from the child but invocable via invokehidden.
            assert_eq!(i.eval_str(b"$c eval {set x 1}"), Code::Error);
            assert_eq!(i.eval_str(b"$c hidden"), Code::Ok);
            assert_eq!(i.result_bytes(), b"set");
            assert_eq!(i.eval_str(b"$c invokehidden set y 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            i.eval_str(b"$c expose set");
            assert_eq!(i.eval_str(b"$c eval {set z 9}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"9");
            // `-safe` hides the host-touching commands it has.
            i.eval_str(b"set s [interp create -safe]");
            assert_eq!(
                i.eval_str(b"expr {[lsearch [$s hidden] file] >= 0}"),
                Code::Ok
            );
            assert_eq!(i.result_bytes(), b"1");
            i.eval_str(b"interp delete $c; interp delete $s");
        });
    }

    #[test]
    fn rename_moves_a_command() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename set put"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            // `put` now works …
            assert_eq!(i.eval_str(b"put x 5"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            // … and `set` no longer resolves.
            assert_eq!(i.eval_str(b"set y 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
        });
    }

    #[test]
    fn rename_to_empty_deletes() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename incr {}"), Code::Ok);
            assert_eq!(i.eval_str(b"incr n"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"incr\"");
        });
    }

    #[test]
    fn rename_missing_is_an_error() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename nope gone"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't rename \"nope\": command doesn't exist"
            );
        });
    }

    /// C Tcl has no protected-command list for `rename`: even `return` may
    /// be renamed and used under its new name (tclsh 8.6.16 / 9.0.4:
    /// `rename ::return ::myreturn` succeeds).
    #[test]
    fn rename_builtin_return_is_allowed() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename return myreturn"), Code::Ok);
            assert_eq!(i.eval_str(b"proc p {} { myreturn done }"), Code::Ok);
            assert_eq!(i.eval_str(b"p"), Code::Ok);
            assert_eq!(i.result_bytes(), b"done");
            // Restore for any later tests in this interp.
            assert_eq!(i.eval_str(b"rename myreturn return"), Code::Ok);
        });
    }

    #[test]
    fn alias_create_and_dispatch() {
        leak_free(|i| {
            // `=` is an alias for `set`.
            assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
            assert_eq!(i.result_bytes(), b"=");
            assert_eq!(i.eval_str(b"= x 42"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"set y $x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
        });
    }

    #[test]
    fn alias_prepends_frozen_prefix() {
        leak_free(|i| {
            // `store` aliases `set k`, so `store v` runs `set k v`.
            assert_eq!(i.eval_str(b"interp alias {} store {} set k"), Code::Ok);
            assert_eq!(i.eval_str(b"store hello"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
            assert_eq!(i.eval_str(b"set out $k"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
        });
    }

    #[test]
    fn alias_query_returns_target_and_prefix() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} store {} set k");
            assert_eq!(i.eval_str(b"interp alias {} store"), Code::Ok);
            assert_eq!(i.result_bytes(), b"set k");
        });
    }

    #[test]
    fn alias_delete_unbinds() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} = {} set");
            assert_eq!(i.eval_str(b"interp alias {} = {}"), Code::Ok);
            assert_eq!(i.eval_str(b"= x 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"=\"");
        });
    }

    #[test]
    fn aliases_lists_every_alias() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} = {} set");
            i.eval_str(b"interp alias {} store {} set k");
            assert_eq!(i.eval_str(b"interp aliases {}"), Code::Ok);
            // BTreeMap order → sorted by name.
            assert_eq!(i.result_bytes(), b"= store");
        });
    }

    #[test]
    fn alias_to_deleted_target_errors_lazily() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} = {} set");
            // Deleting the target makes the alias fail at *its* next dispatch,
            // attributing the miss to the target name.
            assert_eq!(i.eval_str(b"rename set {}"), Code::Ok);
            assert_eq!(i.eval_str(b"= x 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
        });
    }

    #[test]
    fn alias_does_not_follow_target_rename() {
        leak_free(|i| {
            i.eval_str(b"interp alias {} = {} set");
            // Renaming the target out from under the alias: the stored name stops
            // resolving (C Tcl semantics — alias binds by name, not by command).
            assert_eq!(i.eval_str(b"rename set put"), Code::Ok);
            assert_eq!(i.eval_str(b"= x 1"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"set\"");
        });
    }
}
