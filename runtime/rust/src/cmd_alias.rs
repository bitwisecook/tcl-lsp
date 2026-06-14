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

/// Commands that may not be renamed (mirrors Tcl 9's `TclProtectedCommandsList`
/// spirit without trace machinery — see `rename-alias.md` §3.4).
const PROTECTED: &[&[u8]] = &[b"return", b"error"];

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

// -- rename ----------------------------------------------------------------

/// `rename oldName newName` — move a command, or delete it when `newName` is the
/// empty string. Built-in commands on the protected list are refused.
fn rename(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"rename oldName newName");
    }
    let old = obj_bytes(argv[1]);
    let new = obj_bytes(argv[2]);
    if PROTECTED.contains(&old.as_slice()) {
        let mut m = b"can't rename \"".to_vec();
        m.extend_from_slice(&old);
        m.extend_from_slice(b"\": built-in command");
        return interp.set_error(&m);
    }
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
            // named one exists iff it's a child.
            let exists = match argv.get(2) {
                None => true,
                Some(&a) => {
                    let p = obj_bytes(a);
                    p.is_empty() || interp.child_exists(&p)
                }
            };
            interp.set_result_bytes(if exists { b"1" } else { b"0" });
            Code::Ok
        }
        b"children" | b"slaves" => {
            // Children of the current interp (a named sub-path is single-level).
            let names = if argv
                .get(2)
                .map(|&a| obj_bytes(a))
                .is_some_and(|p| !p.is_empty())
            {
                Vec::new()
            } else {
                interp.child_names()
            };
            let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
            interp.set_result(list::new_list_obj(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        b"bgerror" => {
            // `interp bgerror path ?cmdPrefix?` — get/set the current interp's
            // background-error handler. Only the current interp ("") is modelled.
            if argv.len() < 3 || argv.len() > 4 {
                return wrong_args(interp, b"interp bgerror path ?cmdPrefix?");
            }
            match argv.get(3) {
                Some(&p) => {
                    interp.set_bgerror_handler(&obj_bytes(p));
                    interp.set_result_bytes(b"");
                }
                None => {
                    let h = interp.bgerror_handler();
                    interp.set_result_bytes(&h);
                }
            }
            Code::Ok
        }
        b"hide" => interp_hidectl(interp, argv, HideOp::Hide),
        b"expose" => interp_hidectl(interp, argv, HideOp::Expose),
        b"invokehidden" => interp_invokehidden(interp, argv),
        b"hidden" => {
            // `interp hidden ?path?` — hidden command names in the (current or
            // named) interp.
            let path = argv.get(2).map(|&a| obj_bytes(a)).unwrap_or_default();
            let names = if path.is_empty() {
                interp.hidden_names()
            } else {
                interp
                    .with_child(&path, |c| c.hidden_names())
                    .unwrap_or_default()
            };
            let elems: Vec<*mut TclObj> = names.iter().map(|n| obj::new_string_bytes(n)).collect();
            interp.set_result(list::new_list_obj(&elems));
            for e in elems {
                drop_fresh(e);
            }
            Code::Ok
        }
        b"issafe" => {
            // `interp issafe ?path?` — the current interp (no path) or a child.
            let path = argv.get(2).map(|&a| obj_bytes(a)).unwrap_or_default();
            let safe = if path.is_empty() {
                interp.is_safe()
            } else {
                interp.with_child(&path, |c| c.is_safe()).unwrap_or(false)
            };
            interp.set_result_bytes(if safe { b"1" } else { b"0" });
            Code::Ok
        }
        other => {
            let mut m = b"interp subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\" is not supported in this runtime");
            interp.set_error(&m)
        }
    }
}

