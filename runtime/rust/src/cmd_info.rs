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

//! `info` — interpreter introspection (toward running tcltest; used ~110× in
//! the library). C ref `tclCmdIL.c` (`Tcl_InfoObjCmd`).
//!
//! Reads **live** runtime state (the T-INFO contract: never compile-time-folded).
//! The implemented subset covers what the library leans on: `exists`,
//! `commands`/`procs`/`vars`/`globals`/`locals` (glob-filtered), `level`
//! (current depth + `level N`), `frame` (the source-location stack, PC-5),
//! `script`, `tclversion`/`patchlevel`, `nameofexecutable`, and proc
//! introspection `body`/`args`/`default`. `info errorstack` (TIP 348) is the
//! remaining `CmdFrame`-adjacent item.

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::list;
use crate::obj::TclObj;

/// Register `info`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"info", info_cmd);
}

fn info_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"info subcommand ?arg ...?");
    }
    // `info` is an ensemble: resolve an exact name, else an unambiguous prefix
    // (so `info command` → `commands`, matching tclsh).
    const SUBS: &[&[u8]] = &[
        b"args",
        b"body",
        b"class",
        b"cmdcount",
        b"cmdtype",
        b"commands",
        b"complete",
        b"constant",
        b"consts",
        b"coroutine",
        b"default",
        b"errorstack",
        b"exists",
        b"frame",
        b"functions",
        b"globals",
        b"hostname",
        b"level",
        b"library",
        b"loaded",
        b"locals",
        b"nameofexecutable",
        b"object",
        b"patchlevel",
        b"procs",
        b"script",
        b"sharedlibextension",
        b"tclversion",
        b"vars",
    ];
    let raw = obj_bytes(argv[1]);
    // The table is a release fact: `cmdtype`/`constant`/`consts` are Tcl 9,
    // `class`/`coroutine`/`errorstack`/`object` 8.6, `frame` 8.5. Filtered to
    // the emulated release, `info cm` is `cmdcount` on 8.6 and ambiguous with
    // `cmdtype` on 9.0, exactly as tclsh has it.
    let subs = crate::environment::release_subcommands(
        interp.runtime_version().dialect_profile_name(),
        "info",
        SUBS,
    );
    // A miss reports here rather than falling through with the raw word: the
    // arms below match on the canonical name, so a word the *pinned release*
    // does not have (`info cmdtype` under 8.6) would otherwise still dispatch.
    let Some(index) = tcl_cmd_core::ensemble::resolve_subcommand(subs, &raw, true) else {
        return interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
            subs,
            &raw,
            true,
            b"::tcl::info",
        ));
    };
    let sub: &[u8] = subs[index];
    match sub {
        b"exists" => info_exists(interp, argv),
        // commands/procs route through the shared namespace-aware core (over the
        // `Namespaces` enumeration rungs). This also fixed a real bug: `info procs`
        // in a namespace wrongly merged the global procs (the old
        // `visible_proc_names`); C's `InfoProcsCmd` lists the current namespace
        // only, while `info commands` does merge global.
        b"commands" => info_command_list(interp, argv, b"info commands ?pattern?", false),
        b"procs" => info_command_list(interp, argv, b"info procs ?pattern?", true),
        // vars/locals/globals route through the shared variable-listing cores
        // (namespace-aware over `Namespaces::vars_in` + the active-frame
        // `Frames::var_names`/`in_proc`); `globals` owns the Bug 1057461 leading-
        // `::` strip in the core.
        b"vars" => info_var_list(interp, argv, b"info vars ?pattern?", VarList::Vars),
        b"globals" => info_var_list(interp, argv, b"info globals ?pattern?", VarList::Globals),
        b"locals" => info_var_list(interp, argv, b"info locals ?pattern?", VarList::Locals),
        b"level" => info_level(interp, argv),
        b"frame" => info_frame(interp, argv),
        b"coroutine" => {
            if argv.len() != 2 {
                return interp.wrong_args(b"info coroutine");
            }
            interp.set_result_bytes(&crate::cmd_coro::current_coroutine());
            Code::Ok
        }
        b"object" => crate::cmd_oo::info_object(interp, argv),
        b"class" => crate::cmd_oo::info_class(interp, argv),
        // `info tclversion`/`patchlevel` *read* the globals (C reads
        // `tcl_version`/`tcl_patchLevel`), so unsetting them makes these error.
        b"tclversion" => info_global(interp, argv, b"info tclversion", b"tcl_version"),
        b"patchlevel" => info_global(interp, argv, b"info patchlevel", b"tcl_patchLevel"),
        // The host shared-library suffix (`$::tcl_platform(platform)` is unix).
        b"sharedlibextension" => fixed(
            interp,
            argv,
            b"info sharedlibextension",
            tcl_platform::bootstrap::SHARED_LIBRARY_EXTENSION.as_bytes(),
        ),
        b"complete" => {
            if argv.len() != 3 {
                return interp.wrong_args(b"info complete command");
            }
            let ok = tcl_cmd_core::info::complete(&obj_bytes(argv[2]));
            interp.set_result_bytes(if ok { b"1" } else { b"0" });
            Code::Ok
        }
        b"body" => info_body(interp, argv),
        b"args" => info_args(interp, argv),
        b"default" => info_default(interp, argv),
        b"cmdtype" => info_cmdtype(interp, argv),
        b"cmdcount" => {
            if argv.len() != 2 {
                return interp.wrong_args(b"info cmdcount");
            }
            interp.set_result(crate::obj::new_wide_int_obj(interp.cmd_count() as i64));
            Code::Ok
        }
        b"functions" => {
            if argv.len() > 3 {
                return interp.wrong_args(b"info functions ?pattern?");
            }
            let pat = argv.get(2).map(|&a| obj_bytes(a));
            set_filtered(interp, interp.mathfunc_names(), pat.as_deref())
        }
        b"loaded" => {
            if argv.len() > 4 {
                return interp.wrong_args(b"info loaded ?interp? ?prefix?");
            }
            // A named interp must resolve (C's `Tcl_GetChild` in
            // `TclGetLoadedLibraries`); the empty path is the current interp.
            // Nothing is ever loaded in this runtime, so the result is always
            // empty — but an unknown interp is still an error.
            if let Some(&a) = argv.get(2) {
                let path = obj_bytes(a);
                if !path.is_empty() && !interp.child_exists(&path) {
                    let mut m = b"could not find interpreter \"".to_vec();
                    m.extend_from_slice(&path);
                    m.push(b'"');
                    return interp.set_error(&m);
                }
            }
            interp.set_result_bytes(b"");
            Code::Ok
        }
        b"script" => {
            if argv.len() > 3 {
                return interp.wrong_args(b"info script ?filename?");
            }
            if argv.len() == 3 {
                // Set the current script name and return it (C's `info script F`).
                let name = obj_bytes(argv[2]);
                interp.set_current_script(&name);
                interp.set_result_bytes(&name);
            } else {
                interp.set_result_bytes(&interp.current_script());
            }
            Code::Ok
        }
        b"nameofexecutable" => {
            if argv.len() != 2 {
                return interp.wrong_args(b"info nameofexecutable");
            }
            let exe = interp.host().env().current_exe().unwrap_or_default();
            interp.set_result_bytes(exe.as_bytes());
            Code::Ok
        }
        b"library" => {
            if argv.len() != 2 {
                return interp.wrong_args(b"info library");
            }
            match interp.var_get(b"::tcl_library") {
                Some(o) => {
                    interp.set_result(o);
                    Code::Ok
                }
                None => interp.set_error(b"no library has been specified for Tcl"),
            }
        }
        b"hostname" => {
            if argv.len() != 2 {
                return interp.wrong_args(b"info hostname");
            }
            let h = hostname(interp);
            interp.set_result_bytes(&h);
            Code::Ok
        }
        b"errorstack" => {
            // TIP 348: the error stack of the last error (`?interp?` accepted but
            // only the current interp is supported).
            if argv.len() > 3 {
                return interp.wrong_args(b"info errorstack ?interp?");
            }
            let es = interp.error_stack_value();
            interp.set_result_bytes(&es);
            Code::Ok
        }
        b"constant" => {
            // TIP 677: whether `varName` resolves to a `const` scalar.
            if argv.len() != 3 {
                return interp.wrong_args(b"info constant varName");
            }
            let is_const = interp.is_constant(&obj_bytes(argv[2]));
            interp.set_result_bytes(if is_const { b"1" } else { b"0" });
            Code::Ok
        }
        b"consts" => set_list_qualified(
            interp,
            argv,
            b"info consts ?pattern?",
            Interp::consts_in_namespace,
            Interp::visible_const_names,
        ),
        other => interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
            SUBS,
            other,
            true,
            b"::tcl::info",
        )),
    }
}

