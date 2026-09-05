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
//! An alias whose source and target interpreter paths are both `{}` binds
//! within one interpreter; a child-side alias naming the parent binds as
//! `Command::ParentAlias`, dispatched through the parent `Weak` under
//! `CROSS_INTERP_DEPTH`. Querying a child alias, and a non-empty alias
//! *target* path, are not implemented.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{obj_bytes, Code, CommandVisibilityOp, Interp};
use crate::list;
use crate::namespace::RenameOutcome;
use crate::obj::{self, TclObj};

/// Register `rename` and `interp`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"rename", rename);
    interp.register_builtin(b"interp", interp_cmd);
    // `update` is registered by `cmd_event` (the real event loop).
}

// -- rename ----------------------------------------------------------------

/// `rename oldName newName` — move a command, or delete it when `newName` is the
/// empty string. Any command may be renamed, builtins included — C Tcl has no
/// protected list here (`rename ::return ::myreturn` succeeds on tclsh
/// 8.6.16 / 9.0.4).
fn rename(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"rename oldName newName");
    }
    let old = obj_bytes(argv[1]);
    let new = obj_bytes(argv[2]);
    match interp.rename_command(&old, &new) {
        RenameOutcome::Renamed | RenameOutcome::Deleted => {
            interp.set_result_bytes(b"");
            Code::Ok
        }
        RenameOutcome::NoSuchCommand => {
            // TclRenameCommand chooses the verb from the requested operation:
            // an empty destination is a deletion, while a non-empty
            // destination is a rename (tclNamesp.c).
            let verb = if new.is_empty() {
                b"can't delete \"".as_slice()
            } else {
                b"can't rename \"".as_slice()
            };
            let mut m = verb.to_vec();
            m.extend_from_slice(&old);
            m.extend_from_slice(b"\": command doesn't exist");
            interp.set_error(&m)
        }
        // C names the refused alias by the simple command name it would have
        // been bound under (`Tcl_GetCommandName`), not by the written path.
        RenameOutcome::AliasLoop => alias_loop_error(interp, &simple_tail(&new)),
        RenameOutcome::TargetExists => {
            let mut m = b"can't rename to \"".to_vec();
            m.extend_from_slice(&new);
            m.extend_from_slice(b"\": command already exists");
            interp.error_with_code(&m, b"TCL OPERATION RENAME TARGET_EXISTS")
        }
    }
}

/// The simple (unqualified) tail of a written command name — `::a::b` → `b`,
/// and the empty-string `{}` command for a name ending in a separator run
/// (#934), matching where the command table binds it.
fn simple_tail(name: &[u8]) -> Vec<u8> {
    if tcl_syntax::naming::ends_with_separator(name) {
        return Vec::new();
    }
    tcl_syntax::naming::qualifier_segments(name)
        .last()
        .map_or_else(Vec::new, |tail| (*tail).to_vec())
}

/// C's `TclPreventAliasLoop` refusal (`tclInterp.c`), shared by the `interp
/// alias` and `rename` gates.
fn alias_loop_error(interp: &mut Interp, simple: &[u8]) -> Code {
    let mut m = b"cannot define or rename alias \"".to_vec();
    m.extend_from_slice(simple);
    m.extend_from_slice(b"\": would create a loop");
    interp.error_with_code(&m, b"TCL OPERATION INTERP ALIASLOOP")
}

// -- interp ----------------------------------------------------------------

/// `interp`'s subcommand words, in C table order (`options[]`, `tclInterp.c`).
/// C resolves them with `Tcl_GetIndexFromObj(…, "option", 0)`, so `cr`
/// abbreviates `create` and the empty word — a prefix of every entry — is
/// `ambiguous option ""`.
///
/// The table names only the subcommands this runtime dispatches (issue #1412
/// item 3): `cancel`, `share`, and `transfer` need infrastructure it has none
/// of. `slaves` is 8.x's deprecated spelling of `children`: it still resolves
/// (as it does in C, whose `options[]` keeps it) but
/// [`interp_option_choices`] drops it from the 9.0 enumeration, exactly as C
/// reports its misses against `optionsNoSlaves[]`.
const INTERP_OPTIONS: &[&[u8]] = &[
    b"alias",
    b"aliases",
    b"bgerror",
    b"children",
    b"create",
    b"debug",
    b"delete",
    b"eval",
    b"exists",
    b"expose",
    b"hide",
    b"hidden",
    b"issafe",
    b"invokehidden",
    b"limit",
    b"marktrusted",
    b"recursionlimit",
    b"slaves",
    b"target",
];

