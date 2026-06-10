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
        // Single-interp runtime: `issafe`/`exists ""` describe the one interp.
        b"issafe" => {
            interp.set_result_bytes(b"0");
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
/// its name (auto-generated `interpN` when omitted). `-safe` is accepted but the
/// sandbox is not yet enforced (a follow-up).
fn interp_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut name: Option<Vec<u8>> = None;
    let mut i = 2;
    while i < argv.len() {
        let a = obj_bytes(argv[i]);
        match a.as_slice() {
            b"-safe" => {}
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
    interp.set_result(obj::new_string_bytes(&created));
    Code::Ok
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
    if !obj_bytes(argv[2]).is_empty() {
        return only_single_interp(interp);
    }
    let name = obj_bytes(argv[3]);

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

/// `interp aliases ?{}?` — every alias command's name as a Tcl list.
fn interp_aliases(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() > 3 || (argv.len() == 3 && !obj_bytes(argv[2]).is_empty()) {
        return only_single_interp(interp);
    }
    let names = interp.alias_names();
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
