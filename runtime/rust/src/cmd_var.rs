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
// `incr`'s tower add — the only user — is `have_tommath`-gated.
#[cfg(have_tommath)]
use crate::interp::drop_fresh;
use crate::namespace::GLOBAL;
#[cfg(have_tommath)]
use crate::obj;
use crate::obj::TclObj;

/// Register `global`, `variable`, and `upvar`; also re-registers `set` (and,
/// where the numeric tower is linked, `incr`) to fix their return value after
/// a write trace runs (issue #1633 row 1) — see [`set_cmd`] and [`incr_cmd`].
/// The override pattern mirrors TclOO's own `variable` override in
/// `builtins::install` (installed later still wins; nothing after this lane's
/// files re-registers `set`/`incr`).
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"global", global);
    interp.register_builtin(b"variable", variable);
    interp.register_builtin(b"upvar", upvar);
    interp.register_builtin(b"set", set_cmd);
    #[cfg(have_tommath)]
    interp.register_builtin(b"incr", incr_cmd);
}

// -- set / incr: return-after-trace ------------------------------------------
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
        2 => {
            let name = obj_bytes(argv[1]);
            let (base, elem) = split_array_ref(&name);
            if let Some(c) = interp.fire_read_trace(&base, elem.as_deref()) {
                return c;
            }
            let val = match &elem {
                Some(k) => interp.var_get_elem(&base, k),
                None => interp.var_get(&base),
            };
            match val {
                Some(o) => {
                    interp.set_result(o);
                    Code::Ok
                }
                None => {
                    let msg = interp.read_miss_msg(&base, elem.as_deref());
                    interp.set_error(&msg)
                }
            }
        }
        3 => {
            let name = obj_bytes(argv[1]);
            let (base, elem) = split_array_ref(&name);
            let value = argv[2];
            match interp.store_var_result(&base, elem.as_deref(), value) {
                Ok(()) => Code::Ok,
                Err(e) => crate::builtins::var_error(interp, &name, e),
            }
        }
        _ => interp.wrong_args(b"set varName ?newValue?"),
    }
}

/// `can't incr "name": variable is a constant` — duplicated from
/// `builtins::incr_constant_error` (module-private there); C checks this
/// before the read-modify-write, so no read trace fires.
#[cfg(have_tommath)]
fn incr_constant_error(
    interp: &mut Interp,
    base: &[u8],
    elem: &Option<Vec<u8>>,
    name: &[u8],
) -> Option<Code> {
    if elem.is_none() && interp.is_constant(base) {
        let mut msg = b"can't incr \"".to_vec();
        msg.extend_from_slice(name);
        msg.extend_from_slice(b"\": variable is a constant");
        return Some(interp.set_error(&msg));
    }
    None
}

/// `incr varName ?increment?` — overrides `builtins::incr`'s store/result tail
/// only; the numeric-tower add is unchanged.
#[cfg(have_tommath)]
fn incr_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"incr varName ?increment?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = split_array_ref(&name);
    if let Some(c) = incr_constant_error(interp, &base, &elem, &name) {
        return c;
    }

    let cur = interp.read_for_update(&base, elem.as_deref());
    let one = obj::new_wide_int_obj(1);
    let amount = if argv.len() == 3 { argv[2] } else { one };

    let sum = tcl_syntax::value::ValueOps::int_add(interp, cur.as_ref(), &amount);
    drop_fresh(one); // the transient `1` (used or not) is no longer needed
    let sum = match sum {
        Ok(s) => s, // rc 0
        Err(e) => return interp.set_error(e.message().as_bytes()),
    };

    match interp.store_var_result(&base, elem.as_deref(), sum) {
        Ok(()) => Code::Ok,
        Err(e) => crate::builtins::var_error(interp, &name, e),
    }
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
        // C resolves the other-variable first, then rejects a namespace alias
        // to a procedure cell before inspecting the alias's element shape or
        // parent namespace (`TCL UPVAR INVERTED`).
        if interp.upvar_would_invert(&link, &local) {
            return inverted_upvar(interp, &local);
        }
        if split_array_ref(&local).1.is_some() {
            let mut m = b"bad variable name \"".to_vec();
            m.extend_from_slice(&local);
            m.extend_from_slice(
                b"\": can't create a scalar variable that looks like an array element",
            );
            return interp.error_with_code(&m, b"TCL UPVAR LOCAL_ELEMENT");
        }
        // A qualified local name (`ns::lnk`) creates a namespace link variable
        // rather than a frame local; its namespace must exist.
        if is_qualified(&local) {
            match interp.resolve_var_target(interp.current_ns(), &local) {
                Some((target_ns, tail)) => interp.make_upvar_in(target_ns, &tail, link),
                None => {
                    let mut m = b"can't create \"".to_vec();
                    m.extend_from_slice(&local);
                    m.extend_from_slice(b"\": parent namespace doesn't exist");
                    let mut error_code = b"TCL LOOKUP VARNAME ".to_vec();
                    error_code.extend_from_slice(&local);
                    return interp.error_with_code(&m, &error_code);
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

    /// M11: the 8.x namespace-scope fallback to global, off by default (9.0 /
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
    /// (`set_runtime_version` — issue #1328).
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
            assert_eq!(i.result_bytes(), b"8.6.16");
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
    /// side used (issue #1328).
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
