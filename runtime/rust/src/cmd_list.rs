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

//! List commands — `list` / `llength` / `lindex` / `lappend` / `lrange`
//! / `lreverse` / `concat` / `join` / `split` / `lassign` / `lrepeat` /
//! `linsert` / `lreplace` / `lset` / `ledit` / `lsearch` / `lsort`, over the
//! [`crate::list`] value type.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use tcl_cmd_core::index::IndexCompiler;
use tcl_cmd_core::list as list_core;

use crate::interp::{obj_bytes, Code, Interp};
use crate::list;
use crate::obj::{self, TclObj};

/// Map a portable `tcl-cmd-core` result onto the runtime's set-result/`Code` ABI.
/// A fresh result object (rc 0) is retained by `set_result`; a borrowed element
/// (e.g. `lindex`) is retained too, its parent list keeping its own ref.
fn adapt(interp: &mut Interp, result: Result<*mut TclObj, tcl_cmd_core::CmdError>) -> Code {
    match result {
        Ok(v) => {
            interp.set_result(v);
            Code::Ok
        }
        Err(e) => interp.report_cmd_error(e),
    }
}

/// Register the list commands.
pub fn install(interp: &mut Interp) {
    interp.register_stock_builtin(b"list", list_cmd);
    interp.register_stock_builtin(b"llength", llength);
    interp.register_stock_builtin(b"lindex", lindex);
    interp.register_stock_builtin(b"lappend", lappend);
    interp.register_stock_builtin(b"lrange", lrange);
    interp.register_stock_builtin(b"lreverse", lreverse);
    interp.register_stock_builtin(b"concat", concat);
    interp.register_stock_builtin(b"join", join);
    interp.register_stock_builtin(b"split", split);
    interp.register_stock_builtin(b"lassign", lassign);
    interp.register_stock_builtin(b"lrepeat", lrepeat);
    interp.register_stock_builtin(b"linsert", linsert);
    interp.register_stock_builtin(b"lreplace", lreplace);
    interp.register_stock_builtin(b"lset", lset);
    interp.register_stock_builtin(b"ledit", ledit);
    interp.register_stock_builtin(b"lpop", lpop);
    interp.register_stock_builtin(b"lremove", lremove);
    interp.register_stock_builtin(b"lsearch", lsearch);
    interp.register_stock_builtin(b"lsort", lsort);
}

// helpers

/// Set the result to a list built from element objects (each retained).
fn set_list(interp: &mut Interp, elems: &[*mut TclObj]) {
    interp.set_result(interp.new_list_object(elems));
}

// commands

/// `list ?arg ...?` — a list of its arguments.
fn list_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let v = list_core::list(interp, &argv[1..]);
    interp.set_result(v);
    Code::Ok
}

/// `llength list`.
fn llength(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"llength list");
    }
    let r = list_core::llength(interp, &argv[1]);
    adapt(interp, r)
}

/// `lindex list ?index ...?` — drill into a (nested) list. With no index the
/// whole list is returned; a single index argument is itself split into an
/// index *path* (so `lindex {{a b} c} {0 1}` works); multiple index arguments
/// each step one level. An out-of-range step yields the empty string. Mirrors
/// `Tcl_LindexObjCmd` (`TclLindexList`/`TclLindexFlat`).
fn lindex(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(
            interp
                .native_invocation_dialect()
                .list_index_usage()
                .unwrap_or_else(|| list_core::lindex_usage(interp.runtime_version()))
                .as_bytes(),
        );
    }
    // A one-index `lindex` the release compiles reads its literal index as
    // compiled (`IndexCompiler::Lindex`).
    let compiled = (argv.len() == 3)
        .then(|| interp.compiled_index_argv(IndexCompiler::Lindex, argv, &[2]))
        .flatten();
    let argv = compiled.as_ref().map_or(argv, |c| c.argv.as_slice());
    match interp.original_list_index(argv[1], &argv[2..], true) {
        Ok(result) => {
            interp.set_result(result.as_ptr());
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error),
    }
}