/// The host name (`info hostname`, `Tcl_GetHostName`) — best-effort from
/// `/proc/sys/kernel/hostname`, else `/etc/hostname`, else empty. Read through
/// the host filesystem (empty when the host provides none).
fn hostname(interp: &Interp) -> Vec<u8> {
    let host = interp.host();
    let fs = host.filesystem();
    for path in ["/proc/sys/kernel/hostname", "/etc/hostname"] {
        if let Some(bytes) = fs.and_then(|fs| fs.read(path).ok()) {
            return String::from_utf8_lossy(&bytes).trim().as_bytes().to_vec();
        }
    }
    Vec::new()
}

/// `info cmdtype command` — the kind of command (`native`/`proc`/`alias`/…).
fn info_cmdtype(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"info cmdtype commandName");
    }
    let name = obj_bytes(argv[2]);
    match interp.cmdtype(&name) {
        Some(t) => {
            interp.set_result_bytes(t);
            Code::Ok
        }
        None => {
            let mut m = b"unknown command \"".to_vec();
            m.extend_from_slice(&name);
            m.push(b'"');
            interp.set_error(&m)
        }
    }
}

/// Set the result to a Tcl list of `names` filtered by an optional glob pattern.
fn set_filtered(interp: &mut Interp, names: Vec<Vec<u8>>, pattern: Option<&[u8]>) -> Code {
    let objs: Vec<*mut TclObj> = names
        .iter()
        .filter(|n| pattern.is_none_or(|p| glob_match(p, n)))
        .map(|n| new_string(n))
        .collect();
    let l = list::new_list_obj(&objs); // retains each element
    interp.set_result(l); // retains the list; the rc-0 temporaries are now owned by it
    Code::Ok
}

