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

//! `append` + the `string` ensemble (T1.6), per the EXP-STRING decision:
//! capacity-backed in-place `append` (amortised O(1)), and char-indexed `string`
//! ops with an **ASCII fast path** (byte index == char index) falling back to a
//! UTF-8 scan for non-ASCII.
//!
//! Subset now: `string length/index/range/equal/compare/cat/repeat/reverse/`
//! `toupper/tolower/trim/trimleft/trimright/first/last`. (`map`/`match`/`is`/
//! `replace`/`insert`/`wordstart` follow; Unicode case + a non-ASCII char-offset
//! cache are deferred per EXP-STRING.)
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use tcl_cmd_core::prefix::Resolution;

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};

/// Register `append` + the `string` ensemble.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"append", append);
    // `::tcl::string::insert`/`::tcl::string::reverse` are the real commands the
    // `string insert`/`string reverse` ensemble entries map to; some tests (and
    // the byte-compiler) invoke them directly.
    interp.register_builtin(b"::tcl::string::insert", tcl_string_insert);
    interp.register_builtin(b"::tcl::string::reverse", tcl_string_reverse);
    // `tcl::prefix` — prefix matching against a table (`tclIndexObj.c`).
    interp.register_builtin(b"::tcl::prefix", tcl_prefix);
    let registry = tcl_registry::CommandRegistry::build_default();
    interp.register_spec_builtin(
        registry.get("string").expect("core string spec"),
        string_cmd,
    );
}

// -- append ----------------------------------------------------------------

/// `append varName ?value ...?` — append to the string in `varName` (creating
/// it if unset), growing the buffer in place (amortised O(1)) when the value is
/// an unshared plain string, else copy-on-write. Returns the new value.
pub(crate) fn append(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"append varName ?value ...?");
    }
    let name = obj_bytes(argv[1]);
    let values = &argv[2..];
    // Split an `arr(idx)` reference up front and drive the element/scalar store
    // helpers, exactly like `set`/`lappend`. `var_set` does *not* itself parse
    // `(...)`, so passing the raw `x(0)` created a scalar literally named `x(0)`
    // (and left the real variable in a corrupt half-state) instead of erroring
    // `variable isn't array` — an `append x(0)` on a scalar `x`.
    let (base, elem) = crate::frame::split_array_ref(&name);
    let read_cur = |interp: &mut Interp| match &elem {
        Some(k) => interp.var_get_elem(&base, k),
        None => interp.var_get(&base),
    };

    if values.is_empty() {
        // `append x` with no values is a pure read and fires a read trace (the
        // *with-values* form does not, unlike `lappend` — append-7.2/7.3).
        if let Some(c) = interp.fire_read_trace(&base, elem.as_deref()) {
            return c;
        }
        return match read_cur(interp) {
            Some(o) => {
                interp.set_result(o);
                Code::Ok
            }
            None => {
                let msg = interp.read_miss_msg(&base, elem.as_deref());
                interp.set_error(&msg)
            }
        };
    }

    // A constant can't be appended to; reject before the update would bypass the
    // store-time constant check (var-26.2/27.2).
    if let Some(c) = interp.const_write_check(&name) {
        return c;
    }

    // Byte-exact concatenation, shared with the VM via `append_bytes`: it grows
    // the current value in place when it's an unshared plain string (returning
    // that same object) else builds a fresh copy/new value.
    let cur = read_cur(interp);
    let result = tcl_cmd_core::var::append_bytes(interp, cur, values);

    // Always store back: rebinds the variable to `result` — a refcount-neutral
    // re-set when it was grown in place — and fires the write trace exactly once
    // (the in-place path used to skip the store and so fire no trace, diverging
    // from C; this fixes that). `store_var_result` holds a protective reference
    // across the store so a write trace that unsets the variable can't free a
    // fresh `result` before it becomes the result (a use-after-free).
    match interp.store_var_result(&base, elem.as_deref(), result) {
        Ok(()) => Code::Ok,
        Err(e) => crate::builtins::var_error(interp, &name, e),
    }
}

// -- string ensemble -------------------------------------------------------

fn string_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"string subcommand ?arg ...?");
    }
    // The `string` ensemble resolves its subcommand by unambiguous prefix
    // (`Tcl_GetIndexFromObj`): `string fir` → `first`, `string trim` wins over
    // its `trimleft`/`trimright` prefixes via the exact-match rule.
    let sub = obj_bytes(argv[1]);
    // `insert` arrives in Tcl 9, so under an earlier pin it must not resolve
    // and must not make `string in` ambiguous with `index`.
    let subs = crate::environment::release_subcommands(
        interp.runtime_version().dialect_profile_name(),
        "string",
        STRING_SUBCOMMANDS,
    );
    let canonical: &[u8] = match tcl_cmd_core::ensemble::resolve_subcommand(&subs, &sub, true) {
        Some(index) => subs[index],
        // The whole sentence — including the ensemble's comma before `or` —
        // belongs to the owner, not to a literal beside the table.
        None => {
            return interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                &subs,
                &sub,
                true,
                b"::tcl::string",
            ));
        }
    };
    // Portable subcommands now live in the shared command core (`tcl-cmd-core`),
    // driven over this runtime's `*mut TclObj` `ValueOps`. The runtime is a thin
    // adapter: map `Result<*mut TclObj, CmdError>` onto set_result/set_error.
    // Not-yet-ported subcommands fall through to the legacy arms below.
    if let Ok(canon_str) = std::str::from_utf8(canonical) {
        if let Some(result) = tcl_cmd_core::string::dispatch_canon(interp, canon_str, &argv[2..]) {
            return match result {
                Ok(v) => {
                    interp.set_result(v);
                    Code::Ok
                }
                Err(e) => interp.set_error(e.message().as_bytes()),
            };
        }
    }
    match canonical {
        b"length" => str_length(interp, argv),
        b"index" => str_index(interp, argv),
        b"range" => str_range(interp, argv),
        b"cat" => str_cat(interp, argv),
        b"repeat" => str_repeat(interp, argv),
        b"reverse" => str_reverse(interp, argv),
        b"toupper" => str_case(interp, argv, CaseMode::Upper),
        b"tolower" => str_case(interp, argv, CaseMode::Lower),
        b"totitle" => str_case(interp, argv, CaseMode::Title),
        b"trim" => str_trim(interp, argv, true, true),
        b"trimleft" => str_trim(interp, argv, true, false),
        b"trimright" => str_trim(interp, argv, false, true),
        b"first" => str_first_last(interp, argv, true),
        b"last" => str_first_last(interp, argv, false),
        b"match" => str_match(interp, argv),
        b"map" => str_map(interp, argv),
        b"is" => str_is(interp, argv),
        b"replace" => str_replace(interp, argv),
        b"insert" => str_insert(interp, argv),
        // `wordstart`/`wordend` are handled by the shared core above.
        _ => unreachable!("index_lookup only yields a known subcommand"),
    }
}