/// The `interp` subcommands the miss message enumerates for the emulated
/// release: 9.0 retired `slaves` from the advertised list while still
/// dispatching it.
fn interp_option_choices(interp: &Interp) -> Vec<&'static [u8]> {
    if interp.runtime_version() >= tcl_dialect::TclVersion::V9_0 {
        INTERP_OPTIONS
            .iter()
            .copied()
            .filter(|name| *name != b"slaves")
            .collect()
    } else {
        INTERP_OPTIONS.to_vec()
    }
}

/// Resolve an `interp`-family subcommand word through the shared owner:
/// `dispatch` is the table the word may resolve against, `advertised` the
/// (possibly shorter) table the miss message enumerates — C splits the two the
/// same way for `slaves`.
pub(crate) fn resolve_interp_option(
    dispatch: &'static [&'static [u8]],
    advertised: &[&'static [u8]],
    word: &[u8],
) -> Result<&'static [u8], Vec<u8>> {
    match tcl_cmd_core::prefix::scan(dispatch, word, false) {
        tcl_cmd_core::prefix::Resolution::Exact(i)
        | tcl_cmd_core::prefix::Resolution::UniquePrefix(i) => Ok(dispatch[i]),
        miss => Err(tcl_cmd_core::prefix::bad_key_message(
            advertised,
            b"option",
            word,
            matches!(miss, tcl_cmd_core::prefix::Resolution::Ambiguous),
        )),
    }
}

/// The `interp` ensemble. `alias`, `aliases`, `create`, `delete`, `eval`,
/// `exists`, `hide`, `expose`, `hidden`, `invokehidden`, `issafe`,
/// `marktrusted`, `recursionlimit`, `bgerror`, `debug`, `limit`, and `target`
/// dispatch here. `cancel` (script cancellation), `share`, and `transfer`
/// (cross-interp channel sharing) are tclsh subcommands this runtime has no
/// infrastructure for — no cancellation flag on eval, no channel-table
/// sharing between interps — so, unlike `target`, implementing them is not
/// cheap; the bad-option list below advertises only what actually dispatches
/// here, rather than tclsh's full list (issue #1412 item 3).
fn interp_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"interp cmd ?arg ...?");
    }
    let choices = interp_option_choices(interp);
    let sub = match resolve_interp_option(INTERP_OPTIONS, &choices, &obj_bytes(argv[1])) {
        Ok(name) => name,
        Err(m) => return interp.set_error(&m),
    };
    match sub {
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
                return interp.wrong_args(b"interp bgerror path ?cmdPrefix?");
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
        b"hide" => interp_hidectl(interp, argv, CommandVisibilityOp::Hide),
        b"expose" => interp_hidectl(interp, argv, CommandVisibilityOp::Expose),
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
                return interp.wrong_args(b"interp recursionlimit path ?newlimit?");
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
        b"target" => interp_target(interp, argv),
        // Unreachable: every name in `INTERP_OPTIONS` has an arm above.
        other => {
            let mut m = b"bad option \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be ");
            m.extend_from_slice(&tcl_cmd_core::prefix::choice_list_bytes(&choices));
            interp.set_error(&m)
        }
    }
}

