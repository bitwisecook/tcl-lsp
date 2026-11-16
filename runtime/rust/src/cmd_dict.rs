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

//! The `dict` ensemble — `create`/`get`/`getdef`/`set`/`replace`/
//! `remove`/`exists`/`unset`/`size`/`keys`/`values`/`merge`/`filter`/`for`/
//! `map`/`update`/`with`/`append`/`lappend`/`incr`/`info`, over the
//! [`crate::dict`] value type.
//!
//! `dict set`/`unset`/`update`/`with` mutate a dict **variable** (copy-on-write,
//! like `lappend`); the rest read dict **values**. `get`/`exists`/`getdef` take
//! a key *path* (nested dicts); `keys`/`values`/`filter` glob-filter.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::dict;
use crate::frame::VarError;
use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};
use crate::parse;

/// Register the `dict` ensemble.
pub fn install(interp: &mut Interp) {
    let admitted = crate::environment::release_subcommands(
        interp.native_ensemble_profile_name(),
        "dict",
        DICT_SUBS,
    );
    interp.register_stock_ensemble(
        tcl_registry::invocation_words::EnsembleImplementationFamily::Dict,
        b"dict",
        dict_cmd,
        STOCK_MEMBERS,
        admitted,
    );
}

const STOCK_MEMBERS: &[(&[u8], crate::interp::BuiltinFn)] = &[
    (b"append", stock_append),
    (b"create", stock_create),
    (b"exists", stock_exists),
    (b"filter", stock_filter),
    (b"for", stock_for),
    (b"get", stock_get),
    (b"getdef", stock_getdef),
    (b"getwithdefault", stock_getwithdefault),
    (b"incr", stock_incr),
    (b"info", stock_info),
    (b"keys", stock_keys),
    (b"lappend", stock_lappend),
    (b"map", stock_map),
    (b"merge", stock_merge),
    (b"remove", stock_remove),
    (b"replace", stock_replace),
    (b"set", stock_set),
    (b"size", stock_size),
    (b"unset", stock_unset),
    (b"update", stock_update),
    (b"values", stock_values),
    (b"with", stock_with),
];

fn stock_append(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"append"], dict_cmd)
}

fn stock_create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"create"], dict_cmd)
}

fn stock_exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"exists"], dict_cmd)
}

fn stock_filter(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"filter"], dict_cmd)
}

fn stock_for(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"for"], dict_cmd)
}

fn stock_get(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"get"], dict_cmd)
}

fn stock_getdef(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"getdef"], dict_cmd)
}

fn stock_getwithdefault(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"getwithdefault"], dict_cmd)
}

fn stock_incr(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"incr"], dict_cmd)
}

fn stock_info(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"info"], dict_cmd)
}

fn stock_keys(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"keys"], dict_cmd)
}

fn stock_lappend(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"lappend"], dict_cmd)
}

fn stock_map(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"map"], dict_cmd)
}

fn stock_merge(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"merge"], dict_cmd)
}

fn stock_remove(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"remove"], dict_cmd)
}

fn stock_replace(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"replace"], dict_cmd)
}

fn stock_set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"set"], dict_cmd)
}

fn stock_size(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"size"], dict_cmd)
}

fn stock_unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"unset"], dict_cmd)
}

fn stock_update(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"update"], dict_cmd)
}

fn stock_values(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"values"], dict_cmd)
}

fn stock_with(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    interp.invoke_stock_worker(argv, &[b"dict", b"with"], dict_cmd)
}

fn dict_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match crate::interp::native_arguments::OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let code = dict_cmd_entered(interp, argv, &arguments);
    arguments.finish(code)
}

fn dict_cmd_entered(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    arguments: &crate::interp::native_arguments::OriginalArguments,
) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args_for_invocation(argv, b"subcommand ?arg ...?");
    }
    let word = match arguments.bytes(1) {
        Ok(word) => word,
        Err(error) => return interp.report_cmd_error(error),
    };
    // `dict` is a `TclMakeEnsemble` command: exact match, else a unique
    // prefix, so `dict k` is `dict keys`.
    // `getdef`/`getwithdefault` are Tcl 9 (TIP 342), so the table is the
    // emulated release's: under an 8.6 pin they must neither resolve nor make
    // `dict g` — a word that has nothing to do with them — ambiguous.
    let subs = crate::environment::release_subcommands(
        interp.native_ensemble_profile_name(),
        "dict",
        DICT_SUBS,
    );
    let Some(index) = tcl_cmd_core::ensemble::resolve_subcommand(subs, &word, true) else {
        return interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
            subs,
            &word,
            true,
            b"::tcl::dict",
        ));
    };
    let sub = subs[index];
    if let Ok(selected) = std::str::from_utf8(sub) {
        if let Some(dispatch) = tcl_registry::dictionary_scope::scripted_dictionary_dispatch(
            interp.native_invocation_dialect(),
            selected,
            &word,
            argv.len() - 2,
        ) {
            let Some(count) = dispatch.arguments() else {
                return interp.wrong_args(dispatch.usage());
            };
            let target = crate::obj::Owned::fresh(crate::interp::new_string(dispatch.command()));
            let mut words = Vec::with_capacity(count + 1);
            words.push(target.as_ptr());
            words.extend_from_slice(&argv[2..2 + count]);
            return interp.dispatch(&words);
        }
    }
    // Pure dict subcommands now live in the shared command core; the runtime is
    // a thin adapter. Variable-mutating subcommands fall through to the legacy
    // match below.
    if let Ok(sub_str) = std::str::from_utf8(sub) {
        let header = match interp.argument_usage_prefix(argv, 2) {
            Ok(header) => header,
            Err(code) => return code,
        };
        if let Some(result) =
            tcl_cmd_core::dict::dispatch_canon(interp, &header, sub_str, &argv[2..])
        {
            return match result {
                Ok(v) => {
                    if sub_str == "keys" {
                        if let Err(error) =
                            interp.record_original_namespace_info_dictionary_keys(argv, v)
                        {
                            drop_fresh(v);
                            return interp.report_cmd_error(error.into());
                        }
                    }
                    interp.set_result(v);
                    Code::Ok
                }
                // The shared core decodes through the *list* codec and reports
                // errors by message only. A dict value-parse failure has to be
                // re-worded to C's dict spelling and given its `-errorcode`, so
                // `catch … optsVar` sees `TCL VALUE DICTIONARY …` and not
                // `NONE` (dict-4.x) — exactly what the mutating path's
                // [`bad_dict`] already does from its own `DictError`.
                //
                // The read path (`size`/`info`/`get`/`keys`/`values`/`exists`/
                // `merge`/`filter`/`replace`/`remove`/`getdef`) reaches C's
                // parser only through here, so without the re-wording every
                // one of them would report the list noun.
                Err(error) => interp.report_cmd_error(error),
            };
        }
    }
    match sub {
        b"create" => create(interp, argv),
        b"get" => get(interp, argv),
        b"set" => set(interp, argv),
        b"exists" => exists(interp, argv),
        b"unset" => unset(interp, argv),
        b"size" => size(interp, argv),
        b"keys" => keys(interp, argv),
        b"values" => values(interp, argv),
        b"merge" => merge(interp, argv),
        b"filter" => filter(interp, argv),
        b"for" => for_(interp, argv),
        b"map" => map(interp, argv),
        b"update" => update(interp, argv),
        b"with" => with(interp, argv),
        b"append" => append(interp, argv),
        b"lappend" => lappend(interp, argv),
        b"incr" => incr(interp, argv),
        // Unreachable: every name in `DICT_SUBS` is handled above or by the
        // shared core.
        other => interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
            DICT_SUBS,
            other,
            true,
            b"::tcl::dict",
        )),
    }
}