/// The `string` ensemble subcommands, in the order used by the
/// "unknown or ambiguous subcommand" diagnostic.
const STRING_SUBCOMMANDS: &[&[u8]] = &[
    b"cat",
    b"compare",
    b"equal",
    b"first",
    b"index",
    b"insert",
    b"is",
    b"last",
    b"length",
    b"map",
    b"match",
    b"range",
    b"repeat",
    b"replace",
    b"reverse",
    b"tolower",
    b"totitle",
    b"toupper",
    b"trim",
    b"trimleft",
    b"trimright",
    b"wordend",
    b"wordstart",
];

fn str_length(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"string length string");
    }
    let n = char_count(&obj_bytes(argv[2]));
    interp.set_result(obj::new_wide_int_obj(n as i64));
    Code::Ok
}

fn str_index(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"string index string charIndex");
    }
    let s = obj_bytes(argv[2]);
    let n = char_count(&s);
    let idx = match index_spec(&obj_bytes(argv[3]), n) {
        Some(i) => i,
        None => return bad_index(interp, &obj_bytes(argv[3])),
    };
    if idx < 0 || idx as usize >= n {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    let b0 = char_to_byte(&s, idx as usize);
    let b1 = char_to_byte(&s, idx as usize + 1);
    interp.set_result_bytes(&s[b0..b1]);
    Code::Ok
}

fn str_range(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return interp.wrong_args(b"string range string first last");
    }
    let s = obj_bytes(argv[2]);
    let n = char_count(&s);
    let first = match index_spec(&obj_bytes(argv[3]), n) {
        Some(i) => i.max(0) as usize,
        None => return bad_index(interp, &obj_bytes(argv[3])),
    };
    let last = match index_spec(&obj_bytes(argv[4]), n) {
        Some(i) => i,
        None => return bad_index(interp, &obj_bytes(argv[4])),
    };
    if last < 0 || first >= n || (last as usize) < first {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    let last = (last as usize).min(n - 1);
    let b0 = char_to_byte(&s, first);
    let b1 = char_to_byte(&s, last + 1);
    interp.set_result_bytes(&s[b0..b1]);
    Code::Ok
}

/// `string replace string first last ?newstring?` — replace the character range
/// `[first,last]` with `newstring` (or delete it). Out-of-range / `first>last`
/// returns the string unchanged (`StringRplcCmd`).
fn str_replace(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 && argv.len() != 6 {
        return interp.wrong_args(b"string replace string first last ?string?");
    }
    let chars: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[2]))
        .chars()
        .collect();
    let n = chars.len();
    let first = match index_spec(&obj_bytes(argv[3]), n) {
        Some(i) => i,
        None => return bad_index(interp, &obj_bytes(argv[3])),
    };
    let last = match index_spec(&obj_bytes(argv[4]), n) {
        Some(i) => i,
        None => return bad_index(interp, &obj_bytes(argv[4])),
    };
    let lo = first.max(0) as usize;
    let hi = last.min(n as isize - 1);
    if first > last || first >= n as isize || last < 0 || lo > n {
        // No replacement — return the original string unchanged.
        interp.set_result(argv[2]);
        return Code::Ok;
    }
    let hi = hi as usize; // inclusive end, guaranteed >= lo here
    let mut out: String = chars[..lo].iter().collect();
    if argv.len() == 6 {
        out.push_str(&String::from_utf8_lossy(&obj_bytes(argv[5])));
    }
    out.extend(&chars[hi + 1..]);
    interp.set_result_bytes(out.as_bytes());
    Code::Ok
}

/// `string insert string index insertString` — insert before character `index`
/// (`index == end`/`>= length` appends). `StringInsertCmd`.
fn str_insert(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 {
        return interp.wrong_args(b"string insert string index insertString");
    }
    let chars: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[2]))
        .chars()
        .collect();
    let n = chars.len();
    // `string insert` uses `end == length` (append), unlike most index commands.
    let idx = match index_spec(&obj_bytes(argv[3]), n + 1) {
        Some(i) => i.max(0).min(n as isize) as usize,
        None => return bad_index(interp, &obj_bytes(argv[3])),
    };
    let mut out: String = chars[..idx].iter().collect();
    out.push_str(&String::from_utf8_lossy(&obj_bytes(argv[4])));
    out.extend(&chars[idx..]);
    interp.set_result_bytes(out.as_bytes());
    Code::Ok
}

/// `::tcl::string::insert string index insertString` — the command the `string
/// insert` ensemble maps to (called directly by some tests). Reuses `str_insert`
/// by re-aligning argv to the `string insert …` shape.
fn tcl_string_insert(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"tcl::string::insert string index insertString");
    }
    let shifted = [argv[0], argv[0], argv[1], argv[2], argv[3]];
    str_insert(interp, &shifted)
}

/// `::tcl::string::reverse string` — the command behind the `string reverse`
/// ensemble entry. Re-aligns argv to the `string reverse …` shape.
fn tcl_string_reverse(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 2 {
        return interp.wrong_args(b"tcl::string::reverse string");
    }
    let shifted = [argv[0], argv[0], argv[1]];
    str_reverse(interp, &shifted)
}

/// `tcl::prefix match|all|longest …` — prefix matching against a table
/// (`Tcl_PrefixMatchObjCmd` / `Tcl_PrefixAllObjCmd` / `Tcl_PrefixLongestObjCmd`).
fn tcl_prefix(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return interp.wrong_args(b"tcl::prefix subcommand ?arg ...?");
    }
    let word = obj_bytes(argv[1]);
    let canon = match tcl_cmd_core::ensemble::resolve_subcommand(PREFIX_SUBS, &word, true) {
        Some(index) => PREFIX_SUBS[index],
        None => {
            return interp.set_error(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                PREFIX_SUBS,
                &word,
                true,
                b"::tcl::prefix",
            ));
        }
    };
    match canon {
        b"all" => tcl_prefix_all(interp, argv),
        b"longest" => tcl_prefix_longest(interp, argv),
        _ => tcl_prefix_match(interp, argv),
    }
}

/// `tcl::prefix`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
const PREFIX_SUBS: &[&[u8]] = &[b"all", b"longest", b"match"];