/// `lappend varName ?value ...?` — append to the list in `varName` (creating it
/// if unset), copy-on-write if the value is shared. Returns the new list.
pub(crate) fn lappend(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lappend varName ?value ...?");
    }
    let name = obj_bytes(argv[1]);
    let values = &argv[2..];
    // `lappend a(k) ...` must address the array element, not a scalar literally
    // named `a(k)` — split the array ref like `set`/`incr` do.
    let (base, elem) = match interp.variable_name_parts(&name) {
        Ok(parts) => parts,
        Err(error) => return crate::builtins::var_error(interp, &name, error),
    };

    // With no values the native read retains existing bytes. The selected
    // container protocol decides whether malformed list bytes are rejected;
    // an unset variable is created as an empty list.
    if values.is_empty() {
        let Some(validate) = tcl_registry::native_compilation::NativeAppendKind::List
            .validates_empty_result(interp.native_invocation_dialect())
        else {
            return interp.error(b"native list append policy is not selected");
        };
        // `lappend` fires a read trace on the variable it reads (restored in Tcl
        // 8.4 after 8.0 dropped it), but swallows a trace error, unlike `append`
        // (append-7.2/7.3/7.4, bug 3057639).
        let cur = match interp.lappend_read(&base, elem.as_deref()) {
            Ok(value) => value,
            Err(first) => return interp.refuse_native_execution(first),
        };
        return match cur {
            Some(o) if !validate => {
                interp.set_result(o);
                Code::Ok
            }
            Some(o) => match list::list_elements(o) {
                Ok(_) => {
                    interp.set_result(o);
                    Code::Ok
                }
                Err(e) => interp.report_cmd_error(e.into()),
            },
            None => {
                let empty = interp.new_list_object(&[]); // rc 0
                match interp.store_var_result(&base, elem.as_deref(), empty) {
                    Ok(()) => Code::Ok,
                    Err(e) => crate::builtins::var_error(interp, &name, e),
                }
            }
        };
    }

    // `lappend` reads the current value (to append to it), firing the read trace
    // first — before the write, matching C's get-then-set order — and swallowing
    // a trace error (a missing element is then created; bug 3057639, append-9.0).
    let cur = match interp.lappend_read(&base, elem.as_deref()) {
        Ok(value) => value,
        Err(first) => return interp.refuse_native_execution(first),
    };

    // A `lappend` with values writes; reject a constant before the update.
    if let Some(c) = interp.const_write_check(&name) {
        return c;
    }

    // COW-aware list append, shared with the VM via `lappend_value`: it appends in
    // place when the current value is an unshared list (returning that same
    // object), else builds a fresh list. Byte-exact (elements are never
    // stringified).
    let result = match tcl_cmd_core::var::lappend_value(interp, cur, values) {
        Ok(v) => v,
        Err(e) => return interp.report_cmd_error(e),
    };

    // Always store back: rebinds the variable (a refcount-neutral re-set when
    // appended in place) and fires the write trace once. `store_var_result`
    // holds a protective reference across the store so a write trace that unsets
    // the variable can't free a fresh `result` before it becomes the result.
    match interp.store_var_result(&base, elem.as_deref(), result) {
        Ok(()) => Code::Ok,
        Err(e) => crate::builtins::var_error(interp, &name, e),
    }
}

/// `lrange list first last` — the sublist from `first` to `last` (inclusive),
/// clamped to range.
fn lrange(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"lrange list first last");
    }
    let compiled = interp.compiled_index_argv(IndexCompiler::Lrange, argv, &[2, 3]);
    let argv = compiled.as_ref().map_or(argv, |c| c.argv.as_slice());
    match interp.original_list_range(argv[1], argv[2], argv[3]) {
        Ok(result) => {
            // SAFETY: the owned original result remains live; the result slot retains it.
            unsafe { interp.set_obj_result(result.as_ptr()) };
            Code::Ok
        }
        Err(error) => interp.report_cmd_error(error),
    }
}

/// `lreverse list`.
fn lreverse(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"lreverse list");
    }
    let r = list_core::lreverse(interp, &argv[1]);
    adapt(interp, r)
}