/// `info commands|procs|vars ?pattern?` — a namespace-qualified pattern
/// (`::ns::glob`) lists the matching names *in that namespace* (re-qualified);
/// an unqualified pattern filters the names visible from the current scope.
fn set_list_qualified(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    usage: &[u8],
    in_namespace: fn(&Interp, &[u8]) -> Vec<Vec<u8>>,
    visible: fn(&Interp) -> Vec<Vec<u8>>,
) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(usage);
    }
    let pattern = argv.get(2).map(|&a| obj_bytes(a));
    // Find the last `::` so a qualified pattern splits into (prefix, tail glob).
    let split = pattern.as_deref().and_then(|p| {
        p.windows(2)
            .rposition(|w| w == b"::")
            .map(|i| (&p[..i], &p[i + 2..]))
    });
    if let Some((prefix, tail)) = split {
        // Re-qualify through the namespace's canonical full name so results are
        // absolute even for a *relative* qualifier (`info commands ns::pat`).
        let Some(canon) = interp.canonical_ns_prefix(prefix) else {
            interp.set_result(list::new_list_obj(&[]));
            return Code::Ok;
        };
        let names = in_namespace(interp, prefix);
        let objs: Vec<*mut TclObj> = names
            .iter()
            .filter(|n| glob_match(tail, n))
            .map(|n| {
                let mut full = canon.clone();
                full.extend_from_slice(n);
                new_string(&full)
            })
            .collect();
        let l = list::new_list_obj(&objs);
        interp.set_result(l);
        return Code::Ok;
    }
    let list = visible(interp);
    set_filtered(interp, list, pattern.as_deref())
}

/// `info commands ?pattern?` / `info procs ?pattern?` — the shared
/// namespace-aware command/proc listing core (over the `Namespaces` enumeration
/// rungs); `procs_only` selects `procs`. The qualified-pattern split,
/// re-qualification, glob filter, and the global-merge asymmetry all live in the
/// core. (Variable listing — `vars`/`locals`/`globals`/`consts` — stays on
/// [`set_list_qualified`]: the VM has no namespace variables to share against.)
fn info_command_list(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    usage: &[u8],
    procs_only: bool,
) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(usage);
    }
    let result = tcl_cmd_core::info::command_list(interp, argv.get(2), procs_only);
    interp.set_result(result);
    Code::Ok
}

