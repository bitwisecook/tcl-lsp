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
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::list;
use crate::obj::TclObj;

/// Register `info`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"info", info_cmd);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

fn info_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"info subcommand ?arg ...?");
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
        b"default",
        b"exists",
        b"frame",
        b"functions",
        b"globals",
        b"level",
        b"loaded",
        b"library",
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
    let sub: &[u8] = if SUBS.contains(&raw.as_slice()) {
        &raw
    } else {
        let hits: Vec<&&[u8]> = SUBS
            .iter()
            .filter(|s| s.starts_with(raw.as_slice()))
            .collect();
        if hits.len() == 1 {
            hits[0]
        } else {
            &raw // 0 or ambiguous → the error arm
        }
    };
    match sub {
        b"exists" => info_exists(interp, argv),
        b"commands" => set_list(interp, argv, Interp::visible_command_names),
        b"procs" => set_list(interp, argv, Interp::visible_proc_names),
        b"vars" => set_list(interp, argv, Interp::visible_var_names),
        b"globals" => set_list(interp, argv, Interp::global_var_names),
        b"locals" => set_list(interp, argv, Interp::local_var_names),
        b"level" => info_level(interp, argv),
        b"frame" => info_frame(interp, argv),
        b"object" => crate::cmd_oo::info_object(interp, argv),
        b"class" => crate::cmd_oo::info_class(interp, argv),
        b"tclversion" => fixed(interp, argv, b"info tclversion", b"9.0"),
        b"patchlevel" => fixed(interp, argv, b"info patchlevel", b"9.0.3"),
        // The host shared-library suffix (`$::tcl_platform(platform)` is unix).
        b"sharedlibextension" => fixed(interp, argv, b"info sharedlibextension", b".so"),
        b"complete" => {
            if argv.len() != 3 {
                return wrong_args(interp, b"info complete command");
            }
            let ok = command_complete(&obj_bytes(argv[2]));
            interp.set_result_bytes(if ok { b"1" } else { b"0" });
            Code::Ok
        }
        b"body" => info_body(interp, argv),
        b"args" => info_args(interp, argv),
        b"default" => info_default(interp, argv),
        b"cmdtype" => info_cmdtype(interp, argv),
        b"cmdcount" => {
            if argv.len() != 2 {
                return wrong_args(interp, b"info cmdcount");
            }
            interp.set_result(crate::obj::new_wide_int_obj(interp.cmd_count() as i64));
            Code::Ok
        }
        b"functions" => {
            if argv.len() > 3 {
                return wrong_args(interp, b"info functions ?pattern?");
            }
            let pat = argv.get(2).map(|&a| obj_bytes(a));
            set_filtered(interp, interp.mathfunc_names(), pat.as_deref())
        }
        b"loaded" => {
            if argv.len() > 4 {
                return wrong_args(interp, b"info loaded ?interp? ?prefix?");
            }
            interp.set_result_bytes(b"");
            Code::Ok
        }
        b"script" => {
            if argv.len() > 3 {
                return wrong_args(interp, b"info script ?filename?");
            }
            interp.set_result_bytes(&interp.current_script());
            Code::Ok
        }
        b"nameofexecutable" => {
            let exe = std::env::current_exe()
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            interp.set_result_bytes(exe.as_bytes());
            Code::Ok
        }
        b"library" => match interp.var_get(b"::tcl_library") {
            Some(o) => {
                interp.set_result(o);
                Code::Ok
            }
            None => interp.set_error(b"variable \"::tcl_library\" undefined"),
        },
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(
                b"\": must be args, body, class, cmdcount, cmdtype, commands, complete, default, exists, frame, functions, globals, level, library, loaded, locals, nameofexecutable, object, patchlevel, procs, script, sharedlibextension, tclversion, or vars",
            );
            interp.set_error(&m)
        }
    }
}

/// `info cmdtype command` — the kind of command (`native`/`proc`/`alias`/…).
fn info_cmdtype(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"info cmdtype commandName");
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
        .filter(|n| pattern.map_or(true, |p| glob_match(p, n)))
        .map(|n| new_string(n))
        .collect();
    let l = list::new_list_obj(&objs); // retains each element
    interp.set_result(l); // retains the list; the rc-0 temporaries are now owned by it
    Code::Ok
}

/// `info <sub> ?pattern?` over a name producer.
fn set_list(interp: &mut Interp, argv: &[*mut TclObj], names: fn(&Interp) -> Vec<Vec<u8>>) -> Code {
    if argv.len() > 3 {
        return wrong_args(interp, b"info <subcommand> ?pattern?");
    }
    let pattern = argv.get(2).map(|&a| obj_bytes(a));
    let list = names(interp);
    set_filtered(interp, list, pattern.as_deref())
}

fn info_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"info exists varName");
    }
    let exists = interp.var_exists(&obj_bytes(argv[2]));
    interp.set_result_bytes(if exists { b"1" } else { b"0" });
    Code::Ok
}