/// Split `s` as a Tcl list, or set the full list-parse error and return the
/// failing `Code` (used by the `tcl::prefix` subcommands).
///
/// The message comes from [`crate::parse::list_error_message`], which builds it
/// out of the shared codec. This function used to reach a local re-scan of the
/// list — a third implementation of `TclFindElement` alongside the owner and
/// `parse.rs`'s copy — purely to recover the junk fragment (issue #1429).
fn split_list_or_error(interp: &mut Interp, s: &[u8]) -> Result<Vec<Vec<u8>>, Code> {
    match crate::parse::split_list(s) {
        Ok(t) => Ok(t),
        Err(e) => {
            let msg = crate::parse::list_error_message(s, e);
            Err(interp.set_error(&msg))
        }
    }
}

/// `tcl::prefix match`'s own option words, in C table order (`matchOptions[]`,
/// `tclIndexObj.c`): `Tcl_GetIndexFromObj(…, "option", 0)`, so `-m`
/// abbreviates `-message` while `-e` prefixes both `-error` and `-exact` and
/// is `ambiguous option "-e"`.
const PREFIX_MATCH_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &[b"-error", b"-exact", b"-message"]);

fn tcl_prefix_match(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // tcl::prefix match ?options? table string
    if argv.len() < 4 {
        return interp.wrong_args(b"tcl::prefix match ?options? table string");
    }
    let mut exact = false;
    let mut message: Vec<u8> = b"option".to_vec();
    // Some(opts) ⇒ -error given. Options precede the trailing `table string`;
    // `-error`/`-message` need a value that must not intrude into those last two
    // arguments.
    let mut error_opts: Option<Vec<u8>> = None;
    let opt_end = argv.len() - 2;
    let mut i = 2;
    while i < opt_end {
        let opt = obj_bytes(argv[i]);
        match PREFIX_MATCH_OPTIONS.index_of(&opt) {
            Ok(1) => {
                exact = true;
                i += 1;
            }
            Ok(2) => {
                if i + 1 >= opt_end {
                    return interp.set_error(b"missing value for -message");
                }
                message = obj_bytes(argv[i + 1]);
                i += 2;
            }
            Ok(_) => {
                if i + 1 >= opt_end {
                    return interp.set_error(b"missing value for -error");
                }
                let val = obj_bytes(argv[i + 1]);
                let elems = match split_list_or_error(interp, &val) {
                    Ok(e) => e,
                    Err(c) => return c,
                };
                if elems.len() % 2 != 0 {
                    return interp.set_error(b"error options must have an even number of elements");
                }
                error_opts = Some(val);
                i += 2;
            }
            Err(m) => return interp.set_error(&m),
        }
    }
    let table = match split_list_or_error(interp, &obj_bytes(argv[argv.len() - 2])) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = obj_bytes(argv[argv.len() - 1]);
    // The shared `Tcl_GetIndexFromObjStruct` matcher: an exact entry always
    // wins; otherwise a unique prefix (unless `-exact`); an empty string never
    // matches (the old local scan wrongly resolved `""` against a one-entry
    // table). `prefix::scan` (not an `OptionTable`) because the `-message`
    // noun here is caller bytes and `-error {}` returns between the scan and
    // the message build.
    let miss = match tcl_cmd_core::prefix::scan(&table, &s, exact) {
        Resolution::Exact(i) | Resolution::UniquePrefix(i) => {
            interp.set_result(new_string(&table[i]));
            return Code::Ok;
        }
        miss => miss,
    };
    // No unique match: error (or return "" if -error {} was given).
    if let Some(opts) = &error_opts {
        if opts.is_empty() {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
    }
    // The shared message builder — including C's literal `no valid options`
    // for an empty table (the old local builder pluralised the `-message`
    // noun instead; tclsh8.6: `tcl::prefix match -message thing {} foo` →
    // `bad thing "foo": no valid options`).
    let m = tcl_cmd_core::prefix::bad_key_message(
        &table,
        &message,
        &s,
        matches!(miss, Resolution::Ambiguous),
    );
    interp.set_error(&m)
}

fn tcl_prefix_all(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"tcl::prefix all table string");
    }
    let table = match split_list_or_error(interp, &obj_bytes(argv[2])) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = obj_bytes(argv[3]);
    let objs: Vec<*mut TclObj> = table
        .iter()
        .filter(|e| e.starts_with(&s))
        .map(|e| new_string(e))
        .collect();
    interp.set_result(crate::list::new_list_obj(&objs));
    for &o in &objs {
        drop_fresh(o);
    }
    Code::Ok
}

fn tcl_prefix_longest(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"tcl::prefix longest table string");
    }
    let table = match split_list_or_error(interp, &obj_bytes(argv[2])) {
        Ok(t) => t,
        Err(c) => return c,
    };
    let s = obj_bytes(argv[3]);
    let hits: Vec<&Vec<u8>> = table.iter().filter(|e| e.starts_with(&s)).collect();
    // Longest common prefix of all matching elements (at least `s`), compared by
    // character so a shared multi-byte lead byte never yields a partial char.
    let mut longest: Vec<char> = match hits.first() {
        Some(f) => String::from_utf8_lossy(f).chars().collect(),
        None => {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
    };
    for e in &hits[1..] {
        let ec: Vec<char> = String::from_utf8_lossy(e).chars().collect();
        let common = longest
            .iter()
            .zip(ec.iter())
            .take_while(|(a, b)| a == b)
            .count();
        longest.truncate(common);
    }
    let out: String = longest.iter().collect();
    interp.set_result_bytes(out.as_bytes());
    Code::Ok
}

fn str_cat(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut out = Vec::new();
    for &a in &argv[2..] {
        out.extend_from_slice(&obj_bytes(a));
    }
    interp.set_result_bytes(&out);
    Code::Ok
}

fn str_repeat(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return interp.wrong_args(b"string repeat string count");
    }
    let s = obj_bytes(argv[2]);
    let count = match parse_isize(&obj_bytes(argv[3])) {
        Some(c) => c,
        None => return not_integer(interp, &obj_bytes(argv[3])),
    };
    if count <= 0 {
        interp.set_result_bytes(b"");
        return Code::Ok;
    }
    let mut out = Vec::with_capacity(s.len() * count as usize);
    for _ in 0..count {
        out.extend_from_slice(&s);
    }
    interp.set_result_bytes(&out);
    Code::Ok
}

fn str_reverse(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return interp.wrong_args(b"string reverse string");
    }
    let s = obj_bytes(argv[2]);
    let out = if s.is_ascii() {
        let mut v = s.clone();
        v.reverse();
        v
    } else {
        // reverse by character (collect UTF-8 chars, reverse)
        let mut chars: Vec<&[u8]> = Vec::new();
        let mut i = 0;
        while i < s.len() {
            let l = utf8_len(s[i]);
            chars.push(&s[i..(i + l).min(s.len())]);
            i += l;
        }
        let mut v = Vec::with_capacity(s.len());
        for c in chars.into_iter().rev() {
            v.extend_from_slice(c);
        }
        v
    };
    interp.set_result_bytes(&out);
    Code::Ok
}