/// `dict`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it — the
/// full Tcl 9 table.
const DICT_SUBS: &[&[u8]] = &[
    b"append",
    b"create",
    b"exists",
    b"filter",
    b"for",
    b"get",
    b"getdef",
    b"getwithdefault",
    b"incr",
    b"info",
    b"keys",
    b"lappend",
    b"map",
    b"merge",
    b"remove",
    b"replace",
    b"set",
    b"size",
    b"unset",
    b"update",
    b"values",
    b"with",
];

// read subcommands (operate on a dict value)

/// `dict create ?key value ...?`
fn create(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let rest = &argv[2..];
    if rest.len() % 2 != 0 {
        return interp.wrong_args_for_prefix(argv, 2, b"?key value ...?");
    }
    let pairs: Vec<(*mut TclObj, *mut TclObj)> =
        rest.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    interp.set_result(dict::new_dict_obj(&pairs));
    Code::Ok
}

/// `dict get dictValue ?key?` — the value for `key`, or the whole dict if no key.
fn get(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictionary ?key ...?");
    }
    if argv.len() == 3 {
        interp.set_result(argv[2]); // whole dict
        return Code::Ok;
    }
    // Drill the key path: each key descends one nested dict (the value at each
    // step is owned by its parent, alive up the chain to `argv[2]`).
    let mut cur = argv[2];
    for &k in &argv[3..] {
        let key = obj_bytes(k);
        match dict::dict_get(cur, &key) {
            Ok(Some(v)) => cur = v,
            Ok(None) => return key_not_known(interp, &key),
            Err(e) => return bad_dict(interp, e),
        }
    }
    interp.set_result(cur);
    Code::Ok
}

/// `dict exists dictValue key`
fn exists(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictionary key ?key ...?");
    }
    // Drill the key path; a missing key or a non-dict along the way → 0 (Tcl
    // `dict exists` reports false rather than erroring).
    let keys = &argv[3..];
    let mut cur = argv[2];
    for (i, &k) in keys.iter().enumerate() {
        let key = obj_bytes(k);
        match dict::dict_exists(cur, &key) {
            Ok(true) if i + 1 == keys.len() => {
                interp.set_result_bytes(b"1");
                return Code::Ok;
            }
            Ok(true) => match dict::dict_get(cur, &key) {
                Ok(Some(v)) => cur = v,
                _ => break,
            },
            _ => break,
        }
    }
    interp.set_result_bytes(b"0");
    Code::Ok
}

/// `dict size dictionary`
fn size(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictionary");
    }
    match dict::dict_size(argv[2]) {
        Ok(n) => {
            interp.set_result(obj::new_wide_int_obj(n as i64));
            Code::Ok
        }
        Err(e) => bad_dict(interp, e),
    }
}

/// Glob-filter `items` by the optional pattern at `argv[3]` (`keys`/`values`
/// share this), setting the result to the matching list.
fn glob_filtered_result(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    items: Vec<*mut TclObj>,
) -> Code {
    let filtered: Vec<*mut TclObj> = match argv.get(3) {
        Some(&p) => {
            let pat = obj_bytes(p);
            let pat_s = String::from_utf8_lossy(&pat);
            items
                .into_iter()
                .filter(|&o| {
                    tcl_syntax::glob::string_match(&pat_s, &String::from_utf8_lossy(&obj_bytes(o)))
                })
                .collect()
        }
        None => items,
    };
    interp.set_result(interp.new_list_object(&filtered));
    Code::Ok
}

/// `dict keys dictionary ?pattern?` — keys in insertion order, glob-filtered.
fn keys(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictionary ?pattern?");
    }
    match dict::dict_keys(argv[2]) {
        Ok(ks) => glob_filtered_result(interp, argv, ks),
        Err(e) => bad_dict(interp, e),
    }
}

/// `dict values dictionary ?pattern?` — values in insertion order, glob-filtered.
fn values(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictionary ?pattern?");
    }
    match dict::dict_pairs(argv[2]) {
        Ok(pairs) => {
            let vs: Vec<*mut TclObj> = pairs.iter().map(|&(_, v)| v).collect();
            glob_filtered_result(interp, argv, vs)
        }
        Err(e) => bad_dict(interp, e),
    }
}

/// `dict merge ?dictValue ...?` — left to right; later values win, first-seen
/// key position is kept.
fn merge(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let acc = dict::new_dict_obj(&[]); // rc 0
    unsafe { obj::incr_ref_count(acc) }; // own it while building
    for &d in &argv[2..] {
        let pairs = match dict::dict_pairs(d) {
            Ok(p) => p,
            Err(e) => {
                unsafe { obj::decr_ref_count(acc) };
                return bad_dict(interp, e);
            }
        };
        for (k, v) in pairs {
            // acc is unshared (we hold the only ref) → in-place set is sound.
            if let Err(e) = dict::dict_set(acc, k, v) {
                unsafe { obj::decr_ref_count(acc) };
                return bad_dict(interp, e);
            }
        }
    }
    interp.set_result(acc); // retains acc into the result
    unsafe { obj::decr_ref_count(acc) }; // drop our build-time ref
    Code::Ok
}

/// Copy `src`'s pairs into a fresh owned dict (rc 1, caller balances) for
/// in-place mutation. `None` (after stamping `bad_dict`) on a malformed dict.
fn copy_dict(interp: &mut Interp, src: *mut TclObj) -> Option<*mut TclObj> {
    let pairs = match dict::dict_pairs(src) {
        Ok(p) => p,
        Err(e) => {
            bad_dict(interp, e);
            return None;
        }
    };
    let acc = dict::new_dict_obj(&[]);
    unsafe { obj::incr_ref_count(acc) };
    for (k, v) in pairs {
        let _ = dict::dict_set(acc, k, v);
    }
    Some(acc)
}

