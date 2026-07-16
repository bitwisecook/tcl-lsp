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

//! `global` / `variable` / `upvar` (T1.5, the variable-namespace side).
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
//! At the global / `namespace eval` scope (no proc frame) `global`/`variable`
//! are no-ops for the link (the var already lives in the right namespace table);
//! `variable name value` still initialises the value. See `tclVar.c`
//! (`Tcl_GlobalObjCmd` / `Tcl_VariableObjCmd` / `Tcl_UpvarObjCmd`) and
//! `namespace-tree.md` §5.3 for the modelled semantics.

use tcl_syntax::naming::is_qualified;

use crate::frame::{split_array_ref, Link, VarHome};
use crate::interp::{obj_bytes, Code, Interp};
use crate::namespace::GLOBAL;
use crate::obj::TclObj;

/// Register `global`, `variable`, and `upvar`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"global", global);
    interp.register_builtin(b"variable", variable);
    interp.register_builtin(b"upvar", upvar);
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
    interp.set_error(&m)
}

// -- global ----------------------------------------------------------------

/// `global varName ?varName ...?` — link each name's tail to the global of that
/// name (resolved in the global namespace context).
fn global(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // `global` with no names is a no-op (TIP 323).
    for &a in &argv[1..] {
        let name = obj_bytes(a);
        match interp.resolve_var_target(GLOBAL, &name) {
            Some((ns, tail)) => interp.make_variable(ns, &tail),
            None => return no_namespace(interp, b"access", &name),
        }
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

// -- variable --------------------------------------------------------------

/// `variable ?name value ...? name ?value?` — declare/link namespace variables,
/// initialising those given a value. The trailing name may omit its value.
pub(crate) fn variable(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // `variable` with no names is a no-op (TIP 323).
    let current = interp.current_ns();
    let mut i = 1;
    while i < argv.len() {
        let name = obj_bytes(argv[i]);
        let Some((ns, tail)) = interp.resolve_var_target(current, &name) else {
            return no_namespace(interp, b"define", &name);
        };
        // A `variable` may not name an array element (C's `TclObjLookupVarEx`
        // rejects an `arr(elem)` target).
        if crate::frame::split_array_ref(&tail).1.is_some() {
            let mut m = b"can't define \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b"\": name refers to an element in an array");
            return interp.set_error(&m);
        }
        // Install the link first; a following `var_set(tail, …)` then writes
        // *through* it into the target namespace (the link makes the unqualified
        // tail resolve there, in a proc or at namespace scope alike).
        interp.make_variable(ns, &tail);
        if i + 1 < argv.len() {
            let value = argv[i + 1];
            if let Err(e) = interp.var_set(&tail, value) {
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

// -- upvar -----------------------------------------------------------------

/// `upvar ?level? otherVar localVar ?otherVar localVar ...?` — link each
/// `localVar` in the current frame to `otherVar`. The optional level is `#N`
/// (absolute) or `N` (relative, default `1`); a namespace-qualified `otherVar`
/// links to that namespace var regardless of level.
fn upvar(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let usage = b"upvar ?level? otherVar localVar ?otherVar localVar ...?";
    if argv.len() < 3 {
        return interp.wrong_args(usage);
    }
    // A level is present iff the arg count is even (cmd + level + pairs); else
    // the default relative level 1 applies and the pairs start at argv[1].
    let has_level = argv.len() % 2 == 0;
    let (spec, pairs_start) = if has_level {
        (obj_bytes(argv[1]), 2)
    } else {
        (b"1".to_vec(), 1)
    };
    if argv.len() - pairs_start < 2 {
        return interp.wrong_args(usage);
    }
    let target_level = parse_level(&spec, interp.current_level());
    // A *relative* `upvar 0` at namespace-eval scope (no proc frame) resolves an
    // unqualified other-var against the current namespace, not the global frame
    // — e.g. `upvar 0 Option(-debug) debug` inside `namespace eval`. (`#0` is
    // absolute and always means the global level.)
    let relative_here = !spec.starts_with(b"#")
        && !interp.in_proc()
        && interp.current_ns() != GLOBAL
        && target_level == Some(interp.current_level());

    let mut i = pairs_start;
    while i + 1 < argv.len() {
        let other = obj_bytes(argv[i]);
        let local = obj_bytes(argv[i + 1]);
        // The local name may not look like an array element (C's `MakeUpvar`).
        if split_array_ref(&local).1.is_some() {
            let mut m = b"bad variable name \"".to_vec();
            m.extend_from_slice(&local);
            m.extend_from_slice(
                b"\": can't create a scalar variable that looks like an array element",
            );
            return interp.set_error(&m);
        }
        let (base, elem) = split_array_ref(&other);

        let link = if is_qualified(&base) {
            // A qualified other-var names a namespace var (level is irrelevant).
            match interp.resolve_var_target(interp.current_ns(), &base) {
                Some((ns, simple)) => Link {
                    home: VarHome::Namespace(ns),
                    name: simple,
                    elem,
                },
                None => return no_namespace(interp, b"access", &other),
            }
        } else {
            // A frame-local other-var at the resolved level.
            let Some(level) = target_level else {
                let mut m = b"bad level \"".to_vec();
                m.extend_from_slice(&spec);
                m.push(b'"');
                return interp.set_error(&m);
            };
            let home = if relative_here {
                VarHome::Namespace(interp.current_ns())
            } else {
                VarHome::Frame(level)
            };
            Link {
                home,
                name: base,
                elem,
            }
        };
        // A qualified local name (`ns::lnk`) creates a namespace link variable
        // rather than a frame local; its namespace must exist.
        if is_qualified(&local) {
            match interp.resolve_var_target(interp.current_ns(), &local) {
                Some((target_ns, tail)) => interp.make_upvar_in(target_ns, &tail, link),
                None => {
                    let mut m = b"can't create \"".to_vec();
                    m.extend_from_slice(&local);
                    m.extend_from_slice(b"\": parent namespace doesn't exist");
                    return interp.set_error(&m);
                }
            }
        } else {
            interp.make_upvar(link, &local);
        }
        i += 2;
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

/// Parse an `upvar`/`uplevel` level spec to an **absolute** frame level, or
/// `None` if it isn't a valid level for the `current` depth. `#N` is absolute; a
/// bare `N` is relative (`current - N`).
pub(crate) fn parse_level(spec: &[u8], current: usize) -> Option<usize> {
    if let Some(rest) = spec.strip_prefix(b"#") {
        let n = parse_usize(rest)?;
        (n <= current).then_some(n)
    } else if !spec.is_empty() && spec.iter().all(u8::is_ascii_digit) {
        let n = parse_usize(spec)?;
        current.checked_sub(n)
    } else {
        None
    }
}

fn parse_usize(s: &[u8]) -> Option<usize> {
    if s.is_empty() || !s.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut acc: usize = 0;
    for &c in s {
        acc = acc.checked_mul(10)?.checked_add((c - b'0') as usize)?;
    }
    Some(acc)
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
            // the link named `x` was installed in the global table too.
            assert_eq!(i.eval_str(b"set x"), Code::Ok);
            assert_eq!(i.result_bytes(), b"5");
            i.eval_str(b"unset ::a::x");
        });
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

    /// M11: the 8.x namespace-scope fallback to global, off by default (9.0 /
    /// TIP 278) and on via `set_ns_var_global_fallback` — tclsh 8.6/9.0-pinned
    /// (reads fall back, writes hit the global, a `variable` declaration
    /// blocks it, and `info exists` / `unset` agree).
    #[test]
    fn ns_scope_unqualified_falls_back_to_global_only_under_8x() {
        // Default (9.0): no fallback anywhere.
        leak_free(|i| {
            i.eval_str(b"set g GLOBAL");
            assert_eq!(i.eval_str(b"namespace eval foo { set g }"), Code::Error);
            assert_eq!(i.eval_str(b"namespace eval foo { set g WRITTEN }"), Code::Ok);
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
            i.set_ns_var_global_fallback(true);
            i.eval_str(b"set g GLOBAL");
            assert_eq!(i.eval_str(b"namespace eval foo { set g }"), Code::Ok);
            assert_eq!(i.result_bytes(), b"GLOBAL");
            assert_eq!(i.eval_str(b"namespace eval foo { set g WRITTEN }"), Code::Ok);
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
}