/// `string toupper`/`tolower`/`totitle`: the case mapping to apply.
#[derive(Clone, Copy)]
enum CaseMode {
    Upper,
    Lower,
    /// First char of the range → upper, the rest → lower.
    Title,
}

/// `string toupper|tolower|totitle string ?first? ?last?` — case-map a character
/// range (the whole string when no range is given). With a single index, only
/// that one character is mapped (C's `last = first`); `totitle` title-cases the
/// first character of the range and lower-cases the rest. Case mapping is simple
/// (1:1) Unicode, matching `Tcl_UniCharTo{Upper,Lower,Title}`: a character whose
/// Unicode mapping is not a single code point (e.g. `ß`→`SS`) is left unchanged.
fn str_case(interp: &mut Interp, argv: &[*mut TclObj], mode: CaseMode) -> Code {
    let usage: &[u8] = match mode {
        CaseMode::Upper => b"string toupper string ?first? ?last?",
        CaseMode::Lower => b"string tolower string ?first? ?last?",
        CaseMode::Title => b"string totitle string ?first? ?last?",
    };
    if argv.len() < 3 || argv.len() > 5 {
        return interp.wrong_args(usage);
    }
    let s = obj_bytes(argv[2]);

    // No range: map the entire string.
    if argv.len() == 3 {
        let mapped: String = map_case(&String::from_utf8_lossy(&s), mode);
        interp.set_result_bytes(mapped.as_bytes());
        return Code::Ok;
    }

    let chars: Vec<char> = String::from_utf8_lossy(&s).chars().collect();
    let n = chars.len();
    // The index `end`/`end±N` resolves against `n-1` (`TclGetIntForIndexM`).
    let first = match index_spec(&obj_bytes(argv[3]), n) {
        Some(i) => i.max(0) as usize,
        None => return bad_index(interp, &obj_bytes(argv[3])),
    };
    // A lone index maps just that character (`last = first`).
    let last = match argv.get(4) {
        Some(&a) => match index_spec(&obj_bytes(a), n) {
            Some(i) => i,
            None => return bad_index(interp, &obj_bytes(a)),
        },
        None => first as isize,
    };
    let last = if last >= n as isize {
        n as isize - 1
    } else {
        last
    };
    if last < first as isize {
        interp.set_result_bytes(&s); // empty range ⇒ unchanged
        return Code::Ok;
    }
    let last = last as usize;

    let mut out = String::with_capacity(s.len());
    out.extend(&chars[..first]);
    for (k, &c) in chars[first..=last].iter().enumerate() {
        let cm = match mode {
            CaseMode::Upper => simple_upper(c),
            CaseMode::Lower => simple_lower(c),
            // `totitle`: first char of the range title-cased, the rest lowered.
            CaseMode::Title if k == 0 => simple_title(c),
            CaseMode::Title => simple_lower(c),
        };
        out.push(cm);
    }
    out.extend(&chars[last + 1..]);
    interp.set_result_bytes(out.as_bytes());
    Code::Ok
}

/// Map an entire string with the given case mode (`totitle` title-cases the very
/// first character and lower-cases the rest, like `Tcl_UtfToTitle`).
fn map_case(s: &str, mode: CaseMode) -> String {
    match mode {
        CaseMode::Upper => s.chars().map(simple_upper).collect(),
        CaseMode::Lower => s.chars().map(simple_lower).collect(),
        CaseMode::Title => s
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 0 {
                    simple_title(c)
                } else {
                    simple_lower(c)
                }
            })
            .collect(),
    }
}

/// Simple (1:1) Unicode case mapping helpers, matching `Tcl_UniCharTo*`: when the
/// full Unicode mapping is not a single code point, the character is unchanged.
fn simple_upper(c: char) -> char {
    let mut it = c.to_uppercase();
    match (it.next(), it.next()) {
        (Some(u), None) => u,
        _ => c,
    }
}
fn simple_lower(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}
/// Title-case equals upper-case except for the four Latin digraphs, whose
/// title form is the mixed-case middle code point (`Tcl_UniCharToTitle`).
/// (`char::to_titlecase` is still unstable, so these are hard-coded.)
fn simple_title(c: char) -> char {
    match c {
        '\u{01C4}' | '\u{01C5}' | '\u{01C6}' => '\u{01C5}',
        '\u{01C7}' | '\u{01C8}' | '\u{01C9}' => '\u{01C8}',
        '\u{01CA}' | '\u{01CB}' | '\u{01CC}' => '\u{01CB}',
        '\u{01F1}' | '\u{01F2}' | '\u{01F3}' => '\u{01F2}',
        _ => simple_upper(c),
    }
}

/// Tcl's default trim set (`tclDefaultTrimSet`): ASCII whitespace plus the full
/// Unicode whitespace/zero-width set, including NUL.
const DEFAULT_TRIM: &[char] = &[
    '\u{09}', '\u{0A}', '\u{0B}', '\u{0C}', '\u{0D}', ' ', '\u{0000}', '\u{0085}', '\u{00A0}',
    '\u{1680}', '\u{180E}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{200B}', '\u{2028}', '\u{2029}',
    '\u{202F}', '\u{205F}', '\u{2060}', '\u{3000}', '\u{FEFF}',
];

/// `string trim|trimleft|trimright string ?chars?` — strip leading/trailing
/// characters that appear in `chars` (Tcl's default whitespace set otherwise).
/// Matching is character-based so multi-byte trim characters work.
fn str_trim(interp: &mut Interp, argv: &[*mut TclObj], left: bool, right: bool) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        let usage: &[u8] = match (left, right) {
            (true, true) => b"string trim string ?chars?",
            (true, false) => b"string trimleft string ?chars?",
            _ => b"string trimright string ?chars?",
        };
        return interp.wrong_args(usage);
    }
    let s = obj_bytes(argv[2]);
    let chars: Vec<char> = String::from_utf8_lossy(&s).chars().collect();
    let set: Vec<char> = if argv.len() == 4 {
        String::from_utf8_lossy(&obj_bytes(argv[3]))
            .chars()
            .collect()
    } else {
        DEFAULT_TRIM.to_vec()
    };
    let mut lo = 0;
    let mut hi = chars.len();
    if left {
        while lo < hi && set.contains(&chars[lo]) {
            lo += 1;
        }
    }
    if right {
        while hi > lo && set.contains(&chars[hi - 1]) {
            hi -= 1;
        }
    }
    let out: String = chars[lo..hi].iter().collect();
    interp.set_result_bytes(out.as_bytes());
    Code::Ok
}