/// `dict filter dictionary key|value ?globPattern ...?` (glob forms) or
/// `dict filter dictionary script {keyVar valueVar} body` (predicate form):
/// the entries kept, as a new dict.
fn filter(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // Only the `script` filter type reaches here: the shared core
    // (`tcl_cmd_core::dict`) handles `key`/`value` (pure glob), the bad-filterType
    // error, and the missing-filterType arg error, so `dispatch_canon` returns
    // `None` only for `script`. `argv` is therefore `[string filter dict script
    // …]` (≥ 4). Error order matches the original: the dict is parsed before the
    // script-form arg-count check.
    let pairs = match dict::dict_pairs(argv[2]) {
        Ok(p) => p,
        Err(e) => return bad_dict(interp, e),
    };
    if argv.len() != 6 {
        return interp
            .wrong_args(b"dict filter dictionary script {keyVarName valueVarName} filterScript");
    }
    // The two variable names are parsed as a *list*: a malformed list surfaces
    // the list parse error verbatim (dict-17.20), then a count other than two is
    // the dict-filter syntax error (dict-17.19).
    let vars = match crate::parse::split_list(&obj_bytes(argv[4])) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message()),
    };
    if vars.len() != 2 {
        return interp.error_with_code(
            b"must have exactly two variable names",
            b"TCL SYNTAX dict filter",
        );
    }
    let mut kept: Vec<(*mut TclObj, *mut TclObj)> = Vec::new();
    for (k, v) in pairs {
        if interp.var_set(&vars[0], k).is_err() || interp.var_set(&vars[1], v).is_err() {
            return interp.set_error(b"couldn't set dict filter variable");
        }
        // The body's completion code drives the loop (C's `DictFilterCmd` script
        // case): OK ⇒ keep iff its result is true; CONTINUE ⇒ skip; BREAK ⇒ stop,
        // return what's kept; ERROR / RETURN / other ⇒ propagate.
        match interp.eval_control_body(argv[5]) {
            Code::Ok => {
                // The body result is coerced with the interpreter's canonical
                // Tcl boolean parser (the one `if`/`while`/`expr` use), so a
                // numeric false like `0x0`/`0.0` drops the pair and a
                // non-boolean result raises `expected boolean value` — matching
                // C's `Tcl_GetBooleanFromObj` rather than a loose string test.
                match dict_filter_bool(interp.get_obj_result(), interp.native_invocation_dialect())
                {
                    Ok(true) => kept.push((k, v)),
                    Ok(false) => {}
                    Err(e) => return interp.report_cmd_error(e.into()),
                }
            }
            Code::Continue => {}
            Code::Break => break,
            other => return other,
        }
    }
    interp.set_result(dict::new_dict_obj(&kept));
    Code::Ok
}

/// Coerce a `dict filter … script` body result to a boolean like C's
/// `Tcl_GetBooleanFromObj` — the runtime's one typed-read owner, so this
/// accepts exactly what `if` and `expr` do, and refuses with the same
/// message and `-errorcode` (tclsh: `TCL VALUE NUMBER` for a non-boolean).
fn dict_filter_bool(
    o: *mut TclObj,
    dialect: tcl_registry::InvocationDialect,
) -> Result<bool, tcl_syntax::value::ValueError> {
    crate::typed_value::native_boolean(o, dialect)
}

// variable-mutating subcommands (copy-on-write)

/// The dict object to mutate for `dictVar`: the variable's (mutated in place if
/// unshared), a COW copy, or a fresh empty dict. Returns `(obj, is_new)`.
/// Read the dict-variable `name`, splitting an `arr(elem)` reference so a dict
/// command can mutate an array element (and pick up a TIP 508 array default).
fn dict_var_get(interp: &Interp, name: &[u8]) -> Option<*mut TclObj> {
    let (base, elem) = crate::frame::split_array_ref(name);
    match elem {
        Some(k) => interp.var_get_elem(&base, &k),
        None => interp.var_get(&base),
    }
}

/// Write the dict-variable `name`, splitting an `arr(elem)` reference.
fn dict_var_set(interp: &mut Interp, name: &[u8], obj: *mut TclObj) -> Result<(), VarError> {
    let (base, elem) = crate::frame::split_array_ref(name);
    match elem {
        Some(k) => interp.var_set_elem(&base, &k, obj),
        None => interp.var_set(&base, obj),
    }
}

fn working_dict(interp: &mut Interp, name: &[u8]) -> Result<(*mut TclObj, bool), Code> {
    // A constant cannot be the target of a mutating dict op; reject before the
    // in-place update would bypass the store-time check.
    if let Some(c) = interp.const_write_check(name) {
        return Err(c);
    }
    Ok(match dict_var_get(interp, name) {
        None => (dict::new_dict_obj(&[]), true),
        Some(o) if obj::is_shared(o) => (obj::duplicate(o), true),
        Some(o) => (o, false),
    })
}

/// Store a freshly built dict back into `dictVar` (when `is_new`) and set it as
/// the result.
fn store_dict(interp: &mut Interp, name: &[u8], target: *mut TclObj, is_new: bool) -> Code {
    if is_new && dict_var_set(interp, name, target).is_err() {
        drop_fresh(target);
        return cant_set(interp, name);
    }
    interp.set_result(target);
    Code::Ok
}

fn native_variable_objects(
    interp: &mut Interp,
    name: *mut TclObj,
) -> Result<Option<(Vec<u8>, crate::value_ops::RuntimeDictionaryObjects)>, Code> {
    if interp
        .native_invocation_dialect()
        .native_dictionary_append_inputs()
        .is_none()
    {
        return Err(interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary command protocol",
            )
            .into(),
        ));
    }
    let objects = crate::value_ops::RuntimeDictionaryObjects::selected(interp)
        .map_err(|error| interp.report_cmd_error(error))?;
    if objects.protocol.is_jim084() {
        return Ok(None);
    }
    let name = tcl_syntax::value::ValueOps::native_string_bytes(interp, &name)
        .map_err(|error| interp.report_cmd_error(error.into()))?;
    Ok(Some((name.to_vec(), objects)))
}