fn info_level(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    match argv.len() {
        2 => {
            interp.set_result_bytes(interp.level().to_string().as_bytes());
            Code::Ok
        }
        // `info level N` — the command words at level N. N<=0 is relative to the
        // current level (`info level 0` = the current call); N>0 is absolute.
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
            let cur = interp.level() as i64;
            let target = if n <= 0 { cur + n } else { n };
            if target <= 0 {
                return bad_level(interp, &spec);
            }
            match interp.level_words(target as usize) {
                Some(words) => {
                    let objs: Vec<*mut TclObj> =
                        words.iter().map(|w| crate::interp::new_string(w)).collect();
                    interp.set_result(crate::list::new_list_obj(&objs));
                    Code::Ok
                }
                None => bad_level(interp, &spec),
            }
        }
        _ => wrong_args(interp, b"info level ?number?"),
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
        _ => wrong_args(interp, b"info frame ?number?"),
    }
}

/// `Tcl_CommandComplete` (approximation): a script is complete when no brace,
/// bracket, or quote is left open and it doesn't end mid-escape. Inside braces
/// only `{`/`}` nest; inside quotes `[...]` command substitution still nests.
fn command_complete(s: &[u8]) -> bool {
    let mut stack: Vec<u8> = Vec::new(); // expected closers: `}` or `]`
    let mut in_quote = false;
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if c == b'\\' {
            // A backslash escapes the next byte; a trailing one leaves the
            // command incomplete (line continuation awaiting more input).
            if i + 1 >= s.len() {
                return false;
            }
            i += 2;
            continue;
        }
        let in_brace = stack.last() == Some(&b'}');
        if in_brace {
            match c {
                b'{' => stack.push(b'}'),
                b'}' => {
                    stack.pop();
                }
                _ => {}
            }
        } else if in_quote {
            match c {
                b'"' => in_quote = false,
                b'[' => stack.push(b']'),
                b']' if stack.last() == Some(&b']') => {
                    stack.pop();
                }
                _ => {}
            }
        } else {
            match c {
                b'{' => stack.push(b'}'),
                b'[' => stack.push(b']'),
                b']' => {
                    if stack.last() == Some(&b']') {
                        stack.pop();
                    }
                }
                b'"' => in_quote = true,
                _ => {}
            }
        }
        i += 1;
    }
    stack.is_empty() && !in_quote
}

fn bad_level(interp: &mut Interp, spec: &[u8]) -> Code {
    let mut m = b"bad level \"".to_vec();
    m.extend_from_slice(spec);
    m.push(b'"');
    interp.set_error(&m)
}

fn fixed(interp: &mut Interp, argv: &[*mut TclObj], usage: &[u8], value: &[u8]) -> Code {
    if argv.len() != 2 {
        return wrong_args(interp, usage);
    }
    interp.set_result_bytes(value);
    Code::Ok
}

fn info_body(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"info body procname");
    }
    let name = obj_bytes(argv[2]);
    match interp.proc_def(&name) {
        Some(def) => {
            interp.set_result_bytes(&def.body);
            Code::Ok
        }
        None => not_a_proc(interp, &name),
    }
}

fn info_args(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"info args procname");
    }
    let name = obj_bytes(argv[2]);
    match interp.proc_def(&name) {
        Some(def) => {
            let names: Vec<Vec<u8>> = def.params.iter().map(|p| p.name.clone()).collect();
            // info args preserves declaration order (not sorted).
            let objs: Vec<*mut TclObj> = names.iter().map(|n| new_string(n)).collect();
            interp.set_result(list::new_list_obj(&objs));
            Code::Ok
        }
        None => not_a_proc(interp, &name),
    }
}

fn info_default(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return wrong_args(interp, b"info default procname arg varname");
    }
    let proc = obj_bytes(argv[2]);
    let arg = obj_bytes(argv[3]);
    let var = obj_bytes(argv[4]);
    let Some(def) = interp.proc_def(&proc) else {
        return not_a_proc(interp, &proc);
    };
    let Some(param) = def.params.iter().find(|p| p.name == arg) else {
        let mut m = b"procedure \"".to_vec();
        m.extend_from_slice(&proc);
        m.extend_from_slice(b"\" doesn't have an argument \"");
        m.extend_from_slice(&arg);
        m.push(b'"');
        return interp.set_error(&m);
    };
    match &param.default {
        Some(d) => {
            let o = new_string(d);
            if interp.var_set(&var, o).is_err() {
                crate::interp::drop_fresh(o);
                return interp.set_error(b"couldn't store default value");
            }
            interp.set_result_bytes(b"1");
        }
        None => interp.set_result_bytes(b"0"),
    }
    Code::Ok
}

fn not_a_proc(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"\"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\" isn't a procedure");
    interp.set_error(&m)
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
    fn info_version_and_procs() {
        leak_free(|i| {
            assert_eq!(run(i, b"info tclversion"), b"9.0");
            assert_eq!(run(i, b"info patchlevel"), b"9.0.3");
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