fn str_first_last(interp: &mut Interp, argv: &[*mut TclObj], first: bool) -> Code {
    if argv.len() < 4 || argv.len() > 5 {
        return interp.wrong_args(if first {
            b"string first needleString haystackString ?startIndex?"
        } else {
            b"string last needleString haystackString ?lastIndex?"
        });
    }
    let needle = obj_bytes(argv[2]);
    let hay = obj_bytes(argv[3]);
    let n = char_count(&hay);

    // Optional bound index (char-based, `end`/`end±N` aware).
    let bound = if argv.len() == 5 {
        let spec = obj_bytes(argv[4]);
        match index_spec(&spec, n) {
            Some(i) => Some(i),
            None => return bad_index(interp, &spec),
        }
    } else {
        None
    };

    // Empty needles never match ("We don't find empty substrings" — C's
    // TclStringFirst/TclStringLast both bail out for a zero-length needle).
    if char_count(&needle) == 0 {
        interp.set_result(obj::new_wide_int_obj(-1));
        return Code::Ok;
    }
    let nlen = char_count(&needle);

    // Byte search restricted by the bound, then convert to a char index.
    let byte_pos = if first {
        // `startIndex`: search at or after it (clamp negatives to 0).
        let start_char = bound.map_or(0, |i| i.max(0) as usize).min(n);
        let start_byte = char_to_byte(&hay, start_char);
        find_sub(&hay[start_byte..], &needle).map(|bp| bp + start_byte)
    } else {
        // `lastIndex` (default `end`) caps the index of the match's *final*
        // character: the latest start is `last + 1 - nlen`, so scan the prefix
        // ending just past `last` for the rightmost needle.
        let mut last: isize = bound.unwrap_or(n as isize - 1);
        if last >= n as isize {
            last = n as isize - 1;
        }
        if last + 1 < nlen as isize {
            None
        } else {
            let slice_end = char_to_byte(&hay, (last + 1) as usize);
            rfind_sub(&hay[..slice_end], &needle)
        }
    };
    let result = byte_pos.map_or(-1, |bp| char_count(&hay[..bp]) as i64);
    interp.set_result(obj::new_wide_int_obj(result));
    Code::Ok
}

/// `string match ?-nocase? pattern string` — glob match (the shared
/// `tcl_syntax::glob` engine, so the dialect never drifts from the compiler /
/// `switch -glob` / `lsearch -glob`).
fn str_match(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 || argv.len() > 5 {
        return interp.wrong_args(b"string match ?-nocase? pattern string");
    }
    let mut nocase = false;
    if argv.len() == 5 {
        let opt = obj_bytes(argv[2]);
        if is_nocase_opt(&opt) {
            nocase = true;
        } else {
            return bad_nocase_option(interp, &opt);
        }
    }
    let pat = obj_bytes(argv[argv.len() - 2]);
    let s = obj_bytes(argv[argv.len() - 1]);
    let m = tcl_syntax::glob::string_case_match(
        &String::from_utf8_lossy(&pat),
        &String::from_utf8_lossy(&s),
        nocase,
    );
    interp.set_result_bytes(if m { b"1" } else { b"0" });
    Code::Ok
}

/// `string map ?-nocase? mapping string` — apply key→value replacements,
/// scanning left to right and trying the keys in list order (first match wins,
/// then skip past the replacement), per `tclCmdMZ.c` `StringMapCmd`.
fn str_map(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 || argv.len() > 5 {
        return interp.wrong_args(b"string map ?-nocase? charMap string");
    }
    let mut nocase = false;
    if argv.len() == 5 {
        let opt = obj_bytes(argv[2]);
        if is_nocase_opt(&opt) {
            nocase = true;
        } else {
            return bad_nocase_option(interp, &opt);
        }
    }
    let mapping = obj_bytes(argv[argv.len() - 2]);
    let s = obj_bytes(argv[argv.len() - 1]);
    let pairs = match crate::parse::split_list(&mapping) {
        Ok(p) => p,
        Err(e) => return interp.set_error(e.message()),
    };
    if pairs.len() % 2 != 0 {
        return interp.set_error(b"char map list unbalanced");
    }

    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    'scan: while i < s.len() {
        let mut k = 0;
        while k < pairs.len() {
            let key = &pairs[k];
            if !key.is_empty() && i + key.len() <= s.len() {
                let region = &s[i..i + key.len()];
                let hit = if nocase {
                    region.eq_ignore_ascii_case(key)
                } else {
                    region == key.as_slice()
                };
                if hit {
                    out.extend_from_slice(&pairs[k + 1]);
                    i += key.len();
                    continue 'scan;
                }
            }
            k += 2;
        }
        // No key matched here: copy one whole UTF-8 character verbatim.
        let cl = utf8_len(s[i]).min(s.len() - i);
        out.extend_from_slice(&s[i..i + cl]);
        i += cl;
    }
    interp.set_result_bytes(&out);
    Code::Ok
}

/// The recognised `string is` classes, in C's declaration order (`tclCmdMZ.c`
/// `StringIsCmd`'s `isClasses[]`). The order is significant: it drives the
/// `bad class`/`ambiguous class` "must be …" diagnostic, and the class is
/// resolved against this table by exact name or unambiguous prefix.
const IS_CLASSES: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("class", IS_CLASS_NAMES);

const IS_CLASS_NAMES: &[&[u8]] = &[
    b"alnum",
    b"alpha",
    b"ascii",
    b"control",
    b"boolean",
    b"dict",
    b"digit",
    b"double",
    b"entier",
    b"false",
    b"graph",
    b"integer",
    b"list",
    b"lower",
    b"print",
    b"punct",
    b"space",
    b"true",
    b"upper",
    b"wideinteger",
    b"wordchar",
    b"xdigit",
];

/// `string is`'s own options (`Tcl_GetIndexFromObj(…, "option", 0)`), in C
/// table order — the two-entry enumeration has no comma before `or`.
const IS_OPTIONS: tcl_cmd_core::prefix::OptionTable<'static, &[u8]> =
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", &[b"-strict", b"-failindex"]);