/// `dict append dictVarName key ?value ...?` — string-append to the key's value.
fn append(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictVarName key ?value ...?");
    }
    match native_variable_objects(interp, argv[2]) {
        Ok(Some((name, objects))) => {
            let dialect = interp.native_invocation_dialect();
            let Some(policy) = dialect.native_dictionary_append_policy() else {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native dictionary append member policy",
                    )
                    .into(),
                );
            };
            let Some(issued) = dialect.native_object_append_protocol(None) else {
                return interp.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native dictionary member append",
                    )
                    .into(),
                );
            };
            let append_objects = crate::value_ops::RuntimeAppendObjects {
                dialect,
                binary_recipe: dialect.byte_array_string_recipe(None),
            };
            let key = crate::value_ops::RuntimeAppendValue::borrowed(argv[3]);
            let sources: Vec<_> = argv[4..]
                .iter()
                .copied()
                .map(crate::value_ops::RuntimeAppendValue::borrowed)
                .collect();
            return interp.dictionary_variable_update(&name, &objects, |_, prepared| {
                tcl_cmd_core::native_dictionary::update_prepared_member_if(
                    &objects,
                    prepared,
                    &key,
                    |original| {
                        use tcl_registry::native_dictionary::{
                            NativeDictionaryAppendInputs as Inputs,
                            NativeDictionaryEmptyAppend as Empty,
                            NativeDictionaryMissingAppendMember as Missing,
                        };
                        if sources.is_empty()
                            && original.is_some()
                            && policy.empty == Empty::StoreOnly
                        {
                            return Ok(None);
                        }
                        let empty = crate::value_ops::RuntimeAppendValue::fresh_string(b"");
                        let original = match policy.missing {
                            Missing::FreshEmptyReceiver => original.or(Some(&empty)),
                            Missing::AdoptPreparedInput => original,
                        };
                        let result = match policy.inputs {
                            Inputs::Concatenate(cat) if sources.len() > 1 => {
                                let source = tcl_cmd_core::native_cat::concatenate(
                                    &append_objects,
                                    cat.recipe(),
                                    &sources,
                                    true,
                                )?;
                                tcl_cmd_core::native_append::append_dictionary_operands(
                                    &append_objects,
                                    issued.recipe(),
                                    original,
                                    std::slice::from_ref(&source),
                                )?
                            }
                            _ => tcl_cmd_core::native_append::append_dictionary_operands(
                                &append_objects,
                                issued.recipe(),
                                original,
                                &sources,
                            )?,
                        };
                        Ok(Some(result))
                    },
                )
            });
        }
        Ok(None) => {}
        Err(code) => return code,
    }
    let name = obj_bytes(argv[2]);
    let key = argv[3];
    let (target, is_new) = match working_dict(interp, &name) {
        Ok(x) => x,
        Err(c) => return c,
    };
    let mut buf = match dict::dict_get(target, &obj_bytes(key)) {
        Ok(Some(v)) => obj_bytes(v),
        _ => Vec::new(),
    };
    for &s in &argv[4..] {
        buf.extend_from_slice(&obj_bytes(s));
    }
    let val = crate::interp::new_string(&buf); // rc 0; dict_set retains
    if let Err(e) = dict::dict_set(target, key, val) {
        drop_fresh(val);
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp, e);
    }
    store_dict(interp, &name, target, is_new)
}

/// `dict lappend dictVarName key ?value ...?` — list-append to the key's value.
fn lappend(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictVarName key ?value ...?");
    }
    match native_variable_objects(interp, argv[2]) {
        Ok(Some((name, objects))) => {
            let key = crate::value_ops::RuntimeAppendValue::borrowed(argv[3]);
            return interp.dictionary_variable_update(&name, &objects, |_, prepared| {
                tcl_cmd_core::native_dictionary::update_prepared_member(
                    &objects,
                    prepared,
                    &key,
                    |original| {
                        let value = crate::list::append_native_elements(
                            original.map(crate::value_ops::RuntimeAppendValue::as_ptr),
                            &argv[4..],
                            objects.protocol,
                        )?;
                        Ok(crate::value_ops::RuntimeAppendValue::retain(value.as_ptr()))
                    },
                )
            });
        }
        Ok(None) => {}
        Err(code) => return code,
    }
    let name = obj_bytes(argv[2]);
    let key = argv[3];
    let (target, is_new) = match working_dict(interp, &name) {
        Ok(x) => x,
        Err(c) => return c,
    };
    let mut elems: Vec<*mut TclObj> = match dict::dict_get(target, &obj_bytes(key)) {
        Ok(Some(v)) => match crate::list::list_elements(v) {
            Ok(e) => e,
            // The existing value must be a valid list (e.g. `{` is not).
            Err(e) => {
                if is_new {
                    drop_fresh(target);
                }
                return interp.report_cmd_error(e.into());
            }
        },
        _ => Vec::new(),
    };
    elems.extend_from_slice(&argv[4..]);
    let val = interp.new_list_object(&elems); // rc 0; dict_set retains
    if let Err(e) = dict::dict_set(target, key, val) {
        drop_fresh(val);
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp, e);
    }
    store_dict(interp, &name, target, is_new)
}

/// `dict incr dictVarName key ?increment?` — integer-add to the key's value.
///
/// Both the stored value and the increment are read through the shared
/// bignum-aware `int_add` seam (the one scalar `incr` uses), so a radix literal
/// such as `0x10` is accepted and a sum past `i64::MAX` widens to a bignum
/// instead of wrapping — matching C's `TclIncrObj` rather than a decimal-only
/// `wrapping_add`.
fn incr(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 || argv.len() > 5 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictVarName key ?increment?");
    }
    match native_variable_objects(interp, argv[2]) {
        Ok(Some((name, objects))) => {
            let objects = objects.with_preparation(
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::IncrementCommand,
            );
            let increment = match crate::value_ops::RuntimeIncrementObjects::selected(interp) {
                Ok(increment) => increment,
                Err(error) => return interp.report_cmd_error(error),
            };
            let key = crate::value_ops::RuntimeAppendValue::borrowed(argv[3]);
            let default =
                (argv.len() == 4).then(|| crate::obj::Owned::fresh(obj::new_wide_int_obj(1)));
            let amount = crate::value_ops::RuntimeAppendValue::borrowed(
                argv.get(4)
                    .copied()
                    .unwrap_or_else(|| default.as_ref().unwrap().as_ptr()),
            );
            return interp.dictionary_variable_update(&name, &objects, |_, prepared| {
                tcl_cmd_core::native_dictionary::update_prepared_member(
                    &objects,
                    prepared,
                    &key,
                    |original| match original {
                        Some(value) => tcl_cmd_core::native_increment::increment(
                            &increment,
                            Some(value),
                            &amount,
                        ),
                        None if argv.len() == 5 => {
                            tcl_cmd_core::native_increment::missing_dictionary_member(
                                &increment, &amount,
                            )
                        }
                        None => Ok(amount.clone()),
                    },
                )
            });
        }
        Ok(None) => {}
        Err(code) => return code,
    }
    let name = obj_bytes(argv[2]);
    let key = argv[3];
    let (target, is_new) = match working_dict(interp, &name) {
        Ok(x) => x,
        Err(c) => return c,
    };
    // Current value object (borrowed from `target`), or `None` when the key is
    // absent — the seam then treats it as 0, as C's `dict incr` does.
    let cur = match dict::dict_get(target, &obj_bytes(key)) {
        Ok(Some(v)) => Some(v),
        _ => None,
    };
    // Coercion order matches C: an existing value goes through `TclIncrObj`
    // (current value, then increment); a new key only checks the increment is
    // an integer of any size.
    let checked = match cur {
        Some(cur) => [Some(cur), argv.get(4).copied()]
            .into_iter()
            .flatten()
            .try_for_each(crate::typed_value::incr_operand),
        None => argv
            .get(4)
            .copied()
            .map_or(Ok(()), crate::typed_value::integer),
    };
    if let Err(error) = checked {
        if is_new {
            drop_fresh(target);
        }
        return interp.report_typed_error(&error);
    }
    let one = obj::new_wide_int_obj(1);
    let amount = if argv.len() == 5 { argv[4] } else { one };
    let sum = tcl_syntax::value::ValueOps::int_add(interp, cur.as_ref(), &amount);
    drop_fresh(one); // the transient `1` (used or not) is no longer needed
    let sum = match sum {
        Ok(s) => s, // rc 0
        Err(e) => {
            if is_new {
                drop_fresh(target);
            }
            return crate::value_ops::integer_error(interp, e);
        }
    };
    if let Err(e) = dict::dict_set(target, key, sum) {
        drop_fresh(sum);
        if is_new {
            drop_fresh(target);
        }
        return bad_dict(interp, e);
    }
    store_dict(interp, &name, target, is_new)
}