/// `concat ?arg ...?` — trim each arg of surrounding whitespace, drop empties,
/// join with single spaces (Tcl's string-level concat).
fn concat(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match crate::interp::native_arguments::OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let value = interp.concat_original_arguments(&argv[1..]);
    if let Err(error) = arguments.check_for(interp) {
        // A reached working result still completes its genuine retirement.
        if let Ok(value) = value {
            drop(crate::obj::Owned::fresh(value));
        }
        return interp.report_cmd_error(error);
    }
    let code = adapt(interp, value);
    arguments.finish(code)
}

/// `join list ?joinString?` — element string reps joined by `joinString`
/// (default a single space).
fn join(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"join list ?joinString?");
    }
    let sep = if argv.len() == 3 {
        Some(&argv[2])
    } else {
        None
    };
    let r = list_core::join(interp, &argv[1], sep);
    adapt(interp, r)
}

/// `split string ?splitChars?` — selected native character/byte branches or
/// the independently authored checked-Unicode text route.
fn split(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 || argv.len() > 3 {
        return interp.wrong_args(b"split string ?splitChars?");
    }
    let chars = if argv.len() == 3 {
        Some(&argv[2])
    } else {
        None
    };
    let result = list_core::split(interp, &argv[1], chars);
    adapt(interp, result)
}

/// `lassign list ?varName ...?` — assign successive elements to the vars
/// (missing → empty string); return the unassigned tail as a list.
fn lassign(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(grammar) = interp
        .native_invocation_dialect()
        .list_assignment_invocation()
    else {
        return interp.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "list-assignment invocation grammar",
            ),
        );
    };
    let Some((_, arguments)) = argv.split_first() else {
        return interp.wrong_args(grammar.usage().as_bytes());
    };
    if !grammar.accepts(arguments.len()) {
        return interp.wrong_args(grammar.usage().as_bytes());
    }
    let Some((list, vars)) = arguments.split_first() else {
        return interp.wrong_args(grammar.usage().as_bytes());
    };
    let elems = match list::list_elements(*list) {
        Ok(e) => e,
        Err(e) => return interp.report_cmd_error(e.into()),
    };
    for (i, &var) in vars.iter().enumerate() {
        let name = obj_bytes(var);
        // `arr(a)` writes the array *element*, not a literal scalar named
        // `arr(a)` — the same selected variable parts + `var_set`/
        // `var_set_elem` routing `set`/`lset` already use, so this doesn't
        // hand-roll a second name parser.
        let (base, elem) = match interp.variable_name_parts(&name) {
            Ok(parts) => parts,
            Err(error) => return crate::builtins::var_error(interp, &name, error),
        };
        let val = if i < elems.len() {
            elems[i]
        } else {
            obj::new_string_bytes(b"")
        };
        let fresh = i >= elems.len();
        let r = match &elem {
            Some(k) => interp.var_set_elem(&base, k, val),
            None => interp.var_set(&base, val),
        };
        if fresh {
            // `set` retained `val`; release our construction ref to the empty obj
            drop_fresh(val);
        }
        if let Err(e) = r {
            return crate::builtins::var_error(interp, &name, e);
        }
    }
    if vars.len() < elems.len() {
        set_list(interp, &elems[vars.len()..]);
    } else {
        interp.set_result_bytes(b"");
    }
    Code::Ok
}

// error helpers

// lrepeat / linsert / lreplace / lsearch / lsort

/// `lrepeat count ?value ...?` — `count` copies of the value sequence.
///
/// Delegates to the shared, radix-aware [`list_core::lrepeat`] so the count
/// accepts the full Tcl integer grammar (`0x3` → `a a a`), the negative-count
/// error reports the *actual* count (`lrepeat -3 a` → `bad count "-3"…`, not a
/// hard-coded `"-1"`), and the result-capacity multiply is `saturating_mul`
/// rather than an overflowing `count * values.len()`.
fn lrepeat(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lrepeat count ?value ...?");
    }
    let r = list_core::lrepeat(interp, &argv[1], &argv[2..]);
    adapt(interp, r)
}