/// `string is class ?-strict? ?-failindex var? string`. Returns 1/0; with
/// `-failindex`, stores the first failing character index in `var` — but **only
/// when the result is 0** (`StringIsCmd`). Empty input is a class member unless
/// `-strict` (except `list`/`dict`, which ignore `-strict`).
fn str_is(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"string is class ?-strict? ?-failindex var? str";
    if argv.len() < 4 || argv.len() > 7 {
        return interp.wrong_args(USAGE);
    }
    let class_arg = obj_bytes(argv[2]);
    let class: &[u8] = match IS_CLASSES.index_of(&class_arg) {
        Ok(i) => IS_CLASSES.names()[i],
        Err(m) => return interp.set_error(&m),
    };

    // The last argument is always the string under test; the args between the
    // class and it are options (`Tcl_GetIndexFromObj` over IS_OPTIONS).
    let last = argv.len() - 1;
    let mut strict = false;
    let mut failvar: Option<Vec<u8>> = None;
    let mut k = 3;
    while k < last {
        let opt = obj_bytes(argv[k]);
        match IS_OPTIONS.index_of(&opt) {
            Ok(0) => {
                strict = true;
                k += 1;
            }
            Ok(_) => {
                if k + 1 >= last {
                    // C names the resolved class here (`string is double …`), not
                    // the generic "class" the arg-count check uses.
                    let mut usage = b"string is ".to_vec();
                    usage.extend_from_slice(class);
                    usage.extend_from_slice(b" ?-strict? ?-failindex var? str");
                    return interp.wrong_args(&usage);
                }
                failvar = Some(obj_bytes(argv[k + 1]));
                k += 2;
            }
            Err(m) => return interp.set_error(&m),
        }
    }

    let s = obj_bytes(argv[last]);
    let class_str = std::str::from_utf8(class).unwrap_or("");
    let s_str = String::from_utf8_lossy(&s);
    // The numeric classes follow the emulated release's numeral grammar (the
    // ambient syntax this interpreter installed for its runtime version).
    let (ok, fail_index) = tcl_cmd_core::string_is::class_check(
        class_str,
        &s_str,
        strict,
        tcl_syntax::number::runtime_syntax(),
    );

    if !ok {
        if let Some(var) = failvar {
            let o = obj::new_wide_int_obj(fail_index);
            if interp.var_set(&var, o).is_err() {
                drop_fresh(o);
                return cant_set(interp, &var);
            }
        }
    }
    interp.set_result_bytes(if ok { b"1" } else { b"0" });
    Code::Ok
}

// `string is` classification now lives in the shared `tcl_cmd_core::string_is`
// (the per-class membership + fail-index logic). `str_is` above is the thin
// per-runtime wrapper (option parsing + `-failindex` var write) over it.

// -- char helpers (ASCII fast path) ----------------------------------------

#[inline]
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

/// Number of characters. ASCII fast path: byte length == char length.
fn char_count(s: &[u8]) -> usize {
    if s.is_ascii() {
        return s.len();
    }
    let mut n = 0;
    let mut i = 0;
    while i < s.len() {
        i += utf8_len(s[i]);
        n += 1;
    }
    n
}

/// Byte offset of character `ci` (clamped to `s.len()` when `ci` == char count).
/// ASCII fast path: byte offset == char index.
fn char_to_byte(s: &[u8], ci: usize) -> usize {
    if s.is_ascii() {
        return ci.min(s.len());
    }
    let mut i = 0;
    let mut c = 0;
    while i < s.len() && c < ci {
        i += utf8_len(s[i]);
        c += 1;
    }
    i
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn rfind_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(hay.len());
    }
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len())
        .rev()
        .find(|&i| &hay[i..i + needle.len()] == needle)
}

fn parse_isize(b: &[u8]) -> Option<isize> {
    core::str::from_utf8(b).ok()?.trim().parse::<isize>().ok()
}

/// `int` / `end` / `end-N` / `end+N` index spec against `len` chars.
// Index specs (`end`, `int±int`, `end±int`, …) share the full
// `TclGetIntForIndex` grammar with the list commands — reuse one parser.
use crate::cmd_list::index_spec;