/// `dict set dictVarName key ?key ...? value` — set the value at a (possibly
/// nested) key path in the dict held by the variable, creating intermediate
/// dicts along the path as needed.
fn set(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 5 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictVarName key ?key ...? value");
    }
    if interp
        .native_invocation_dialect()
        .native_dictionary_path_publication()
        == Some(
            tcl_registry::native_dictionary::NativeDictionaryPathPublication::JimOriginalVariable,
        )
    {
        return original_jim_dictionary_path(
            interp,
            argv,
            tcl_registry::native_dictionary::NativeDictionaryRootPurpose::VariableSet,
        );
    }
    match native_variable_objects(interp, argv[2]) {
        Ok(Some((name, objects))) => {
            let keys: Vec<_> = argv[3..argv.len() - 1]
                .iter()
                .copied()
                .map(crate::value_ops::RuntimeAppendValue::borrowed)
                .collect();
            let value = crate::value_ops::RuntimeAppendValue::borrowed(argv[argv.len() - 1]);
            return interp.dictionary_variable_update(&name, &objects, |_, prepared| {
                tcl_cmd_core::native_dictionary::set_prepared_path(&objects, prepared, &keys, value)
            });
        }
        Ok(None) => {}
        Err(code) => return code,
    }
    interp.report_cmd_error(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original dictionary path worker",
        )
        .into(),
    )
}

/// Reach the same selected Jim root/cell path worker with original argv.
fn original_jim_dictionary_path(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    purpose: tcl_registry::native_dictionary::NativeDictionaryRootPurpose,
) -> Code {
    let arguments = match crate::interp::native_arguments::OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error),
    };
    let (keys, value) =
        if purpose == tcl_registry::native_dictionary::NativeDictionaryRootPurpose::VariableSet {
            (&argv[3..argv.len() - 1], Some(argv[argv.len() - 1]))
        } else {
            (&argv[3..], None)
        };
    let reached =
        interp.mutate_original_jim_dictionary_path(argv[2], keys, value, purpose, &arguments);
    if let Err(error) = arguments.check() {
        return interp.report_cmd_error(error);
    }
    let code = match reached {
        Ok(Some(root)) => {
            interp.set_result(root);
            Code::Ok
        }
        Ok(None) => interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "generic Jim dictionary path publication",
            )
            .into(),
        ),
        Err(error) => error.report(interp),
    };
    arguments.finish(code)
}

/// One level of a `dict_path_set` descent, in the order the
/// iterative rewrite (below) needs to unwind it: the parent dict, the key
/// this level's `sub` sits under in that parent, `sub` itself, and whether
/// `sub` was freshly duplicated/created (so an aborted path knows whether it
/// owns `sub` and must free it, vs. `sub` being a borrow already owned by its
/// parent).
type DictPathLevel = (*mut TclObj, *mut TclObj, *mut TclObj, bool);

/// Drop every freshly duplicated/created sub-dict still in `chain` that has
/// not yet been re-bound into its parent, after a multi-segment `dict
/// set`/`dict unset` path failed partway through. The iterative form of the
/// original recursive `dict_path_set`'s per-frame
/// `if sub_fresh { drop_fresh(sub) }` unwind on the way back out of a failed
/// recursive call — same bookkeeping, expressed over an
/// explicit stack instead of the native call stack.
fn unwind_fresh_chain(chain: &[DictPathLevel]) {
    for &(_, _, sub, fresh) in chain {
        if fresh {
            drop_fresh(sub);
        }
    }
}

/// Set `value` at the key path `keys` (len ≥ 1) within the **unshared** dict
/// `dict`, descending through (and copy-on-write replacing) intermediate
/// sub-dicts, creating empty ones for missing path segments.
///
/// Iterative, not recursive: `dict set d {*}[lrepeat N k] v` makes
/// `keys.len()` — and so the native recursion depth a recursive
/// implementation would cost — trivially attacker-controlled via `{*}`
/// argument expansion. A recursive descend-then-rebind shape (one Rust
/// stack-frame group per key segment) has no depth cap, so pathologically
/// deep input could abort the process with an uncatchable native-stack
/// overflow. The explicit loop + stack here removes the recursion (and so
/// the whole crash class) rather than merely bounding it, so there is no
/// `MAX_X_DEPTH` to calibrate: an arbitrarily long path just does more work
/// (heap-bounded, like any other `Vec`-backed loop), not more native stack.
fn dict_path_set(
    dict: *mut TclObj,
    keys: &[*mut TclObj],
    value: *mut TclObj,
) -> Result<(), dict::DictError> {
    if let [last] = keys {
        return dict::dict_set(dict, *last, value);
    }
    // Descend through every intermediate key segment, recording each level
    // (the sub-dict for `head`: an unshared one we can mutate — copy a shared
    // one, create an empty one for a missing segment) so the loop below can
    // walk back up and re-bind the (possibly modified) chain into its parents,
    // or unwind it cleanly on failure.
    let mut chain: Vec<DictPathLevel> = Vec::with_capacity(keys.len() - 1);
    let mut cur = dict;
    for &head in &keys[..keys.len() - 1] {
        let got = match dict::dict_get(cur, &obj_bytes(head)) {
            Ok(g) => g,
            Err(e) => {
                unwind_fresh_chain(&chain);
                return Err(e);
            }
        };
        let (sub, sub_fresh) = match got {
            Some(s) if !obj::is_shared(s) => (s, false),
            Some(s) => (obj::duplicate(s), true),    // rc 0
            None => (dict::new_dict_obj(&[]), true), // rc 0
        };
        chain.push((cur, head, sub, sub_fresh));
        cur = sub;
    }
    let last = *keys
        .last()
        .expect("keys is non-empty: the [last] case above handles len 1");
    if let Err(e) = dict::dict_set(cur, last, value) {
        // A freshly duplicated/created `sub` (rc 0) at any level is only
        // retained once re-bound into its parent below; nothing in `chain` has
        // been re-bound yet, so unwind all of it.
        unwind_fresh_chain(&chain);
        return Err(e);
    }
    // Walk back up, re-binding each (possibly modified) sub-dict into its
    // parent. This is required even when a level was mutated in place: it
    // invalidates the parent's string rep so the nested change is visible up
    // the chain.
    let mut sub_result = cur;
    while let Some((parent, head, sub, fresh)) = chain.pop() {
        debug_assert!(core::ptr::eq(sub, sub_result));
        if let Err(e) = dict::dict_set(parent, head, sub_result) {
            // Effectively unreachable: `parent` was already proven a valid
            // dict by the `dict_get` above that produced `sub`, and nothing
            // between then and now could have changed that. Handled anyway,
            // strictly more leak-safe than leaving `sub_result` dangling.
            if fresh {
                drop_fresh(sub_result);
            }
            unwind_fresh_chain(&chain);
            return Err(e);
        }
        sub_result = parent;
    }
    Ok(())
}