/// `linsert list index ?element ...?` — insert before `index` (`end` appends).
fn linsert(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"linsert list index ?element ...?");
    }
    let compiled = interp.compiled_index_argv(IndexCompiler::Linsert, argv, &[2]);
    let argv = compiled.as_ref().map_or(argv, |c| c.argv.as_slice());
    let r = list_core::linsert(interp, &argv[1], &argv[2], &argv[3..]);
    adapt(interp, r)
}

/// `lreplace list first last ?element ...?` — replace the `[first,last]` range.
fn lreplace(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"lreplace list first last ?element ...?");
    }
    let compiled = interp.compiled_index_argv(IndexCompiler::Lreplace, argv, &[2, 3]);
    let argv = compiled.as_ref().map_or(argv, |c| c.argv.as_slice());
    let r = list_core::lreplace(interp, &argv[1], &argv[2], &argv[3], &argv[4..]);
    adapt(interp, r)
}

/// The list a list variable holds, read as `set` reads it: `listVar` may name
/// an array element (`a(k)`), and a missing variable is the read error, with
/// C's three-way distinction (variable-is-array / no-such-element /
/// no-such-variable).
fn read_list_var(
    interp: &mut Interp,
    base: &[u8],
    elem: Option<&[u8]>,
) -> Result<*mut TclObj, Code> {
    let cur = match elem {
        Some(k) => interp.var_get_elem(base, k),
        None => interp.var_get(base),
    };
    cur.ok_or_else(|| {
        let msg = interp.read_miss_msg(base, elem);
        interp.set_error(&msg)
    })
}

/// `lset listVar ?index ...? value` — set the element at the index path in the
/// list stored in `listVar` over the shared [`list_core::lset`] core, as the
/// emulated release computes it, store it back (firing write traces), and
/// return the new list (`Tcl_LsetObjCmd`).
fn lset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args(b"lset listVar ?index? ?index ...? value");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = match interp.variable_name_parts(&name) {
        Ok(parts) => parts,
        Err(error) => return crate::builtins::var_error(interp, &name, error),
    };
    let listobj = match read_list_var(interp, &base, elem.as_deref()) {
        Ok(obj) => obj,
        Err(code) => return code,
    };
    let value = argv[argv.len() - 1];
    let release = interp.runtime_version();
    let newlist = match list_core::lset(interp, &listobj, &argv[2..argv.len() - 1], value, release)
    {
        Ok(l) => l,
        Err(e) => return interp.report_cmd_error(e),
    };
    let stored = match &elem {
        Some(k) => interp.var_set_elem(&base, k, newlist),
        None => interp.var_set(&base, newlist),
    };
    if let Err(e) = stored {
        // `newlist` is fresh (rc 0) unless the path was empty (then it is the
        // borrowed `value`); only the fresh one needs releasing.
        if newlist != value {
            drop_fresh(newlist);
        }
        return crate::builtins::var_error(interp, &name, e);
    }
    interp.set_result(newlist);
    Code::Ok
}

/// `ledit listVar first last ?element ...?` — the in-place `lreplace` on a list
/// *variable* (Tcl 9.0) over the shared [`list_core::ledit`] core: reads
/// `listVar`, replaces the `[first,last]` range with the new elements, stores
/// the result back into the variable, and returns the new list value
/// (`Tcl_LeditObjCmd`).
fn ledit(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args(b"ledit listVar first last ?element ...?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = match interp.variable_name_parts(&name) {
        Ok(parts) => parts,
        Err(error) => return crate::builtins::var_error(interp, &name, error),
    };
    let listobj = match read_list_var(interp, &base, elem.as_deref()) {
        Ok(obj) => obj,
        Err(code) => return code,
    };
    let release = interp.runtime_version();
    // Build the new list first (retains every element), *then* store it: the
    // store releases the old value, but the elements survive because the new
    // list now holds its own refs. The new list is rc 0; `var_set*` retains it.
    let newlist = match list_core::ledit(interp, &listobj, &argv[2], &argv[3], &argv[4..], release)
    {
        Ok(l) => l,
        Err(e) => return interp.report_cmd_error(e),
    };
    let stored = match &elem {
        Some(k) => interp.var_set_elem(&base, k, newlist),
        None => interp.var_set(&base, newlist),
    };
    if stored.is_err() {
        drop_fresh(newlist);
        let mut m = b"can't set \"".to_vec();
        m.extend_from_slice(&name);
        m.extend_from_slice(b"\": variable is array");
        return interp.set_error(&m);
    }
    interp.set_result(newlist);
    Code::Ok
}