/// `interp target path alias` — the interp-path (from this interp) to the
/// target interpreter of `alias`, as installed in the interpreter addressed
/// by `path`. Cheap given the two alias shapes this runtime supports
/// (same-interp, or child-to-immediate-parent) — see
/// [`Interp::alias_target_path`]. `cancel`/`share`/`transfer` are the other
/// three subcommands tclsh advertises here that this runtime does not
/// implement; unlike `target` they need infrastructure (script cancellation,
/// cross-interp channel sharing) this runtime has none of (issue #1412 item 3).
fn interp_target(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"interp target path alias");
    }
    let path = interp_path(argv[2]);
    let alias = obj_bytes(argv[3]);
    match interp.alias_target_path(&path, &alias) {
        Some(target_path) => {
            let elems: Vec<*mut TclObj> = target_path
                .iter()
                .map(|n| obj::new_string_bytes(n))
                .collect();
            interp.set_result(list::new_list_obj(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        None => {
            let mut m = b"alias \"".to_vec();
            m.extend_from_slice(&alias);
            m.extend_from_slice(b"\" in path \"");
            m.extend_from_slice(&obj_bytes(argv[2]));
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

/// `interp invokehidden`'s leading option words (`hiddenOptions[]`,
/// `tclInterp.c`), resolved the same way: `-g`/`-n` abbreviate and the lone `-`
/// is `ambiguous option "-"`. Only a word starting with `-` reaches the table.
const HIDDEN_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &[b"-global", b"-namespace", b"--"]);

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
            match CREATE_OPTIONS.index_of(&a) {
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
                    return interp.set_error(&m);
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

/// `interp limit path limitType ?-option value …?` — query/configure the
/// `commands` or `time` limit on a child interp.
fn interp_limit(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // argv = [interp, limit, path, limitType, opts…]
    if argv.len() < 4 {
        return interp.wrong_args(b"interp limit path limitType ?-option value ...?");
    }
    let path = interp_path(argv[2]);
    let ltype = obj_bytes(argv[3]);
    // Validate the limit type before the current-interp guard so a bad type is
    // reported ahead of the inaccessibility error (interp-35.3 vs .23).
    if let Err(m) = crate::interp::LIMIT_TYPES.index_of(&ltype) {
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
        return interp.wrong_args(b"interp marktrusted path");
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
        return interp.wrong_args(b"interp debug path ?-frame ?bool??");
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
fn interp_hidectl(interp: &mut Interp, argv: &[*mut TclObj], op: CommandVisibilityOp) -> Code {
    // `interp hide   path cmdName     ?hiddenCmdName?`
    // `interp expose path hiddenName  ?cmdName?`
    if argv.len() != 4 && argv.len() != 5 {
        return match op {
            CommandVisibilityOp::Hide => {
                interp.wrong_args(b"interp hide path cmdName ?hiddenCmdName?")
            }
            CommandVisibilityOp::Expose => {
                interp.wrong_args(b"interp expose path hiddenCmdName ?cmdName?")
            }
        };
    }
    // A safe interpreter may not touch the hidden-command table of itself or
    // any of its children (the check is on the *executing* interp).
    if interp.is_safe() {
        return interp.set_error(match op {
            CommandVisibilityOp::Hide => {
                b"permission denied: safe interpreter cannot hide commands"
            }
            CommandVisibilityOp::Expose => {
                b"permission denied: safe interpreter cannot expose commands"
            }
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
        CommandVisibilityOp::Hide => {
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
        CommandVisibilityOp::Expose => {
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
    // Tcl's `Tcl_HideCommand` reports a missing source as `unknown command`;
    // retaining the old silent no-op swallowed typos in security-sensitive
    // command-surface setup. Exposure remains a separate operation with its
    // existing hidden-command error behaviour.
    let moved = interp.with_child_path(&path, |c| match op {
        CommandVisibilityOp::Hide => c.hide_command(&cmd, &token),
        CommandVisibilityOp::Expose => c.expose_command(&cmd, &token),
    });
    let Some(moved) = moved else {
        return not_found_path(interp, &path);
    };
    interp.finish_command_visibility(op, &cmd, &token, moved)
}

/// `interp invokehidden path ?-namespace ns? ?-global? ?--? cmdName ?arg
/// ...?` — invoke a hidden command in the named (or current) interpreter, in
/// the `-namespace`/`-global` evaluation context when given.
///
/// C's `ChildInvokeHidden` (`tclInterp.c`) takes the *last* of `-global`
/// (`::`) / `-namespace ns` given, not a mutual-exclusion refusal — passing
/// both is legal on tclsh 8.6.16/9.0.4, the last one simply wins (issue
/// #1412's own item 5 claimed a `cannot use -global option and -namespace
/// option together` error exists; it does not, on either release). An
/// unrecognized option is a hard `bad option` error rather than the previous
/// silent skip. `-namespace`'s namespace is resolved from the **global**
/// namespace regardless of the caller's current one, matching
/// `TCL_GLOBAL_ONLY` (tclsh-pinned: `-namespace bar` from inside `::foo`
/// still names `::bar`, not `::foo::bar`).
///
/// Simplification: unlike C's `Tcl_GetIndexFromObj`, this does not accept an
/// *abbreviated* option name (`-g` for `-global`) — only the three exact
/// spellings.
fn interp_invokehidden(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"interp invokehidden path ?-namespace ns? ?-global? ?--? cmd ?arg ..?";
    if argv.len() < 4 {
        return interp.wrong_args(USAGE);
    }
    let path = interp_path(argv[2]);
    let mut ns_name: Option<Vec<u8>> = None;
    let mut i = 3;
    while i < argv.len() {
        let opt = obj_bytes(argv[i]);
        if opt.first() != Some(&b'-') {
            break;
        }
        match HIDDEN_OPTIONS.index_of(&opt) {
            Ok(0) => {
                ns_name = Some(b"::".to_vec());
                i += 1;
            }
            Ok(1) => {
                i += 1;
                if i == argv.len() {
                    // C: "there must be more arguments" — stop scanning
                    // options and fall through to the arg-count check below.
                    break;
                }
                ns_name = Some(obj_bytes(argv[i]));
                i += 1;
            }
            Ok(_) => {
                i += 1;
                break;
            }
            Err(m) => return interp.set_error(&m),
        }
    }
    if i >= argv.len() {
        return interp.wrong_args(USAGE);
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
    // Run in the addressed interp (the current one for an empty path), in the
    // requested namespace context if any, copying its result back up the path.
    let code = match interp.with_child_path(&path, |c| {
        let saved_ns = c.current_ns();
        if let Some(name) = &ns_name {
            let target = c.ensure_global_namespace(name);
            c.set_current_ns(target);
        }
        let result = (c.invoke_hidden(&cmd, &hidden_argv), c.result_bytes());
        c.set_current_ns(saved_ns);
        result
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
        return interp.wrong_args(b"interp eval path arg ?arg ...?");
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
        return interp.wrong_args(b"interp alias srcPath srcCmd ?targetPath targetCmd? ?arg ...?");
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
    match interp.install_alias(&name, target, prefix) {
        Ok(()) => {
            interp.set_result(obj::new_string_bytes(&name));
            Code::Ok
        }
        Err(simple) => alias_loop_error(interp, &simple),
    }
}

/// `interp aliases ?path?` — every alias command's name in the named interp (the
/// current one for an empty/missing path) as a Tcl list.
fn interp_aliases(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(b"interp aliases ?path?");
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

    /// Issue #1607: the `interp` ensemble and the child-as-command dispatch
    /// are `Tcl_GetIndexFromObj(…, "option", 0)` tables (`options[]` in
    /// `Tcl_InterpObjCmd` and `NRChildCmd`, `tclInterp.c`), so subcommands
    /// abbreviate and the empty word — a prefix of every entry — is
    /// `ambiguous option ""`. The `interp` list still names only what this
    /// runtime dispatches (#1412 item 3); the child list is tclsh's in full.
    ///
    /// tclsh 8.6.16 / 9.0.4 (the verdicts, not the shortened `interp` list):
    ///   interp {}       -> ambiguous option "": must be …
    ///   interp c j      -> ambiguous option "c": must be …
    ///   interp cr j     -> j
    ///   interp e {}     -> ambiguous option "e": must be …
    ///   interp sl       -> the children list (8.x's deprecated spelling)
    ///   kid ev {set x 1} -> 1   ;  kid h / kid hi -> ambiguous option "h" / "hi"
    ///   kid x           -> bad option "x": must be alias, aliases, bgerror,
    ///                      debug, eval, expose, hide, hidden, issafe,
    ///                      invokehidden, limit, marktrusted, or recursionlimit
    #[test]
    fn interp_subcommand_words_resolve_like_tcl_get_index_from_obj() {
        const MUST: &str = "must be alias, aliases, bgerror, children, create, debug, \
                            delete, eval, exists, expose, hide, hidden, issafe, \
                            invokehidden, limit, marktrusted, recursionlimit, or target";
        const CHILD_MUST: &str = "must be alias, aliases, bgerror, debug, eval, expose, \
                                  hide, hidden, issafe, invokehidden, limit, marktrusted, \
                                  or recursionlimit";
        leak_free(|i| {
            let err = |i: &mut Interp, script: &[u8]| {
                assert_eq!(i.eval_str(script), Code::Error, "expected an error");
                String::from_utf8_lossy(&i.result_bytes()).into_owned()
            };
            assert_eq!(
                err(i, b"interp {}"),
                format!("ambiguous option \"\": {MUST}")
            );
            assert_eq!(err(i, b"interp x"), format!("bad option \"x\": {MUST}"));
            assert_eq!(
                err(i, b"interp c j"),
                format!("ambiguous option \"c\": {MUST}")
            );
            assert_eq!(i.eval_str(b"interp cr j"), Code::Ok);
            assert_eq!(i.result_bytes(), b"j");
            assert_eq!(
                err(i, b"interp e {set x 1}"),
                format!("ambiguous option \"e\": {MUST}")
            );
            assert_eq!(i.eval_str(b"interp ev {} {set x 1}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // 8.x's deprecated `slaves` spelling still resolves and dispatches.
            assert_eq!(i.eval_str(b"llength [interp sl]"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
            // The child-as-command table.
            assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
            assert_eq!(err(i, b"kid x"), format!("bad option \"x\": {CHILD_MUST}"));
            assert_eq!(
                err(i, b"kid {}"),
                format!("ambiguous option \"\": {CHILD_MUST}")
            );
            assert_eq!(
                err(i, b"kid h"),
                format!("ambiguous option \"h\": {CHILD_MUST}")
            );
            assert_eq!(
                err(i, b"kid hi"),
                format!("ambiguous option \"hi\": {CHILD_MUST}")
            );
            assert_eq!(i.eval_str(b"kid ev {set x 1}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"1");
        });
    }

    /// Issue #1607: `interp create`'s and `interp invokehidden`'s leading
    /// options are `Tcl_GetIndexFromObj(…, "option", 0)` tables
    /// (`createOptions[]` / `hiddenOptions[]`, `tclInterp.c`), so they
    /// abbreviate and the lone `-` — a prefix of every entry — is `ambiguous`.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   interp create -x k         -> bad option "-x": must be -safe or --
    ///   interp create - k          -> ambiguous option "-": must be -safe or --
    ///   interp create -s k         -> k
    ///   interp invokehidden i -x f -> bad option "-x": must be -global, -namespace, or --
    ///   interp invokehidden i - f  -> ambiguous option "-": must be -global, -namespace, or --
    #[test]
    fn interp_create_and_invokehidden_options_resolve_like_tcl_get_index_from_obj() {
        const CREATE_MUST: &str = "must be -safe or --";
        const HIDDEN_MUST: &str = "must be -global, -namespace, or --";
        leak_free(|i| {
            let err = |i: &mut Interp, script: &[u8]| {
                assert_eq!(i.eval_str(script), Code::Error, "expected an error");
                String::from_utf8_lossy(&i.result_bytes()).into_owned()
            };
            assert_eq!(
                err(i, b"interp create -x k"),
                format!("bad option \"-x\": {CREATE_MUST}")
            );
            assert_eq!(
                err(i, b"interp create - k"),
                format!("ambiguous option \"-\": {CREATE_MUST}")
            );
            assert_eq!(i.eval_str(b"interp create -s k"), Code::Ok);
            assert_eq!(i.result_bytes(), b"k");
            assert_eq!(i.eval_str(b"interp create -- k2"), Code::Ok);
            assert_eq!(i.result_bytes(), b"k2");
            assert_eq!(i.eval_str(b"interp create kid"), Code::Ok);
            assert_eq!(
                err(i, b"interp invokehidden kid -x foo"),
                format!("bad option \"-x\": {HIDDEN_MUST}")
            );
            assert_eq!(
                err(i, b"interp invokehidden kid - foo"),
                format!("ambiguous option \"-\": {HIDDEN_MUST}")
            );
        });
    }

    // Needs the numeric tower: the child's `dbl` proc computes via `expr`.
    #[cfg(have_tommath)]
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

    // Needs the numeric tower: alias targets are `expr`-backed (`padd`) and `::tcl::mathop::*`.
    #[cfg(have_tommath)]
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

    // Needs the numeric tower: the `-safe` hidden-list assert compares via `expr`.
    #[cfg(have_tommath)]
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
            assert_eq!(i.eval_str(b"rename nope {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't delete \"nope\": command doesn't exist"
            );
        });
    }

    #[test]
    fn rename_and_hide_error_shapes_and_lifecycle() {
        leak_free(|i| {
            // The verb is selected by the empty destination, including for a
            // qualified missing name (TclRenameCommand / tclNamesp.c).
            assert_eq!(i.eval_str(b"rename ::missing {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't delete \"::missing\": command doesn't exist"
            );

            // Qualified moves preserve the command and resolve from the new
            // namespace; deletion uses the same canonical rename seam.
            assert_eq!(
                i.eval_str(
                    b"namespace eval ::rename_ns {proc p {} {return qualified}}; rename ::rename_ns::p ::rename_ns::q"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"::rename_ns::q"), Code::Ok);
            assert_eq!(i.result_bytes(), b"qualified");
            assert_eq!(i.eval_str(b"rename ::rename_ns::q {}"), Code::Ok);
            assert_eq!(i.eval_str(b"::rename_ns::q"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"::rename_ns::q\"");

            // Hiding an absent command is an error, so repeated hide cannot
            // silently swallow a typo in a security-sensitive setup.
            assert_eq!(i.eval_str(b"interp hide {} nosuchcmd"), Code::Error);
            assert_eq!(i.result_bytes(), b"unknown command \"nosuchcmd\"");
            assert_eq!(
                i.eval_str(b"proc visible {} {return visible}; interp hide {} visible"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"interp hidden {}"), Code::Ok);
            assert_eq!(i.result_bytes(), b"visible");
            assert_eq!(i.eval_str(b"interp hide {} visible"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"hidden command named \"visible\" already exists"
            );
            assert_eq!(i.eval_str(b"interp invokehidden {} visible"), Code::Ok);
            assert_eq!(i.result_bytes(), b"visible");
            assert_eq!(i.eval_str(b"interp expose {} visible"), Code::Ok);
            assert_eq!(i.eval_str(b"visible"), Code::Ok);
            assert_eq!(i.result_bytes(), b"visible");

            // Destination conflicts are diagnosed before either table is
            // mutated. Both the source command and existing destination keep
            // their identities and remain callable.
            assert_eq!(
                i.eval_str(
                    b"proc p {} {return P}
                      interp hide {} p held
                      proc q {} {return Q}
                      set c [catch {interp hide {} q held} m o]
                      list $c $m [dict get $o -errorcode] [q] \
                           [interp invokehidden {} held] [interp hidden {}]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} Q P held"
            );

            assert_eq!(
                i.eval_str(
                    b"proc E {} {return E}
                      set c [catch {interp expose {} held E} m o]
                      list $c $m [dict get $o -errorcode] [E] \
                           [interp invokehidden {} held] [interp hidden {}]"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"1 {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS} E P held"
            );

            // Release-hidden builtins are absent from both rename and hide;
            // neither operation may make an 8.5+ command callable in 8.4.
            i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
            assert_eq!(i.eval_str(b"rename lassign {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't delete \"lassign\": command doesn't exist"
            );
            assert_eq!(i.eval_str(b"interp hide {} lassign"), Code::Error);
            assert_eq!(i.result_bytes(), b"unknown command \"lassign\"");
        });
    }

    #[test]
    fn visibility_failures_match_direct_and_child_oracles() {
        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set out {}
                      foreach script {
                          {interp hide {} nosuch}
                          {interp expose {} nosuch}
                      } {
                          catch $script m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      proc held {} {return OLD}
                      interp hide {} held
                      proc held {} {return NEW}
                      catch {interp hide {} held} m o
                      lappend out $m [dict get $o -errorcode]
                      proc E {} {return OLD}
                      interp hide {} E
                      proc E {} {return NEW}
                      catch {interp expose {} E} m o
                      lappend out $m [dict get $o -errorcode]
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown command \"nosuch\"} {TCL LOOKUP COMMAND nosuch} \
                  {unknown hidden command \"nosuch\"} \
                  {TCL LOOKUP HIDDENTOKEN nosuch} \
                  {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} \
                  {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS}"
            );
        });

        leak_free(|i| {
            assert_eq!(
                i.eval_str(
                    b"set c [interp create]
                      set out {}
                      foreach subcommand {hide expose} {
                          catch [list $c $subcommand nosuch] m o
                          lappend out $m [dict get $o -errorcode]
                      }
                      $c eval {proc held {} {return OLD}}
                      $c hide held
                      $c eval {proc held {} {return NEW}}
                      catch [list $c hide held] m o
                      lappend out $m [dict get $o -errorcode]
                      $c eval {proc E {} {return OLD}}
                      $c hide E
                      $c eval {proc E {} {return NEW}}
                      catch [list $c expose E] m o
                      lappend out $m [dict get $o -errorcode]
                      set out"
                ),
                Code::Ok
            );
            assert_eq!(
                i.result_bytes(),
                b"{unknown command \"nosuch\"} {TCL LOOKUP COMMAND nosuch} \
                  {unknown hidden command \"nosuch\"} \
                  {TCL LOOKUP HIDDENTOKEN nosuch} \
                  {hidden command named \"held\" already exists} \
                  {TCL HIDE ALREADY_HIDDEN} \
                  {exposed command \"E\" already exists} \
                  {TCL EXPOSE COMMAND_EXISTS}"
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

    /// C's `TclPreventAliasLoop` (`tclInterp.c`) refuses the alias that closes
    /// a cycle at *definition* time — the pair installs fine in neither order
    /// (tclsh 8.6.16 / 9.0.4-pinned wording, shared with the VM's
    /// `cross_interp_alias_e2e` vectors).
    #[test]
    fn a_mutual_alias_pair_is_refused_at_the_closing_alias() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} a {} b"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} b {} a"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"b\": would create a loop"
            );
            // The refused alias is not left behind …
            assert_eq!(i.eval_str(b"info commands b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
            // … so the surviving one still just misses its (absent) target
            // instead of recursing without bound.
            assert_eq!(i.eval_str(b"a"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"b\"");
        });
    }

    /// A self-alias is refused too — and, as in C, the command it displaced is
    /// *not* restored (the alias is created first, then unbound on the loop).
    #[test]
    fn a_self_alias_is_refused_and_destroys_the_command_it_displaced() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"proc x {} {return REAL}"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} x {} x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"x\": would create a loop"
            );
            assert_eq!(i.eval_str(b"x"), Code::Error);
            assert_eq!(i.result_bytes(), b"invalid command name \"x\"");
            // A qualified self-spelling is caught by *resolution*, not by a
            // string compare of the two names.
            assert_eq!(i.eval_str(b"interp alias {} q {} ::q"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"q\": would create a loop"
            );
        });
    }

    /// The walk follows the whole chain, not just one hop, and a frozen prefix
    /// on the closing alias does not hide the cycle.
    #[test]
    fn a_longer_alias_cycle_is_refused_at_its_closing_hop() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} c {} d"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} d {} e"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} e {} c extra"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"e\": would create a loop"
            );
            // A namespaced cycle is refused the same way, named by the alias's
            // simple command name (C's `Tcl_GetCommandName`), not its path.
            assert_eq!(i.eval_str(b"namespace eval ns {}"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} ns::p {} ns::q"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} ns::q {} ns::p"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"q\": would create a loop"
            );
        });
    }

    /// C guards `rename` with the same check: moving an alias onto a name its
    /// own target chain resolves back to is refused, and the table is left
    /// exactly as it was.
    #[test]
    fn a_rename_that_would_close_a_loop_is_refused_and_rolled_back() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} a {} b"), Code::Ok);
            assert_eq!(i.eval_str(b"rename a b"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"cannot define or rename alias \"b\": would create a loop"
            );
            assert_eq!(i.eval_str(b"info commands a"), Code::Ok);
            assert_eq!(i.result_bytes(), b"a");
            assert_eq!(i.eval_str(b"info commands b"), Code::Ok);
            assert_eq!(i.result_bytes(), b"");
        });
    }

    /// The refused rename must not damage the command sitting at the
    /// destination either: the tentative move the gate makes is undone
    /// completely. (tclsh refuses this one earlier still, with `can't rename
    /// to "bb": command already exists` — this runtime's `rename` has no
    /// destination-exists check, so it reports the loop instead; the surviving
    /// table state is the same either way.)
    #[test]
    fn a_refused_rename_leaves_the_destination_command_intact() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"proc bb {} {return REAL}"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} aa {} bb"), Code::Ok);
            assert_eq!(i.eval_str(b"rename aa bb"), Code::Error);
            assert_eq!(i.eval_str(b"bb"), Code::Ok);
            assert_eq!(i.result_bytes(), b"REAL");
            assert_eq!(i.eval_str(b"aa"), Code::Ok);
            assert_eq!(i.result_bytes(), b"REAL");
        });
    }

    /// The gate refuses cycles only: a legitimate alias-of-alias chain still
    /// dispatches, and an alias to a target that does not exist yet is legal
    /// (aliases late-bind).
    #[test]
    fn legitimate_alias_chains_and_late_binding_still_work() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"interp alias {} = {} set"), Code::Ok);
            assert_eq!(i.eval_str(b"interp alias {} := {} ="), Code::Ok);
            assert_eq!(i.eval_str(b":= zz 42"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"set out $zz"), Code::Ok);
            assert_eq!(i.result_bytes(), b"42");
            assert_eq!(i.eval_str(b"interp alias {} lb {} no_such_yet"), Code::Ok);
            assert_eq!(i.eval_str(b"proc no_such_yet {} {return LATE}"), Code::Ok);
            assert_eq!(i.eval_str(b"lb"), Code::Ok);
            assert_eq!(i.result_bytes(), b"LATE");
            // Renaming a non-alias, and renaming an alias somewhere harmless,
            // are both unaffected by the gate.
            assert_eq!(i.eval_str(b"rename := assign"), Code::Ok);
            assert_eq!(i.eval_str(b"assign zz 7"), Code::Ok);
            assert_eq!(i.result_bytes(), b"7");
        });
    }

    /// #934 definition-direction parity: a written trailing separator names
    /// the empty-string `{}` command inside its full qualifier chain — for
    /// `proc`, `rename`'s NEW name, and dispatch alike (`proc x:: {} {…}`
    /// defines `::x::` and `x::` invokes it; `rename foo x::` / `rename bar
    /// ::` rebind the `{}` command — all tclsh 8.6.16/9.0.4-pinned).
    /// Previously the definition split dropped the empty tail, so the proc
    /// just defined could not be invoked.
    #[test]
    fn trailing_separator_definitions_match_resolution() {
        leak_free(|i| {
            // proc with a trailing separator: defined AND callable.
            assert_eq!(
                i.eval_str(b"namespace eval x {}; proc x:: {} { return EMPTYTAIL }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"x::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"EMPTYTAIL");
            // … and reachable via any separator-run spelling.
            assert_eq!(i.eval_str(b"::x::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"EMPTYTAIL");
            // rename TO a trailing-separator name binds the `{}` command.
            assert_eq!(
                i.eval_str(b"proc foo {} { return F }; rename foo y::"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"y::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"F");
            // rename to `::` binds the GLOBAL `{}` command.
            assert_eq!(
                i.eval_str(b"proc bar {} { return B }; rename bar ::"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"B");
            // The `{}` command can be renamed AWAY from its spelling too.
            assert_eq!(i.eval_str(b"rename y:: plain"), Code::Ok);
            assert_eq!(i.eval_str(b"plain"), Code::Ok);
            assert_eq!(i.result_bytes(), b"F");
            assert_eq!(i.eval_str(b"y::"), Code::Error);
            // Variables share the rule: `set vx:: V` writes the `{}` variable
            // in `::vx` (tclsh: `info vars ::vx::*` lists `::vx::`).
            assert_eq!(i.eval_str(b"namespace eval vx {}; set vx:: V"), Code::Ok);
            assert_eq!(i.eval_str(b"set vx::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"V");
            // And ensembles: `namespace ensemble create -command ::e::` binds
            // the `{}` command in `::e` (tclsh-pinned: `e:: go` dispatches).
            assert_eq!(
                i.eval_str(
                    b"namespace eval e { proc go {} {return GO}; namespace export go; \
                       namespace ensemble create -command ::e:: }"
                ),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"e:: go"), Code::Ok);
            assert_eq!(i.result_bytes(), b"GO");
            // A proc named `:` inside a namespace named `:` has NO absolute
            // spelling (W314's case) but IS reachable by relative dispatch
            // from inside its namespace — `namespace eval : { : }` and
            // `namespace inscope : :` both invoke it, while every absolute
            // spelling misses (tclsh 8.6.16/9.0.4-pinned).
            assert_eq!(
                i.eval_str(b"namespace eval : { proc : args { return hello } }"),
                Code::Ok
            );
            assert_eq!(i.eval_str(b"namespace eval : { : }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
            assert_eq!(i.eval_str(b"namespace inscope : :"), Code::Ok);
            assert_eq!(i.result_bytes(), b"hello");
            // Every all-colon spelling is separator runs around an empty
            // tail — it can only ever reach the GLOBAL `{}` command (bound
            // above by `rename bar ::`), never the `:`-named proc
            // (tclsh-pinned: prints B here, and errors once no global `{}`
            // exists).
            assert_eq!(i.eval_str(b":::::"), Code::Ok);
            assert_eq!(i.result_bytes(), b"B");
        });
    }
}