// -- error helpers ---------------------------------------------------------
/// Whether `opt` abbreviates `-nocase` (`strncmp` with `length > 1`), the sole
/// option of `string map`/`string match`.
fn is_nocase_opt(opt: &[u8]) -> bool {
    opt.len() > 1 && b"-nocase".starts_with(opt)
}
fn bad_nocase_option(interp: &mut Interp, opt: &[u8]) -> Code {
    let mut m = b"bad option \"".to_vec();
    m.extend_from_slice(opt);
    m.extend_from_slice(b"\": must be -nocase");
    interp.set_error(&m)
}
fn cant_set(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"can't set \"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": variable is array");
    interp.set_error(&m)
}
fn not_integer(interp: &mut Interp, bytes: &[u8]) -> Code {
    let mut m = b"expected integer but got \"".to_vec();
    m.extend_from_slice(bytes);
    m.push(b'"');
    interp.set_error(&m)
}
fn bad_index(interp: &mut Interp, spec: &[u8]) -> Code {
    let mut m = b"bad index \"".to_vec();
    m.extend_from_slice(spec);
    m.extend_from_slice(b"\": must be integer?[+-]integer? or end?[+-]integer?");
    interp.set_error(&m)
}
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

    fn run(src: &[u8]) -> (Code, Vec<u8>) {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(
            counters::finalize(),
            0,
            "leak: {} objs {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
        (code, bytes)
    }
    fn ok(src: &[u8]) -> Vec<u8> {
        let (c, b) = run(src);
        assert_eq!(c, Code::Ok, "result={:?}", String::from_utf8_lossy(&b));
        b
    }

    fn err(src: &[u8]) -> Vec<u8> {
        let (c, b) = run(src);
        assert_eq!(c, Code::Error, "result={:?}", String::from_utf8_lossy(&b));
        b
    }

    #[test]
    fn string_match_map_is() {
        // match (glob, -nocase)
        assert_eq!(ok(b"string match {a*c} abxc"), b"1");
        assert_eq!(ok(b"string match {[a-z]*} Hello"), b"0");
        assert_eq!(ok(b"string match -nocase {A*C} abxc"), b"1");
        // map (ordered, -nocase)
        assert_eq!(ok(b"string map {a 1 b 2} abcab"), b"12c12");
        assert_eq!(ok(b"string map -nocase {AB X} aBcAb"), b"XcX");
        // is
        assert_eq!(ok(b"string is integer 123"), b"1");
        assert_eq!(ok(b"string is integer 12x"), b"0");
        assert_eq!(ok(b"string is integer {}"), b"1"); // empty is a member
        assert_eq!(ok(b"string is integer 0xff"), b"1");
        assert_eq!(ok(b"string is alpha abZ"), b"1");
        assert_eq!(ok(b"string is alpha ab2"), b"0");
        assert_eq!(ok(b"string is double 1.5"), b"1");
        // -failindex reports the first failing character index.
        assert_eq!(ok(b"string is alpha -failindex pos ab2c; set pos"), b"2");
        assert_eq!(ok(b"string is dict {a 1 b 2}"), b"1");
        assert_eq!(ok(b"string is dict {a 1 b}"), b"0");
        // Abbreviated class + option names resolve (Tcl_GetIndexFromObj).
        assert_eq!(ok(b"string is int 42"), b"1");
        assert_eq!(ok(b"string is bool yes"), b"1");
        // Numeric classes tolerate surrounding whitespace; fail index is where
        // parsing stops.
        assert_eq!(ok(b"string is double \"  +1.0e-1 \""), b"1");
        assert_eq!(ok(b"string is integer -fail v 123abc; set v"), b"3");
        // wideinteger overflow reports -1; integer accepts the bignum.
        assert_eq!(
            ok(b"string is wideinteger -fail v 9223372036854775808; set v"),
            b"-1"
        );
        assert_eq!(ok(b"string is integer 9223372036854775808"), b"1");
        // Boolean classes are strict about the keyword set (25 is not a bool).
        assert_eq!(ok(b"string is true 25"), b"0");
        assert_eq!(ok(b"string is bool 1.0"), b"0");
        // -failindex var is left untouched when the result is 1.
        assert_eq!(
            ok(b"catch {unset v}; string is alpha -failindex v abc; info exists v"),
            b"0"
        );
    }

    #[test]
    fn string_compare_equal_options() {
        assert_eq!(ok(b"string compare -nocase ABC abc"), b"0");
        assert_eq!(ok(b"string compare -length 2 abcx abcy"), b"0");
        assert_eq!(ok(b"string equal -nocase ABC abc"), b"1");
        assert_eq!(ok(b"string equal -length 2 abxx abyy"), b"1");
        assert_eq!(
            ok("string equal -nocase -length 1 \u{0130} i".as_bytes()),
            b"1"
        );
        assert_eq!(
            err(b"string equal -length 9223372036854775808 a b"),
            b"integer value too large to represent"
        );
        assert_eq!(
            err(b"string compare -length nope a b"),
            b"expected integer but got \"nope\""
        );
    }

    #[test]
    fn string_replace_insert_word() {
        assert_eq!(ok(b"string replace abcde 1 3 XY"), b"aXYe");
        assert_eq!(ok(b"string replace abcde 1 3"), b"ae");
        assert_eq!(ok(b"string replace abcde 2 1 X"), b"abcde"); // first>last → no-op
        assert_eq!(ok(b"string insert abcde 2 XY"), b"abXYcde");
        assert_eq!(ok(b"string insert abcde end X"), b"abcdeX"); // end == append
        assert_eq!(ok(b"tcl::string::insert 0123 2 _"), b"01_23");
        assert_eq!(ok(b"string wordstart {abc def} 5"), b"4");
        assert_eq!(ok(b"string wordend {abc def} 1"), b"3");
        assert_eq!(ok(b"string wordend ab.cd 2"), b"3"); // single non-word char
    }

    #[test]
    fn tcl_prefix() {
        assert_eq!(
            ok(b"tcl::prefix match {apple apricot banana} app"),
            b"apple"
        );
        assert_eq!(
            ok(b"tcl::prefix all {apple apricot banana} ap"),
            b"apple apricot"
        );
        assert_eq!(ok(b"tcl::prefix longest {apple apricot banana} ap"), b"ap");
        assert_eq!(ok(b"tcl::prefix match -error {} {apple apricot} xy"), b"");
    }

    /// Issue #1607: `string` and `tcl::prefix` are `TclMakeEnsemble` commands,
    /// so both the scan and the whole miss sentence belong to
    /// `tcl_cmd_core::ensemble`; `tcl::prefix`'s dispatch matched exactly and
    /// its enumeration came from `prefix::choice_list_bytes` (the wrong owner —
    /// the same bytes only because the list has three entries).
    /// `tcl::prefix match`'s own options and `string is`'s class/option words
    /// are `Tcl_GetIndexFromObj` tables whose sentences were spelled by hand.
    ///
    /// tclsh 8.6.16 / 9.0.4:
    ///   tcl::prefix {} {a} a       -> unknown or ambiguous subcommand "":
    ///                                 must be all, longest, or match
    ///   tcl::prefix m {a} a        -> a
    ///   tcl::prefix match -e {a} a -> ambiguous option "-e": must be -error,
    ///                                 -exact, or -message
    ///   tcl::prefix match -x {a} a -> bad option "-x": must be <same>
    ///   string {} a                -> unknown or ambiguous subcommand "":
    ///                                 must be cat, compare, …, or wordstart
    ///   string is {} x             -> ambiguous class "": must be alnum, …
    ///   string is integer -z x     -> bad option "-z": must be -strict or -failindex
    #[test]
    fn string_and_prefix_ensembles_resolve_like_tclsh() {
        const OPT_MUST: &str = "must be -error, -exact, or -message";
        const STRING_MUST: &str = "must be cat, compare, equal, first, index, insert, is, last, \
                                   length, map, match, range, repeat, replace, reverse, tolower, \
                                   totitle, toupper, trim, trimleft, trimright, wordend, or \
                                   wordstart";
        assert_eq!(
            err(b"tcl::prefix {} {a} a"),
            b"unknown or ambiguous subcommand \"\": must be all, longest, or match"
        );
        assert_eq!(ok(b"tcl::prefix m {a} a"), b"a");
        assert_eq!(
            err(b"tcl::prefix match -e {a} a"),
            format!("ambiguous option \"-e\": {OPT_MUST}").as_bytes()
        );
        assert_eq!(
            err(b"tcl::prefix match -x {a} a"),
            format!("bad option \"-x\": {OPT_MUST}").as_bytes()
        );
        assert_eq!(
            err(b"string {} a"),
            format!("unknown or ambiguous subcommand \"\": {STRING_MUST}").as_bytes()
        );
        assert_eq!(
            err(b"string to abc"),
            format!("unknown or ambiguous subcommand \"to\": {STRING_MUST}").as_bytes()
        );
        assert_eq!(ok(b"string le hello"), b"5");
        // `string is`'s class and option words are plain option tables.
        assert!(err(b"string is {} x").starts_with(b"ambiguous class \"\": must be alnum, "));
        assert_eq!(
            err(b"string is integer -z x"),
            b"bad option \"-z\": must be -strict or -failindex"
        );
        assert_eq!(ok(b"string is int 12"), b"1");
    }

    #[test]
    fn append_builds_in_place() {
        assert_eq!(ok(b"append s a; append s b c; set s"), b"abc");
        // append onto an unset var creates it
        assert_eq!(ok(b"append fresh hello"), b"hello");
        // many appends stay correct (capacity-backed; no O(n^2))
        assert_eq!(
            ok(b"set i 0; append acc x; append acc y; append acc z"),
            b"xyz"
        );
    }

    /// `append` on an `arr(idx)` reference splits it like `set`/`lappend`
    /// (append-3.2): appending to an element of a *scalar* errors `variable
    /// isn't array` instead of silently creating a bogus `x(0)`-named scalar and
    /// corrupting the store; appending to a real / fresh element works. The
    /// `run` helper's leak + double-free assertions guard the memory safety.
    #[test]
    fn append_array_element() {
        // Element append on a scalar base errors (was: silent corrupt store).
        let (c, m) = run(b"set x {}; append x(0) 44");
        assert_eq!(c, Code::Error);
        assert_eq!(&m, b"can't set \"x(0)\": variable isn't array");
        // Append onto an existing array element grows it.
        assert_eq!(ok(b"array set a {p 1}; append a(p) X; set a(p)"), b"1X");
        // Append onto a fresh element auto-creates the array.
        assert_eq!(ok(b"append fresh(k) hi; set fresh(k)"), b"hi");
        // The no-values read of a missing element reports the element-aware miss.
        let (c, m) = run(b"array set a {p 1}; append a(q)");
        assert_eq!(c, Code::Error);
        assert_eq!(&m, b"can't read \"a(q)\": no such element in array");
    }

    /// A write trace that unsets the variable during `append` (append-7.x): the
    /// result is empty (the var's post-trace value), and the fresh string object
    /// is not freed mid-command — the `run` helper's leak / double-free counters
    /// guard against the former use-after-free.
    #[test]
    fn append_write_trace_unset() {
        assert_eq!(
            ok(b"proc foo args {global y; unset y}\ntrace add variable y write foo\nappend y abc"),
            b""
        );
    }

    /// `append` routed through the shared byte core: byte-exact (a high byte
    /// survives, where the lossy char seam would corrupt it to U+FFFD), the
    /// no-values read matches tclsh (returns the value, errors if unset — the VM
    /// shared the same fix), and the in-place growth now fires the write trace
    /// once (the old in-place path skipped the store and so fired no trace).
    #[test]
    fn append_shared_core_parity() {
        // byte-exact: 0x41 then 0xc8 → "41c8", not a corrupted multi-byte U+FFFD.
        assert_eq!(
            ok(b"set b [binary format H* 41]; append b [binary format H* c8]; binary scan $b H* h; set h"),
            b"41c8"
        );
        // no-values: read the value; error if unset (was a VM bug, now both fixed).
        assert_eq!(ok(b"set s hello; append s"), b"hello");
        let (c, m) = run(b"append nope");
        assert_eq!(c, Code::Error);
        assert_eq!(m, b"can't read \"nope\": no such variable");
        // the in-place growth path fires the write trace exactly once.
        assert_eq!(
            ok(b"set t abc; set n 0; trace add variable t write {incr ::n;#}; append t xyz; set n"),
            b"1"
        );
    }

    #[test]
    fn string_length_index_range() {
        assert_eq!(ok(b"string length hello"), b"5");
        assert_eq!(ok(b"string length {}"), b"0");
        assert_eq!(ok(b"string index hello 1"), b"e");
        assert_eq!(ok(b"string index hello end"), b"o");
        assert_eq!(ok(b"string index hello 9"), b"");
        assert_eq!(ok(b"string range hello 1 3"), b"ell");
        assert_eq!(ok(b"string range hello 2 end"), b"llo");
        // `integer±integer` arithmetic in an index spec (Tcl 9 / safe-base):
        // `string range $s 0 $last-1` with last=5 → indices 0..4.
        assert_eq!(ok(b"string range abcdefghij 0 5-1"), b"abcde");
        assert_eq!(
            ok(b"set last 5; string range abcdefghij 0 $last-1"),
            b"abcde"
        );
        assert_eq!(ok(b"string index abcdef 2+1"), b"d");
    }

    #[test]
    fn string_compare_equal_cat_repeat_reverse() {
        assert_eq!(ok(b"string equal abc abc"), b"1");
        assert_eq!(ok(b"string equal abc abd"), b"0");
        assert_eq!(ok(b"string compare abc abd"), b"-1");
        assert_eq!(ok(b"string compare abc abc"), b"0");
        assert_eq!(ok(b"string cat foo bar baz"), b"foobarbaz");
        assert_eq!(ok(b"string repeat ab 3"), b"ababab");
        assert_eq!(ok(b"string reverse abcd"), b"dcba");
    }

    #[test]
    fn string_case_trim_first_last() {
        assert_eq!(ok(b"string toupper Hello"), b"HELLO");
        assert_eq!(ok(b"string tolower Hello"), b"hello");
        // `totitle`: first char up, rest down (verified vs tclsh 9.0).
        assert_eq!(ok(b"string totitle hello"), b"Hello");
        assert_eq!(ok(b"string totitle {hello world}"), b"Hello world");
        assert_eq!(ok(b"string totitle ABC"), b"Abc");
        // `?first? ?last?` range form (Tcl 9): only the range is case-mapped.
        assert_eq!(ok(b"string totitle abcdef 2 3"), b"abCdef");
        assert_eq!(ok(b"string toupper abcdef 2 3"), b"abCDef");
        assert_eq!(ok(b"string tolower ABCDEF 0 1"), b"abCDEF");
        assert_eq!(ok(b"string trim {  hi  }"), b"hi");
        assert_eq!(ok(b"string trimleft xxhi x"), b"hi");
        assert_eq!(ok(b"string trimright hixx x"), b"hi");
        assert_eq!(ok(b"string first lo hello"), b"3");
        assert_eq!(ok(b"string first zz hello"), b"-1");
        assert_eq!(ok(b"string last l hello"), b"3");
    }

    #[test]
    fn string_first_last_honour_index() {
        // `string first` searches at or after startIndex.
        assert_eq!(ok(b"string first a abcabc"), b"0");
        assert_eq!(ok(b"string first a abcabc 2"), b"3");
        assert_eq!(ok(b"string first a abcabc 4"), b"-1");
        // `string last` finds the last match starting at or before lastIndex.
        assert_eq!(ok(b"string last a abcabc"), b"3");
        assert_eq!(ok(b"string last a abcabc 2"), b"0");
        assert_eq!(ok(b"string last a abcabc end-4"), b"0");
        // negative bound ⇒ nothing matches.
        assert_eq!(ok(b"string first a abc -1"), b"0"); // clamped to 0
    }

    #[test]
    fn utf8_char_indexing() {
        // "héllo" — 'é' is 2 bytes; char ops must count chars, not bytes
        assert_eq!(ok("string length héllo".as_bytes()), b"5");
        assert_eq!(ok("string index héllo 1".as_bytes()), "é".as_bytes());
        assert_eq!(ok("string range héllo 1 2".as_bytes()), "él".as_bytes());
    }

    #[test]
    fn append_shimmers_typed_var() {
        // appending to a list var shimmers it to a string
        assert_eq!(ok(b"set l {a b}; append l c; set l"), b"a bc");
    }
}