/// `lpop varName ?index ...?` — remove and return the element at the index
/// path (default the last element) over the shared [`list_core::lpop`] core,
/// storing the shortened list back into the variable.
fn lpop(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lpop varName ?index?");
    }
    let name = obj_bytes(argv[1]);
    let (base, elem) = match interp.variable_name_parts(&name) {
        Ok(parts) => parts,
        Err(error) => return crate::builtins::var_error(interp, &name, error),
    };
    let listobj = match read_list_var(interp, &base, elem.as_deref()) {
        Ok(obj) => obj,
        Err(code) => return code,
    };
    let release = interp.runtime_version();
    let (removed, newlist) = match list_core::lpop(interp, &listobj, &argv[2..], release) {
        Ok(r) => r,
        Err(e) => return interp.report_cmd_error(e),
    };
    // Retain `removed` (via the result) *before* the store releases the old
    // list, so it survives to be returned.
    interp.set_result(removed);
    let stored = match &elem {
        Some(k) => interp.var_set_elem(&base, k, newlist),
        None => interp.var_set(&base, newlist),
    };
    if stored.is_err() {
        drop_fresh(newlist);
        let mut m = b"can't set \"".to_vec();
        m.extend_from_slice(&name);
        m.extend_from_slice(b"\": variable is array");
        return interp.set_error(&m);
    }
    Code::Ok
}

/// `lremove list ?index ...?` — return `list` with the elements at the given
/// indices removed. Indices resolve (radix + `end`-relative), duplicates
/// collapse, and out-of-range indices are ignored.
fn lremove(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"lremove list ?index ...?");
    }
    let elems = match list::list_elements(argv[1]) {
        Ok(v) => v,
        Err(e) => return interp.report_cmd_error(e.into()),
    };
    let len = elems.len();
    let mut remove = vec![false; len];
    for &iv in &argv[2..] {
        match tcl_cmd_core::index::resolve_value(interp, &iv, len) {
            Ok(i) if i >= 0 && usize::try_from(i).is_ok_and(|i| i < len) => {
                remove[usize::try_from(i).expect("checked native index")] = true;
            }
            Ok(_) => {} // out of range is ignored by the selected command.
            Err(error) => return interp.report_cmd_error(error),
        }
    }
    let out: Vec<*mut TclObj> = elems
        .iter()
        .enumerate()
        .filter_map(|(i, e)| (!remove[i]).then_some(*e))
        .collect();
    set_list(interp, &out);
    Code::Ok
}

/// `lsearch ?-option value ...? list pattern` — a thin adapter over the shared
/// [`tcl_cmd_core::lsearch`] core, driven by the real Tcl ARE engine for the
/// `-regexp` mode. The whole command is a pure value->value function in the
/// core (`lsearch` never writes a variable); this adapter only maps the result
/// onto `set_result` and the error onto `set_error`/`error_with_code`.
fn lsearch(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp
        .native_invocation_dialect()
        .native_jim_regex_protocol()
        .is_some()
    {
        use tcl_cmd_core::native_jim_lsearch::JimLsearchError;
        return match tcl_cmd_core::native_jim_lsearch::lsearch(
            interp,
            &argv[1..],
            crate::cmd_regex::invoke_jim_match_command,
        ) {
            Ok(value) => {
                interp.set_result(value);
                Code::Ok
            }
            Err(JimLsearchError::Command(error)) => interp.report_cmd_error(error),
            Err(JimLsearchError::Callback(_)) | Err(JimLsearchError::NegativeMatch) => Code::Error,
            Err(JimLsearchError::Usage) => interp.wrong_args(b"lsearch ?options? list pattern"),
        };
    }
    let version = interp.runtime_version();
    use tcl_cmd_core::regex::OriginalRegexConsumerError;
    match tcl_cmd_core::lsearch::lsearch_original_with_jim::<
        Interp,
        crate::cmd_regex::AreEngine,
        Code,
    >(
        interp,
        &argv[1..],
        version,
        crate::cmd_regex::invoke_jim_regexp,
    ) {
        Ok(value) => {
            interp.set_result(value);
            Code::Ok
        }
        Err(OriginalRegexConsumerError::Callback(code)) => code,
        Err(OriginalRegexConsumerError::Command(error))
            if error.native_access_refusal.is_some() =>
        {
            interp.refuse_native_access(error.native_access_refusal.unwrap())
        }
        Err(OriginalRegexConsumerError::Command(error)) if error.command_error.is_some() => {
            interp.report_cmd_error(error.command_error.unwrap())
        }
        Err(OriginalRegexConsumerError::Command(error)) => match error.code {
            Some(code) => interp.error_with_code(&error.message, code),
            None => interp.set_error(&error.message),
        },
    }
}