/// Which variable-listing core `info_var_list` dispatches to.
enum VarList {
    /// `info vars` — current context (frame locals in a proc, else namespace vars).
    Vars,
    /// `info locals` — the current proc frame's genuine locals.
    Locals,
    /// `info globals` — the global namespace's variables.
    Globals,
}

/// `info vars`/`locals`/`globals` — the shared variable-listing cores (over
/// `Namespaces::vars_in` + the active-frame `Frames` rungs). The whole
/// context/namespace/link logic lives in the core; this adapter only checks
/// arity and maps the result. (`consts` stays on [`set_list_qualified`] — TIP 677
/// is runtime-only.)
fn info_var_list(interp: &mut Interp, argv: &[*mut TclObj], usage: &[u8], which: VarList) -> Code {
    if argv.len() > 3 {
        return interp.wrong_args(usage);
    }
    let pattern = argv.get(2);
    let result = match which {
        VarList::Vars => tcl_cmd_core::info::vars(interp, pattern),
        VarList::Locals => tcl_cmd_core::info::locals(interp, pattern),
        VarList::Globals => tcl_cmd_core::info::globals(interp, pattern),
    };
    interp.set_result(result);
    Code::Ok
}

fn info_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"info exists varName");
    }
    // The shared Family-B core over `VarStore::exists`.
    let result = tcl_cmd_core::info::exists(interp, &argv[2]);
    interp.set_result(result);
    Code::Ok
}

fn info_level(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // The shared Family-B core over `Introspect` (`tcl_cmd_core::info::level`);
    // the runtime is a thin adapter mapping `Result<*mut TclObj, CmdError>` onto
    // its set-result/`Code` ABI. (`info level 0` is relative — the current call;
    // `N>0` absolute.)
    let number = match argv.len() {
        2 => None,
        3 => Some(&argv[2]),
        _ => return interp.wrong_args(b"info level ?number?"),
    };
    match tcl_cmd_core::info::level(interp, number) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => interp.set_error(e.message().as_bytes()),
    }
}

/// `info frame ?number?` — the source-location stack (PC-5). No arg: the stack
/// depth. With a number: a dict describing that frame (`type`/`line`/`cmd`,
/// `proc`/`file` where applicable, and `level`). C ref `tclCmdIL.c`
/// `InfoFrameCmd`; numbering matches `info level` (N>0 absolute, N<=0 relative).
fn info_frame(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        2 => {
            interp.set_result_bytes(interp.cmd_frame_depth().to_string().as_bytes());
            Code::Ok
        }
        3 => {
            let spec = obj_bytes(argv[2]);
            let n = match core::str::from_utf8(&spec)
                .ok()
                .and_then(|s| s.trim().parse::<i64>().ok())
            {
                Some(n) => n,
                None => {
                    let mut m = b"expected integer but got \"".to_vec();
                    m.extend_from_slice(&spec);
                    m.push(b'"');
                    return interp.set_error(&m);
                }
            };
            match interp.cmd_frame_info(n) {
                Some(pairs) => {
                    let kv: Vec<(*mut TclObj, *mut TclObj)> = pairs
                        .iter()
                        .map(|(k, v)| (new_string(k), new_string(v)))
                        .collect();
                    interp.set_result(crate::dict::new_dict_obj(&kv));
                    Code::Ok
                }
                None => bad_level(interp, &spec),
            }
        }
        _ => interp.wrong_args(b"info frame ?number?"),
    }
}

/// `Tcl_CommandComplete` (approximation): a script is complete when no brace,
/// bracket, or quote is left open and it doesn't end mid-escape. Inside braces
/// only `{`/`}` nest; inside quotes `[...]` command substitution still nests.
fn bad_level(interp: &mut Interp, spec: &[u8]) -> Code {
    let mut m = b"bad level \"".to_vec();
    m.extend_from_slice(spec);
    m.push(b'"');
    interp.set_error(&m)
}

fn fixed(interp: &mut Interp, argv: &[*mut TclObj], usage: &[u8], value: &[u8]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(usage);
    }
    interp.set_result_bytes(value);
    Code::Ok
}