/// `dict unset dictVarName key ?key ...?` — remove the value at a (possibly
/// nested) key path from the dict held by the variable.
fn unset(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictVarName key ?key ...?");
    }
    if interp
        .native_invocation_dialect()
        .native_dictionary_path_publication()
        == Some(
            tcl_registry::native_dictionary::NativeDictionaryPathPublication::JimOriginalVariable,
        )
    {
        return original_jim_dictionary_path(
            interp,
            argv,
            tcl_registry::native_dictionary::NativeDictionaryRootPurpose::VariableUnset,
        );
    }
    match native_variable_objects(interp, argv[2]) {
        Ok(Some((name, objects))) => {
            let keys: Vec<_> = argv[3..]
                .iter()
                .copied()
                .map(crate::value_ops::RuntimeAppendValue::borrowed)
                .collect();
            return interp.dictionary_variable_update(&name, &objects, |_, prepared| {
                tcl_cmd_core::native_dictionary::remove_prepared_path(&objects, prepared, &keys)
            });
        }
        Ok(None) => {}
        Err(code) => return code,
    }
    interp.report_cmd_error(
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original dictionary path worker",
        )
        .into(),
    )
}

// iteration

/// `dict for {keyVar valueVar} dictValue body` — iterate in insertion order,
/// evaluating `body` in the current scope with the loop vars set.
fn for_(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_runtime_api::completion_options::ControlOptionPolicy;

    if argv.len() != 5 {
        return interp.wrong_args_for_prefix(
            argv,
            2,
            b"{keyVarName valueVarName} dictionary script",
        );
    }
    let var_spec = obj_bytes(argv[2]);
    let vars = match parse::split_list(&var_spec) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message()),
    };
    if vars.len() != 2 {
        return interp.set_error(b"must have exactly two variable names");
    }
    let (kvar, vvar) = (vars[0].clone(), vars[1].clone());
    let compiled = interp.release_compiles_control(argv);
    let pairs = match dict::dict_pairs(argv[3]) {
        Ok(p) => p,
        Err(e) => return bad_dict(interp, e),
    };

    let policy = ControlOptionPolicy::FRESH_SETTLED;
    if let Err(first) = interp.begin_control_options_checked(policy) {
        return interp.refuse_native_execution(first);
    }
    for (k, v) in pairs {
        if let Err(first) = interp.begin_control_options_checked(policy) {
            return interp.refuse_native_execution(first);
        }
        if interp.var_set(&kvar, k).is_err() {
            return cant_set(interp, &kvar);
        }
        if interp.var_set(&vvar, v).is_err() {
            return cant_set(interp, &vvar);
        }
        match interp.eval_compiled_control_body(argv[4], compiled) {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            Code::Error => {
                if !compiled {
                    interp.append_body_frame(b"dict for");
                }
                return Code::Error;
            }
            other => return other, // Return propagates (result already set)
        }
    }
    interp.set_result_bytes(b"");
    if let Err(first) = interp.settle_control_options_checked(policy, Code::Ok) {
        return interp.refuse_native_execution(first);
    }
    Code::Ok
}

/// `dict map {keyVar valueVar} dictValue body` — like `dict for`, but each
/// iteration's body result becomes the new value for that key; returns the
/// transformed dict. `continue` drops the key, `break` stops.
fn map(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_runtime_api::completion_options::ControlOptionPolicy;

    if argv.len() != 5 {
        return interp.wrong_args_for_prefix(
            argv,
            2,
            b"{keyVarName valueVarName} dictionary script",
        );
    }
    let vars = match parse::split_list(&obj_bytes(argv[2])) {
        Ok(v) => v,
        Err(e) => return interp.set_error(e.message()),
    };
    if vars.len() != 2 {
        return interp.set_error(b"must have exactly two variable names");
    }
    let pairs = match dict::dict_pairs(argv[3]) {
        Ok(p) => p,
        Err(e) => return bad_dict(interp, e),
    };
    let compiled = interp.release_compiles_control(argv);
    let acc = dict::new_dict_obj(&[]);
    unsafe { obj::incr_ref_count(acc) };
    let policy = ControlOptionPolicy::FRESH_FORWARDED;
    if let Err(first) = interp.begin_control_options_checked(policy) {
        interp.refuse_native_execution(first.clone());
        unsafe { obj::decr_ref_count(acc) };
        let _ = interp.check_entered_native_operation();
        return interp.refuse_native_execution(first);
    }
    for (k, v) in pairs {
        if let Err(first) = interp.begin_control_options_checked(policy) {
            interp.refuse_native_execution(first.clone());
            unsafe { obj::decr_ref_count(acc) };
            let _ = interp.check_entered_native_operation();
            return interp.refuse_native_execution(first);
        }
        if interp.var_set(&vars[0], k).is_err() || interp.var_set(&vars[1], v).is_err() {
            unsafe { obj::decr_ref_count(acc) };
            return cant_set(interp, &vars[0]);
        }
        match interp.eval_compiled_control_body(argv[4], compiled) {
            Code::Ok => {
                let _ = dict::dict_set(acc, k, interp.get_obj_result());
            }
            Code::Continue => {}
            Code::Break => break,
            Code::Error => {
                unsafe { obj::decr_ref_count(acc) };
                if !compiled {
                    interp.append_body_frame(b"dict map");
                }
                return Code::Error;
            }
            other => {
                unsafe { obj::decr_ref_count(acc) };
                return other;
            }
        }
    }
    interp.set_result(acc);
    unsafe { obj::decr_ref_count(acc) };
    if let Err(first) = interp.settle_control_options_checked(policy, Code::Ok) {
        return interp.refuse_native_execution(first);
    }
    Code::Ok
}