impl tcl_cmd_core::native_jim_lsearch::NativeJimLsearchObjects for Interp {
    type Hold = obj::Owned;
    type Accumulator = obj::Owned;
    fn jim_search_bytes(
        &mut self,
        value: &*mut TclObj,
    ) -> Result<std::rc::Rc<[u8]>, tcl_syntax::value::ValueError> {
        crate::native_source::bind_context(*value, &self.native_jim_object_context()?)?;
        crate::dict::native_object_bytes(
            *value,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .map(std::rc::Rc::from)
    }
    fn jim_search_character_count(
        &mut self,
        value: &*mut TclObj,
    ) -> Result<usize, tcl_syntax::value::ValueError> {
        obj::native_character_count(
            *value,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
            tcl_registry::native_string_length::NativeStringLengthRepresentation::JimCachedString,
        )
    }
    fn jim_search_option(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<usize, tcl_cmd_core::CmdError> {
        let table = tcl_registry::native_index_lookup::NativeStaticIndexTable::supported_backend(
            &tcl_cmd_core::native_jim_lsearch::OPTIONS,
        );
        match self.native_jim_enum_from_original(
            *original,
            &table,
            tcl_registry::native_jim_enum::NativeJimEnumFlags::options(true),
            None,
        )? {
            Ok(index) => Ok(index),
            Err(message) => Err(
                tcl_cmd_core::CmdError::new_bytes(message.unwrap_or_default())
                    .with_native_string_result(
                        tcl_syntax::native_string::NativeStringProtocol::Jim084,
                    ),
            ),
        }
    }
    fn jim_search_hold(
        &mut self,
        value: &*mut TclObj,
    ) -> Result<obj::Owned, tcl_syntax::value::ValueError> {
        obj::check_native_liveness(*value)?;
        Ok(obj::Owned::retain(*value))
    }
    fn jim_search_borrow(&self, value: &*mut TclObj) -> *mut TclObj {
        *value
    }
    fn jim_search_elements(
        &mut self,
        value: &*mut TclObj,
    ) -> Result<Vec<*mut TclObj>, tcl_syntax::value::ValueError> {
        <Self as tcl_syntax::value::ValueOps>::list_elements(self, value)
    }
    fn jim_search_current_elements(
        &self,
        value: &*mut TclObj,
    ) -> Result<Vec<*mut TclObj>, tcl_syntax::value::ValueError> {
        obj::check_native_liveness(*value)?;
        if crate::list::native_list_backing(*value).is_none() {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "Jim lsearch callback changed selected List primary",
            ));
        }
        crate::list::list_elements_native_checked(
            *value,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
    }
    fn jim_search_regexp_command(&mut self) -> *mut TclObj {
        obj::new_string_bytes(b"regexp")
    }
    fn jim_search_group(&mut self, values: Vec<*mut TclObj>) -> *mut TclObj {
        self.new_list_object(&values)
    }
    fn jim_search_begin(&mut self) -> obj::Owned {
        obj::Owned::fresh(self.new_list_object(&[]))
    }
    fn jim_search_append(
        &mut self,
        list: &mut obj::Owned,
        values: &[*mut TclObj],
    ) -> Result<(), tcl_syntax::value::ValueError> {
        crate::list::append_prepared_native_elements(
            list.as_ptr(),
            values,
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
    }
    fn jim_search_finish(
        &mut self,
        result: obj::Owned,
    ) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        self.set_result(result.as_ptr());
        Ok(self.get_obj_result())
    }
    fn jim_search_publish(
        &mut self,
        value: &*mut TclObj,
    ) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        obj::check_native_liveness(*value)?;
        self.set_result(*value);
        Ok(self.get_obj_result())
    }
    fn jim_search_current_result(&self) -> Result<*mut TclObj, tcl_syntax::value::ValueError> {
        let value = self.get_obj_result();
        obj::check_native_liveness(value)?;
        Ok(value)
    }
}