/// `interp create ?-safe? ?--? ?path?` — create a child interpreter, returning
/// its name (auto-generated `interpN` when omitted). `-safe` hides the
/// host-touching commands (the Safe Base's re-aliasing is a follow-up).
fn interp_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut name: Option<Vec<u8>> = None;
    let mut safe = false;
    let mut i = 2;
    while i < argv.len() {
        let a = obj_bytes(argv[i]);
        match a.as_slice() {
            b"-safe" => safe = true,
            b"--" => {
                i += 1;
                break;
            }
            _ => break,
        }
        i += 1;
    }
    if i < argv.len() {
        let p = obj_bytes(argv[i]);
        // Only single-level (simple) names are supported; a path list is not.
        if interp.child_exists(&p) {
            let mut m = b"interpreter named \"".to_vec();
            m.extend_from_slice(&p);
            m.extend_from_slice(b"\" already exists, cannot create");
            return interp.set_error(&m);
        }
        name = Some(p);
    }
    let created = interp.create_child(name);
    if safe {
        interp.with_child(&created, |c| c.make_safe());
    }
    interp.set_result(obj::new_string_bytes(&created));
    Code::Ok
}

enum HideOp {
    Hide,
    Expose,
}

/// `interp hide|expose path cmdName` — move a command into/out of the hidden
/// table of the named (or current, when path is `{}`) interpreter.
fn interp_hidectl(interp: &mut Interp, argv: &[*mut TclObj], op: HideOp) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"interp hide|expose path cmdName");
    }
    let path = obj_bytes(argv[2]);
    let cmd = obj_bytes(argv[3]);
    let did = if path.is_empty() {
        match op {
            HideOp::Hide => interp.hide_command(&cmd),
            HideOp::Expose => interp.expose_command(&cmd),
        }
    } else {
        interp
            .with_child(&path, |c| match op {
                HideOp::Hide => c.hide_command(&cmd),
                HideOp::Expose => c.expose_command(&cmd),
            })
            .unwrap_or(false)
    };
    if !did {
        // Hiding a missing command, or exposing a non-hidden one.
        let _ = did;
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// `interp invokehidden path ?-opt ...? cmdName ?arg ...?` — invoke a hidden
/// command in the named (or current) interpreter.
fn interp_invokehidden(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"interp invokehidden path ?-opt? cmd ?arg ...?");
    }
    let path = obj_bytes(argv[2]);
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
    let cmd = obj_bytes(argv[i]);
    // Build the hidden command's argv (cmd + remaining args).
    let mut hidden_argv: Vec<*mut TclObj> = Vec::with_capacity(argv.len() - i);
    for &a in &argv[i..] {
        unsafe { obj::incr_ref_count(a) };
        hidden_argv.push(a);
    }
    let code = if path.is_empty() {
        interp.invoke_hidden(&cmd, &hidden_argv)
    } else {
        // Run in the child; copy its result back.
        match interp.with_child(&path, |c| {
            (c.invoke_hidden(&cmd, &hidden_argv), c.result_bytes())
        }) {
            Some((code, res)) => {
                interp.set_result_bytes(&res);
                code
            }
            None => interp.set_error(b"could not find interpreter"),
        }
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
    let path = obj_bytes(argv[2]);
    let mut script = Vec::new();
    for (k, &a) in argv[3..].iter().enumerate() {
        if k > 0 {
            script.push(b' ');
        }
        script.extend_from_slice(&obj_bytes(a));
    }
    // `interp eval {} script` runs in the current interp; else in the child.
    if path.is_empty() {
        interp.eval_str(&script)
    } else {
        interp.eval_in_child(&path, &script)
    }
}

/// `interp delete ?path ...?` — delete each named child interpreter.
fn interp_delete(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    for &a in &argv[2..] {
        let path = obj_bytes(a);
        if path.is_empty() || !interp.delete_child(&path) {
            let mut m = b"could not find interpreter \"".to_vec();
            m.extend_from_slice(&path);
            m.push(b'"');
            return interp.set_error(&m);
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

    #[test]
    fn rename_protected_is_refused() {
        leak_free(|i| {
            assert_eq!(i.eval_str(b"rename return ret"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"can't rename \"return\": built-in command"
            );
            // `return` still works.
            assert_eq!(i.eval_str(b"return done"), Code::Return);
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