fn scope_plan(
    interp: &Interp,
    argv: &[*mut TclObj],
    spec: tcl_registry::dictionary_scope::DictionaryScopeSpec,
) -> Option<tcl_registry::dictionary_scope::DictionaryScopePlan> {
    let words = vec![tcl_registry::InvocationWord::Dynamic; argv.len().checked_sub(2)?];
    let arguments = tcl_registry::InvocationArguments::structured(&words)
        .with_dialect(interp.native_invocation_dialect());
    let tcl_registry::dictionary_scope::DictionaryScopeSelection::Selected(plan) =
        spec.select(arguments, 0)
    else {
        return None;
    };
    Some(plan)
}

fn scope_route(
    interp: &Interp,
    code: Code,
) -> tcl_registry::completion_route::InvocationCompletionRoute {
    use tcl_registry::completion_route::{
        InvocationCompletionRoute as Route, ReturnCompletionRoute,
    };
    if interp.exit_pending() {
        return Route::ProcessExit;
    }
    if code == Code::Return {
        Route::Return(ReturnCompletionRoute {
            eventual_code: tcl_registry::CompletionCode::from_int(
                interp.pending_return_code().as_int() as i32,
            ),
            remaining_level: interp.pending_return_level() as u64,
        })
    } else {
        Route::Tcl(tcl_registry::CompletionCode::from_int(code.as_int() as i32))
    }
}

fn scope_writeback_failure(plan: &tcl_registry::dictionary_scope::DictionaryScopePlan) -> Code {
    match plan.writeback_failure_route() {
        tcl_registry::completion_route::InvocationCompletionRoute::Tcl(code) => {
            Code::from_int(code.as_int() as i32)
        }
        _ => unreachable!("authored writeback failure route is exact"),
    }
}

fn finish_scope_completion(
    interp: &mut Interp,
    plan: &tcl_registry::dictionary_scope::DictionaryScopePlan,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    original: Code,
) -> Code {
    use tcl_registry::completion_route::InvocationCompletionRoute as Route;
    match plan.completion_after_writeback(route) {
        Route::Tcl(code) => {
            if original == Code::Return {
                interp.set_return_state(0, Code::Ok);
            }
            Code::from_int(code.as_int() as i32)
        }
        Route::Return(returned) => {
            interp.set_return_state(
                returned.remaining_level as usize,
                Code::from_int(returned.eventual_code.as_int() as i32),
            );
            Code::Return
        }
        _ => original,
    }
}