/// `lsort ?-option value ...? list` — sort the list (`Tcl_LsortObjCmd`).
/// `lsort ?-option value ...? list` — a thin adapter over the shared
/// [`tcl_cmd_core::lsort`] core. Non-command modes are sorted+built entirely in
/// the core; `-command` is split (the core prepares, this adapter runs the merge
/// sort over the user comparator via `lsort_cmd_compare`, then the core builds).
fn lsort(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_cmd_core::index::NativeIndexOps;
    if interp.index_c_version().is_some() || interp.index_jim_sort_selected() {
        let release = interp.runtime_version();
        return match tcl_cmd_core::lsort::run_original(
            interp,
            &argv[1..],
            release,
            lsort_compare_original,
            Interp::report_cmd_error,
        ) {
            Ok(value) => {
                interp.set_result(value);
                Code::Ok
            }
            Err(code) => code,
        };
    }
    use tcl_cmd_core::lsort::{build_command, prepare, sort_command, Lsort};
    let release = interp.runtime_version();
    let job = match prepare(interp, &argv[1..], release) {
        Ok(Lsort::Done(v)) => {
            interp.set_result(v);
            return Code::Ok;
        }
        Ok(Lsort::Original(result)) => {
            let (tcl_cmd_core::index::IndexResult::Borrowed(value)
            | tcl_cmd_core::index::IndexResult::Created(value)
            | tcl_cmd_core::index::IndexResult::Detached(value)
            | tcl_cmd_core::index::IndexResult::ReturnedOwnedTransport(value)) = result;
            interp.set_result(value);
            return Code::Ok;
        }
        Ok(Lsort::Command(job)) => job,
        Err(e) => {
            if let Some(error) = e.native_access_refusal {
                return interp.refuse_native_access(error);
            }
            if let Some(error) = e.command_error {
                return interp.report_cmd_error(error);
            }
            return interp.set_error(&e.message);
        }
    };
    // `-command`: pre-split the comparison prefix into words, run the reentrant
    // merge sort over the user comparator (which evaluates Tcl), then build.
    let words = match list::list_elements(job.cmd_prefix) {
        Ok(v) => v.iter().map(|&w| obj_bytes(w)).collect::<Vec<_>>(),
        Err(e) => return interp.report_cmd_error(e.into()),
    };
    let mut job = job;
    if let Err(c) = sort_command(&mut job, |a, b| {
        lsort_cmd_compare(
            interp,
            &words,
            *tcl_cmd_core::index::key_value::<Interp>(a),
            *tcl_cmd_core::index::key_value::<Interp>(b),
        )
    }) {
        return c;
    }
    let v = build_command(interp, &job);
    interp.set_result(v);
    Code::Ok
}

/// Dispatch the genuine command object and get the actual returned original.
fn lsort_compare_original(
    interp: &mut Interp,
    original: &<Interp as tcl_cmd_core::index::NativeIndexOps>::IndexOperation,
    command: &*mut TclObj,
) -> Result<core::cmp::Ordering, Code> {
    use tcl_cmd_core::index::NativeIndexOps;
    let code = if interp.index_jim_sort_selected() {
        interp.eval_body_obj(*command)
    } else {
        let words = interp
            .index_list_elements(command)
            .map_err(|error| interp.report_cmd_error(error))?;
        interp.dispatch(&words)
    };
    interp
        .check_index_operation(original)
        .map_err(|error| interp.report_cmd_error(error))?;
    if code != Code::Ok {
        if interp.index_c_version().is_some() {
            interp.append_error_info_context(b"(-compare command)");
        }
        return Err(code);
    }
    lsort_original_comparison_result(interp, original)
        .map_err(|error| interp.report_cmd_error(error))
}