/// `info tclversion`/`patchlevel` — read the **global** `var` (C uses
/// `TCL_GLOBAL_ONLY`), erroring `can't read "VAR": no such variable` if it has
/// been unset (info-14.3/18.3). `var` is the unqualified name (for the message);
/// the read is `::`-qualified so it works inside a proc/eval frame.
fn info_global(interp: &mut Interp, argv: &[*mut TclObj], usage: &[u8], var: &[u8]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(usage);
    }
    let mut qualified = b"::".to_vec();
    qualified.extend_from_slice(var);
    match interp.var_get(&qualified) {
        Some(o) => {
            interp.set_result(o);
            Code::Ok
        }
        None => {
            let mut m = b"can't read \"".to_vec();
            m.extend_from_slice(var);
            m.extend_from_slice(b"\": no such variable");
            interp.set_error(&m)
        }
    }
}

fn info_body(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"info body procname");
    }
    match tcl_cmd_core::info::body(interp, &argv[2]) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => interp.set_error(e.message().as_bytes()),
    }
}

fn info_args(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"info args procname");
    }
    match tcl_cmd_core::info::args(interp, &argv[2]) {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => interp.set_error(e.message().as_bytes()),
    }
}

fn info_default(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return interp.wrong_args(b"info default procname arg varname");
    }
    let var = obj_bytes(argv[4]);
    // The core computes the `(value, has_default)` pair (and the not-a-proc /
    // no-such-argument errors); the **store** stays here because it is
    // trace-aware — a write trace or array-typed target makes it fail with the
    // variable error verbatim (`can't set "a": variable is array`).
    let (val, has) = match tcl_cmd_core::info::default(interp, &argv[2], &argv[3]) {
        Ok(pair) => pair,
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };
    if let Err(e) = interp.var_set(&var, val) {
        crate::interp::drop_fresh(val);
        return crate::builtins::var_error(interp, &var, e);
    }
    interp.set_result_bytes(if has { b"1" } else { b"0" });
    Code::Ok
}