/// `dict update dictVar key var ?key var ...? body` — link each key's value to
/// a local var, run body, then write the (possibly changed/unset) vars back
/// into the dict variable. The body's completion is the result.
fn update(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // [dict, update, dictVar, k, v, k, v, …, body]: at least one pair + body.
    if argv.len() < 6 || argv.len() % 2 != 0 {
        return interp.wrong_args_for_prefix(
            argv,
            2,
            b"dictVarName key varName ?key varName ...? script",
        );
    }
    let Some(plan) = scope_plan(
        interp,
        argv,
        tcl_registry::dictionary_scope::DictionaryScopeSpec::Update,
    ) else {
        return interp.error(b"native dictionary scope protocol is not selected");
    };
    if let Some(wrapper) = plan.scripted_wrapper() {
        let command = crate::interp::new_string(wrapper.command.as_bytes());
        let command = crate::obj::Owned::fresh(command);
        let mut words = Vec::with_capacity(argv.len() - 1);
        words.push(command.as_ptr());
        words.extend_from_slice(&argv[2..]);
        return interp.dispatch(&words);
    }
    let body_obj = argv[argv.len() - 1];
    let pairs_args = &argv[3..argv.len() - 1];

    let d = match interp
        .dictionary_scope_variable_read(argv[2], crate::interp::DictionaryScopeRead::Initial)
    {
        Ok(Some(dictionary)) => dictionary,
        Ok(None) => {
            let name = match interp.native_object_string_bytes(argv[2]) {
                Ok(name) => name,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            return no_such_var(interp, &name);
        }
        Err(code) => return code,
    };
    let original_dictionary = crate::obj::Owned::retain(d);
    // Link phase: set each local to its key's value (or unset if absent).
    for c in pairs_args.chunks_exact(2) {
        let key = obj_bytes(c[0]);
        match dict::dict_get(original_dictionary.as_ptr(), &key) {
            Ok(Some(val)) => {
                if let Err(code) = interp.assign_original_named_variable(c[1], val) {
                    return code;
                }
            }
            Ok(None) => {
                if plan.missing_key == tcl_registry::dictionary_scope::DictionaryMissingKey::Unset {
                    let name = match interp.native_object_string_bytes(c[1]) {
                        Ok(name) => name,
                        Err(error) => return interp.report_cmd_error(error.into()),
                    };
                    if interp.store_escapes(&name) {
                        return interp.confined_unset_error(&name);
                    }
                    interp.var_unset(&name);
                }
            }
            Err(e) => return bad_dict(interp, e),
        }
    }

    let code = interp.eval_control_body(body_obj);
    let route = scope_route(interp, code);
    if !plan.writeback_routes(route).captured {
        return code;
    }
    finish_update_writeback(interp, argv, &plan, route, code)
}

fn finish_update_writeback(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    plan: &tcl_registry::dictionary_scope::DictionaryScopePlan,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    code: Code,
) -> Code {
    let pairs_args = &argv[3..argv.len() - 1];
    let result = crate::obj::Owned::retain(interp.get_obj_result());

    // Write-back: re-read the dict (the body may have replaced it), then apply
    // each local var (set if it exists, drop the key if it was unset).
    let current = match interp.dictionary_scope_variable_read(
        argv[2],
        crate::interp::DictionaryScopeRead::UpdateWriteback,
    ) {
        Ok(current) => current,
        Err(code) => return code,
    };
    if let Some(cur) = current {
        let saved_error = interp.save_dictionary_scope_error();
        let Some(acc) = copy_dict(interp, cur) else {
            return scope_writeback_failure(plan);
        };
        for c in pairs_args.chunks_exact(2) {
            let value = match interp.dictionary_scope_variable_read(
                c[1],
                crate::interp::DictionaryScopeRead::UpdateWriteback,
            ) {
                Ok(value) => value,
                Err(code) => {
                    unsafe { obj::decr_ref_count(acc) };
                    return code;
                }
            };
            match value {
                Some(val) => {
                    let _ = dict::dict_set(acc, c[0], val);
                }
                None => {
                    let _ = dict::dict_unset(acc, &obj_bytes(c[0]));
                }
            }
        }
        if let Err(code) = interp.assign_original_named_variable(argv[2], acc) {
            unsafe { obj::decr_ref_count(acc) };
            return code;
        }
        unsafe { obj::decr_ref_count(acc) };
        if let Err(first) = interp.restore_dictionary_scope_error(saved_error) {
            return interp.refuse_native_execution(first);
        }
    }
    interp.set_result(result.as_ptr());
    finish_scope_completion(interp, plan, route, code)
}

/// `dict with dictVarName ?key ...? script` — map every key of the (sub-)dict to a
/// local var, run body, write the vars back. Supports a leading key path.
fn with(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return interp.wrong_args_for_prefix(argv, 2, b"dictVarName ?key ...? script");
    }
    let Some(plan) = scope_plan(
        interp,
        argv,
        tcl_registry::dictionary_scope::DictionaryScopeSpec::With,
    ) else {
        return interp.error(b"native dictionary scope protocol is not selected");
    };
    let body_obj = argv[argv.len() - 1];
    let path = &argv[3..argv.len() - 1];

    let d = match interp
        .dictionary_scope_variable_read(argv[2], crate::interp::DictionaryScopeRead::Initial)
    {
        Ok(Some(dictionary)) => dictionary,
        Ok(None) => {
            let name = match interp.native_object_string_bytes(argv[2]) {
                Ok(name) => name,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            return no_such_var(interp, &name);
        }
        Err(code) => return code,
    };
    let original_dictionary = crate::obj::Owned::retain(d);
    // Navigate the optional key path to the sub-dict.
    let mut sub = original_dictionary.as_ptr();
    for &k in path {
        match dict::dict_get(sub, &obj_bytes(k)) {
            Ok(Some(v)) => sub = v,
            Ok(None) => return key_not_known(interp, &obj_bytes(k)),
            Err(e) => return bad_dict(interp, e),
        }
    }
    let pairs = match dict::dict_pairs(sub) {
        Ok(p) => p,
        Err(e) => return bad_dict(interp, e),
    };
    // Map every key to a local var.
    let keys: Vec<*mut TclObj> = pairs.iter().map(|&(key, _)| key).collect();
    for (key, value) in &pairs {
        if let Err(code) = interp.assign_original_named_variable(*key, *value) {
            return code;
        }
    }

    let code = interp.eval_control_body(body_obj);
    let route = scope_route(interp, code);
    if !plan.writeback_routes(route).captured {
        return code;
    }
    finish_with_writeback(interp, argv, &keys, &plan, route, code)
}

fn finish_with_writeback(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    keys: &[*mut TclObj],
    plan: &tcl_registry::dictionary_scope::DictionaryScopePlan,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    code: Code,
) -> Code {
    let path = &argv[3..argv.len() - 1];
    let result = crate::obj::Owned::retain(interp.get_obj_result());

    let saved_error = interp.save_dictionary_scope_error();

    // Write-back: rebuild the (sub-)dict at the key path from the mapped locals,
    // then store it back through the path. The body may have replaced the dict,
    // so re-read it; if the path no longer resolves, skip the write-back.
    let current = match interp
        .dictionary_scope_variable_read(argv[2], crate::interp::DictionaryScopeRead::WithWriteback)
    {
        Ok(current) => current,
        Err(code) => return code,
    };
    if let Some(cur) = current {
        // A malformed current/sub dict is a write-back error, not a silent
        // skip (the `copy_dict` call has already set the result).
        let Some(acc) = copy_dict(interp, cur) else {
            return scope_writeback_failure(plan);
        };
        // Navigate `acc` to the sub-dict at `path`.
        let mut sub_src = acc;
        let mut reached = true;
        for &k in path {
            match dict::dict_get(sub_src, &obj_bytes(k)) {
                Ok(Some(v)) => sub_src = v,
                _ => {
                    reached = false;
                    break;
                }
            }
        }
        if reached {
            let Some(newsub) = copy_dict(interp, sub_src) else {
                unsafe { obj::decr_ref_count(acc) };
                return scope_writeback_failure(plan);
            };
            for &key in keys {
                let value = match interp.dictionary_scope_variable_read(
                    key,
                    crate::interp::DictionaryScopeRead::UpdateWriteback,
                ) {
                    Ok(value) => value,
                    Err(code) => {
                        unsafe {
                            obj::decr_ref_count(newsub);
                            obj::decr_ref_count(acc);
                        }
                        return code;
                    }
                };
                match value {
                    Some(value) => {
                        let _ = dict::dict_set(newsub, key, value);
                    }
                    None => {
                        let _ = dict::dict_unset(newsub, &obj_bytes(key));
                    }
                }
            }
            // The rebuilt sub is the whole dict (no path) or is set back
            // at the path within `acc`.
            let store = if path.is_empty() {
                newsub
            } else {
                let _ = dict_path_set(acc, path, newsub);
                acc
            };
            if let Err(code) = interp.assign_original_named_variable(argv[2], store) {
                unsafe {
                    obj::decr_ref_count(newsub);
                    obj::decr_ref_count(acc);
                }
                return if interp.host_refusal_pending() {
                    code
                } else {
                    scope_writeback_failure(plan)
                };
            }
            unsafe { obj::decr_ref_count(newsub) };
        }
        unsafe { obj::decr_ref_count(acc) };
    }
    if let Err(first) = interp.restore_dictionary_scope_error(saved_error) {
        return interp.refuse_native_execution(first);
    }
    interp.set_result(result.as_ptr());
    finish_scope_completion(interp, plan, route, code)
}

// helpers

/// Present a typed dictionary conversion failure with its exact byte message
/// and dictionary-specific native error code.
fn bad_dict(interp: &mut Interp, e: crate::dict::DictError) -> Code {
    interp.error_with_code(&e.message_bytes(), &e.error_code())
}

/// `key "…" not known in dictionary` with `TCL LOOKUP DICT <key>`.
fn key_not_known(interp: &mut Interp, key: &[u8]) -> Code {
    let mut m = b"key \"".to_vec();
    m.extend_from_slice(key);
    m.extend_from_slice(b"\" not known in dictionary");
    let code = crate::interp::error_code_list(&[b"TCL", b"LOOKUP", b"DICT", key]);
    interp.error_with_code(&m, &code)
}

fn cant_set(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"can't set \"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": variable is array");
    interp.set_error(&m)
}
fn no_such_var(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"can't read \"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": no such variable");
    interp.set_error(&m)
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
mod native_scope_trace_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_path_publication_tests;

#[cfg(test)]
#[path = "cmd_dict/original_path_tests.rs"]
mod original_path_tests;

#[cfg(test)]
mod original_map_option_cleanup_tests;