fn lsort_original_comparison_result(
    interp: &mut Interp,
    original: &<Interp as tcl_cmd_core::index::NativeIndexOps>::IndexOperation,
) -> Result<core::cmp::Ordering, tcl_cmd_core::CmdError> {
    use tcl_cmd_core::index::NativeIndexOps;
    use tcl_cmd_core::CmdError;
    use tcl_registry::native_numeric_error::NativeNumericCommandPurpose;
    use tcl_syntax::{scalar_getter::NativeScalarGetterValue, value::ValueError};
    let result = interp.get_obj_result();
    interp.check_index_operation(original)?;
    obj::check_native_liveness(result)?;
    let lifetime = obj::NativeObjectLifetime::retain(result);
    let dialect = interp.native_invocation_dialect();
    let selected = dialect
        .native_numeric_command_protocol(NativeNumericCommandPurpose::ListSortComparisonResult)
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original sort comparison-result purpose",
        ))?;
    let reached = {
        let host = interp.host();
        crate::typed_value::native_scalar_getter_with_environment_and_currency(
            result,
            dialect,
            selected.getter_kind(),
            host.numeric_environment(),
            Some(original.original_currency()),
        )
    };
    let live = obj::check_native_liveness(lifetime.as_ptr());
    drop(lifetime);
    interp.check_index_operation(original)?;
    live?;
    let reached = reached.map_err(|error| {
        if !matches!(error, ValueError::NativeScalarGetter(_)) {
            return CmdError::from(error);
        }
        selected.comparison_result_error().map_or_else(
            || CmdError::from(error),
            |wrapper| CmdError::with_error_code_bytes(wrapper.message, wrapper.error_code),
        )
    })?;
    let NativeScalarGetterValue::Wide(integer) = reached else {
        return Err(ValueError::ScalarNumericInputUnavailable.into());
    };
    Ok(integer.cmp(&0))
}

/// Evaluate `<prefix words...> a b` and read its integer result (the `-command`
/// comparator). Returns the sign as an `i32`.
fn lsort_cmd_compare(
    interp: &mut Interp,
    words: &[Vec<u8>],
    a: *mut TclObj,
    b: *mut TclObj,
) -> Result<i32, Code> {
    use crate::interp::new_string;
    let mut call: Vec<*mut TclObj> = Vec::with_capacity(words.len() + 2);
    for w in words {
        call.push(new_string(w));
    }
    call.push(a);
    call.push(b);
    for &o in &call {
        unsafe { obj::incr_ref_count(o) };
    }
    let code = interp.dispatch(&call);
    for &o in &call {
        unsafe { obj::decr_ref_count(o) };
    }
    if code != Code::Ok {
        return Err(code);
    }
    // C reads the comparator's result as a C `int`.
    match crate::typed_value::int(interp.result_obj()) {
        Ok(order) => Ok(order.signum()),
        Err(_) => Err(interp.error_with_code(
            b"-compare command returned non-integer result",
            b"TCL OPERATION LSORT COMPARISONFAILED",
        )),
    }
}

/// Free a freshly created (`rc 0`) object not stored anywhere.
fn drop_fresh(obj: *mut TclObj) {
    // SAFETY: `obj` is a live rc-0 object; retain-then-release frees it cleanly.
    unsafe {
        obj::incr_ref_count(obj);
        obj::decr_ref_count(obj);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_jim_lsearch_controls;

#[cfg(all(test, have_tommath))]
#[path = "cmd_list/native_original_option_index_controls.rs"]
mod native_original_option_index_controls;

#[cfg(all(test, have_tommath))]
mod native_original_stride_controls;

#[cfg(all(test, have_tommath))]
mod native_checked_search_sort_controls;

#[cfg(test)]
mod native_original_numeric_controls;

#[cfg(test)]
#[path = "cmd_list/native_original_group_command_controls.rs"]
mod native_original_group_command_controls;

#[cfg(test)]
mod c86_split_tests;