fn glob_match(pat: &[u8], name: &[u8]) -> bool {
    match (core::str::from_utf8(pat), core::str::from_utf8(name)) {
        (Ok(p), Ok(n)) => tcl_syntax::glob::string_match(p, n),
        _ => false,
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

    fn run(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    /// Issue #1607: `info` is a `TclMakeEnsemble` command, so both the prefix
    /// scan and the whole miss sentence belong to `tcl_cmd_core::ensemble` —
    /// the prefix loop and the `unknown or ambiguous subcommand "` literal
    /// were duplicated here beside the owner's `subcommand_choices`.
    ///
    /// tclsh 9.0.4:
    ///   info {}   -> unknown or ambiguous subcommand "": must be args, body,
    ///                class, cmdcount, cmdtype, commands, complete, constant,
    ///                consts, coroutine, default, errorstack, exists, frame,
    ///                functions, globals, hostname, level, library, loaded,
    ///                locals, nameofexecutable, object, patchlevel, procs,
    ///                script, sharedlibextension, tclversion, or vars
    ///   info e x  -> unknown or ambiguous subcommand "e": must be <same>
    ///   info ex x -> 0        (a unique prefix still resolves)
    #[test]
    fn info_ensemble_miss_carries_the_full_option_list() {
        const MUST: &str = "must be args, body, class, cmdcount, cmdtype, commands, complete, \
                            constant, consts, coroutine, default, errorstack, exists, frame, \
                            functions, globals, hostname, level, library, loaded, locals, \
                            nameofexecutable, object, patchlevel, procs, script, \
                            sharedlibextension, tclversion, or vars";
        leak_free(|i| {
            assert_eq!(i.eval_str(b"info {}"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("unknown or ambiguous subcommand \"\": {MUST}").as_bytes()
            );
            assert_eq!(i.eval_str(b"info e x"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                format!("unknown or ambiguous subcommand \"e\": {MUST}").as_bytes()
            );
            assert_eq!(run(i, b"set x 1; info ex x"), b"1");
            assert_eq!(run(i, b"info comm set"), b"set");
        });
    }

    #[test]
    fn info_commands_qualified_pattern() {
        leak_free(|i| {
            run(
                i,
                b"namespace eval ::myns { proc aaa {} {}; proc abb {} {}; proc zzz {} {} }",
            );
            // A namespace-qualified glob lists that namespace's commands,
            // re-qualified, sorted.
            assert_eq!(
                run(i, b"lsort [info commands ::myns::a*]"),
                b"::myns::aaa ::myns::abb"
            );
            assert_eq!(run(i, b"info commands ::myns::zzz"), b"::myns::zzz");
            // A non-existent namespace yields nothing.
            assert_eq!(run(i, b"info commands ::nope::*"), b"");
            // An unqualified pattern still works (global builtin present).
            assert_eq!(run(i, b"info commands ::set"), b"::set");
        });
    }

    #[test]
    fn info_level_reports_call_words() {
        leak_free(|i| {
            assert_eq!(run(i, b"info level"), b"0"); // global scope
            run(i, b"proc outer {a b} {inner $a}");
            run(
                i,
                b"proc inner {x} {list [info level] [info level 0] [info level 1]}",
            );
            // outer 1 2 → inner 1 ; inside inner: level 2, this call, the outer call.
            assert_eq!(run(i, b"outer 1 2"), b"2 {inner 1} {outer 1 2}");
        });
    }

    #[test]
    fn info_frame_stack() {
        // Eval mode (the runtime's tests): `type proc`/`eval` with body-relative
        // lines — verified against tclsh 9.0 (via stdin). A `[info frame]` reports
        // the substituted command but the enclosing line.
        leak_free(|i| {
            // Depth: 1 at top level, 2 inside a proc.
            assert_eq!(run(i, b"info frame"), b"1");
            run(i, b"proc p {} { return [info frame] }");
            assert_eq!(run(i, b"p"), b"2");
            // `info frame 0` inside a proc: type proc, the FQN, level 0, and the
            // substituted command at its (body-relative) line.
            run(i, b"proc q {} {\n    return [info frame 0]\n}");
            assert_eq!(
                run(i, b"q"),
                b"type proc line 2 cmd {info frame 0} proc ::q level 0"
            );
            // Absolute index 1 = the root (top-level) frame: type eval.
            run(i, b"proc r {} { return [info frame 1] }");
            let out = run(i, b"r");
            assert!(
                out.starts_with(b"type eval line ") && out.ends_with(b"cmd r level 1"),
                "got {:?}",
                String::from_utf8_lossy(&out)
            );
            // Relative -1 from a callee names the caller proc.
            run(i, b"proc a {} { b }");
            run(i, b"proc b {} { return [info frame -1] }");
            assert_eq!(run(i, b"a"), b"type proc line 1 cmd {b } proc ::a level 1");
            // Out of range → bad level.
            assert_eq!(i.eval_str(b"info frame 99"), Code::Error);
        });
    }

    #[test]
    fn info_frame_uplevel_is_eval_typed_without_level() {
        // An `uplevel` body is `type eval` (a fresh dynamically-evaluated body),
        // keeps the *invoking* proc's name, and omits the `level` key (its scope
        // is redirected). Byte-verified vs tclsh 9.0.
        leak_free(|i| {
            run(i, b"proc h {} { uplevel 1 { return [info frame 0] } }");
            assert_eq!(
                run(i, b"h"),
                b"type eval line 1 cmd {info frame 0} proc ::h"
            );
        });
    }

    // Needs the numeric tower: `info functions` needs the `::tcl::mathfunc::*` table.
    #[cfg(have_tommath)]
    #[test]
    fn info_cmdtype_loaded_functions() {
        leak_free(|i| {
            assert_eq!(run(i, b"info cmdtype puts"), b"native");
            assert_eq!(run(i, b"proc p {} {}; info cmdtype p"), b"proc");
            assert_eq!(run(i, b"info loaded"), b"");
            assert_eq!(run(i, b"info functions sin"), b"sin");
            // cmdcount increases as commands run.
            assert_eq!(
                run(
                    i,
                    b"set a [info cmdcount]; set b [info cmdcount]; expr {$b > $a}"
                ),
                b"1"
            );
        });
    }

    // Needs the numeric tower: this covers `info functions` and the shared
    // `info commands` namespace enumeration over the math-function table.
    #[cfg(have_tommath)]
    #[test]
    fn math_enumeration_tracks_the_registry_selected_runtime_surface() {
        use tcl_dialect::TclVersion;

        leak_free(|i| {
            i.set_runtime_version(TclVersion::V8_4);
            // TP/TN: Tcl 8.4 introspects its fixed function table, while the
            // later command-table wrapper itself remains absent.
            assert_eq!(run(i, b"info functions sin"), b"sin");
            assert_eq!(run(i, b"info commands ::tcl::mathfunc::sin"), b"");

            i.set_runtime_version(TclVersion::V8_6);
            // FN: the 9.0-only builtin is hidden by both enumerators.
            assert_eq!(run(i, b"info functions isfinite"), b"");
            assert_eq!(run(i, b"info commands ::tcl::mathfunc::isfinite"), b"");
            // TN: the older command-table builtin remains visible.
            assert_eq!(run(i, b"info functions sin"), b"sin");
            assert_eq!(
                run(i, b"info commands ::tcl::mathfunc::sin"),
                b"::tcl::mathfunc::sin"
            );

            i.set_runtime_version(TclVersion::V9_0);
            // TP: the 9.0 surface exposes the builtin consistently.
            assert_eq!(run(i, b"info functions isfinite"), b"isfinite");
            assert_eq!(
                run(i, b"info commands ::tcl::mathfunc::isfinite"),
                b"::tcl::mathfunc::isfinite"
            );

            i.set_runtime_version(TclVersion::V8_6);
            run(
                i,
                b"proc ::tcl::mathfunc::isfinite {x} {return replacement}",
            );
            // FP guard: a user replacement is not a registry builtin and must
            // remain visible even when the native builtin is version-hidden.
            assert_eq!(run(i, b"info functions isfinite"), b"isfinite");
            assert_eq!(
                run(i, b"info commands ::tcl::mathfunc::isfinite"),
                b"::tcl::mathfunc::isfinite"
            );
        });
    }

    #[test]
    fn info_complete_balances() {
        leak_free(|i| {
            assert_eq!(run(i, b"info complete {set x 1}"), b"1");
            assert_eq!(run(i, b"info complete {set x {a b}}"), b"1");
            assert_eq!(run(i, b"info complete \"set x {a\""), b"0"); // open brace
            assert_eq!(run(i, b"info complete {puts [expr 1}"), b"0"); // open bracket
            assert_eq!(run(i, b"info complete {a \"b}"), b"0"); // open quote
        });
    }

    #[test]
    fn info_exists_scalar_and_array() {
        leak_free(|i| {
            assert_eq!(run(i, b"info exists nope"), b"0");
            run(i, b"set x 1");
            assert_eq!(run(i, b"info exists x"), b"1");
            run(i, b"set a(k) v");
            assert_eq!(run(i, b"info exists a(k)"), b"1");
            assert_eq!(run(i, b"info exists a(nope)"), b"0");
            assert_eq!(run(i, b"info exists a"), b"1"); // the array exists
            i.eval_str(b"unset x a");
        });
    }

    #[test]
    fn fresh_interp_exposes_c_create_interp_versions() {
        leak_free(|i| {
            // C installs these globals in `Tcl_CreateInterp`, before `Tcl_Init`;
            // every fresh Rust interp must expose the same core lifecycle.
            assert_eq!(run(i, b"info tclversion"), b"9.0");
            assert_eq!(run(i, b"info patchlevel"), b"9.0.4");
            assert_eq!(
                run(i, b"info sharedlibextension"),
                tcl_platform::bootstrap::SHARED_LIBRARY_EXTENSION.as_bytes()
            );
        });
    }

    #[test]
    fn info_patchlevel_still_errors_after_global_is_unset() {
        leak_free(|i| {
            // The command still reads the variable rather than hiding a
            // constant: explicitly unsetting it keeps C's error behaviour.
            assert_eq!(
                i.eval_str(b"unset ::tcl_patchLevel; info patchlevel"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"can't read \"tcl_patchLevel\": no such variable"
            );
        });
    }

    #[test]
    fn info_procs() {
        leak_free(|i| {
            run(i, b"proc greet {name {g hi}} {return $g-$name}");
            assert_eq!(run(i, b"info args greet"), b"name g");
            assert_eq!(run(i, b"info body greet"), b"return $g-$name");
            // default: sets the var, returns 1; missing default returns 0.
            assert_eq!(run(i, b"info default greet g d"), b"1");
            assert_eq!(run(i, b"set d"), b"hi");
            assert_eq!(run(i, b"info default greet name d"), b"0");
            // a proc appears in `info procs`; a builtin does not.
            assert_eq!(run(i, b"info procs greet"), b"greet");
            assert_eq!(run(i, b"info procs nosuch*"), b"");
            i.eval_str(b"unset d");
        });
    }

    // Needs the numeric tower: membership asserts go through `expr`.
    #[cfg(have_tommath)]
    #[test]
    fn info_vars_locals_globals() {
        leak_free(|i| {
            run(
                i,
                b"namespace eval foo { variable a 1; variable b 2 }\nset gx 10",
            );
            // In a proc: `info locals` is the genuine locals; `info vars` also
            // lists the linked namespace var by its local alias.
            run(
                i,
                b"proc p {} { set loc 5; variable ::foo::a; \
                  set ::out \"[lsort [info locals]]|[lsort [info vars]]\" }",
            );
            run(i, b"p");
            assert_eq!(run(i, b"set ::out"), b"loc|a loc");
            // `info globals` lists global-namespace vars only (not `foo::a`).
            assert_eq!(run(i, b"lsearch -exact [info globals] foo::a"), b"-1");
            assert_eq!(
                run(i, b"expr {[lsearch -exact [info globals] gx] >= 0}"),
                b"1"
            );
            // A qualified `info vars` lists that namespace re-qualified.
            assert_eq!(run(i, b"lsort [info vars ::foo::*]"), b"::foo::a ::foo::b");
            // At namespace scope, `info vars` lists the namespace's own vars.
            assert_eq!(run(i, b"namespace eval foo { lsort [info vars] }"), b"a b");
        });
    }

    // Needs the numeric tower: membership asserts go through `expr`.
    #[cfg(have_tommath)]
    #[test]
    fn info_commands_procs_namespaced() {
        leak_free(|i| {
            run(
                i,
                b"namespace eval foo { proc bar {} {}; proc baz {} {} }\n\
                  namespace eval foo::sub { proc deep {} {} }\nproc gproc {} {}",
            );
            // A qualified glob lists that namespace re-qualified (direct members
            // only — not `foo::sub::deep`).
            assert_eq!(
                run(i, b"lsort [info commands ::foo::*]"),
                b"::foo::bar ::foo::baz"
            );
            assert_eq!(
                run(i, b"lsort [info procs foo::*]"),
                b"::foo::bar ::foo::baz"
            );
            // A namespaced proc is not in the global listing.
            assert_eq!(run(i, b"lsearch -exact [info commands] foo::bar"), b"-1");
            // The bug this share fixed: inside a namespace, `info procs` lists only
            // that namespace's procs (no global merge), while `info commands` does
            // see the global `gproc`.
            assert_eq!(
                run(i, b"namespace eval foo { lsort [info procs] }"),
                b"bar baz"
            );
            assert_eq!(
                run(
                    i,
                    b"namespace eval foo { expr {[lsearch -exact [info commands] gproc] >= 0} }"
                ),
                b"1"
            );
            // A missing namespace qualifier → the empty list, not an error.
            assert_eq!(run(i, b"info commands ::nope::*"), b"");
        });
    }

    #[test]
    fn info_proc_introspection_errors() {
        leak_free(|i| {
            run(i, b"proc greet {name {g hi}} {return $g-$name}");
            // body/args/default on a non-proc (unknown name or a builtin) → the
            // shared `"name" isn't a procedure` error (pinned vs tclsh 9.0).
            for src in [
                &b"info body nosuch"[..],
                &b"info args nosuch"[..],
                &b"info default nosuch a d"[..],
                &b"info body set"[..],
            ] {
                assert_eq!(i.eval_str(src), Code::Error);
            }
            assert_eq!(i.result_bytes(), b"\"set\" isn't a procedure");
            // An unknown formal parameter.
            assert_eq!(i.eval_str(b"info default greet zz d"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"procedure \"greet\" doesn't have an argument \"zz\""
            );
        });
    }

    #[test]
    fn info_level_and_vars() {
        leak_free(|i| {
            assert_eq!(run(i, b"info level"), b"0"); // global scope
            run(i, b"set alpha 1");
            run(i, b"set beta 2");
            // info globals lists global vars (glob-filtered).
            assert_eq!(run(i, b"info globals alpha"), b"alpha");
            assert_eq!(run(i, b"info exists alpha"), b"1");
            i.eval_str(b"unset alpha beta");
        });
    }
}
