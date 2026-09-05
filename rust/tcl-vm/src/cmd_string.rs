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

//! The `string` ensemble and `append`.

use tcl_runtime_api::Completion;
use tcl_syntax::glob::string_case_match;

use crate::command::resolve_index;
use crate::interp::{Vm, err, err_wrong_args, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("append", cmd_append);
    // The compiler lowers `string <sub>` to a direct `::tcl::string::<sub>`
    // invocation (the ensemble-rewrite path); register those as forwarders onto
    // the `string` dispatcher. `BuiltinFn` is a plain `fn`, so each closure must
    // be non-capturing (a literal subcommand name).
    vm.register("::tcl::string::cat", |vm, a| string_op(vm, "cat", a));
    vm.register("::tcl::string::compare", |vm, a| {
        string_op(vm, "compare", a)
    });
    vm.register("::tcl::string::equal", |vm, a| string_op(vm, "equal", a));
    vm.register("::tcl::string::first", |vm, a| string_op(vm, "first", a));
    vm.register("::tcl::string::index", |vm, a| string_op(vm, "index", a));
    vm.register("::tcl::string::insert", |vm, a| string_op(vm, "insert", a));
    vm.register("::tcl::string::is", |vm, a| string_op(vm, "is", a));
    vm.register("::tcl::string::last", |vm, a| string_op(vm, "last", a));
    vm.register("::tcl::string::length", |vm, a| string_op(vm, "length", a));
    vm.register("::tcl::string::map", |vm, a| string_op(vm, "map", a));
    vm.register("::tcl::string::match", |vm, a| string_op(vm, "match", a));
    vm.register("::tcl::string::range", |vm, a| string_op(vm, "range", a));
    vm.register("::tcl::string::repeat", |vm, a| string_op(vm, "repeat", a));
    vm.register("::tcl::string::replace", |vm, a| {
        string_op(vm, "replace", a)
    });
    vm.register("::tcl::string::reverse", |vm, a| {
        string_op(vm, "reverse", a)
    });
    vm.register("::tcl::string::tolower", |vm, a| {
        string_op(vm, "tolower", a)
    });
    vm.register("::tcl::string::totitle", |vm, a| {
        string_op(vm, "totitle", a)
    });
    vm.register("::tcl::string::toupper", |vm, a| {
        string_op(vm, "toupper", a)
    });
    vm.register("::tcl::string::trim", |vm, a| string_op(vm, "trim", a));
    vm.register("::tcl::string::trimleft", |vm, a| {
        string_op(vm, "trimleft", a)
    });
    vm.register("::tcl::string::trimright", |vm, a| {
        string_op(vm, "trimright", a)
    });
    let registry = tcl_registry::CommandRegistry::build_default();
    let spec = registry.get("string").expect("core string spec");
    vm.register_spec_builtin(spec, cmd_string);
}

/// Dispatch a `::tcl::string::<sub>` forwarder by prepending the subcommand and
/// running the normal `string` handler.
fn string_op(vm: &mut Vm, sub: &str, args: &[Value]) -> Completion<Value> {
    let mut full = Vec::with_capacity(args.len() + 1);
    full.push(Value::string(sub));
    full.extend_from_slice(args);
    cmd_string(vm, &full)
}

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// The canonical `string` subcommands (Tcl 9 order), used for unique-prefix
/// resolution and the error message.
const STRING_SUBS: &[&str] = &[
    "cat",
    "compare",
    "equal",
    "first",
    "index",
    "insert",
    "is",
    "last",
    "length",
    "map",
    "match",
    "range",
    "repeat",
    "replace",
    "reverse",
    "tolower",
    "totitle",
    "toupper",
    "trim",
    "trimleft",
    "trimright",
    "wordend",
    "wordstart",
];

/// Resolve a (possibly abbreviated) `string` subcommand to its canonical name,
/// honouring Tcl's unique-prefix matching. Returns the standard error message on
/// no/ambiguous match.
fn resolve_string_sub(input: &str) -> Result<&'static str, String> {
    if let Some(&s) = STRING_SUBS.iter().find(|&&s| s == input) {
        return Ok(s);
    }
    let mut hits = STRING_SUBS.iter().filter(|&&s| s.starts_with(input));
    match (hits.next(), hits.next()) {
        (Some(&s), None) if !input.is_empty() => Ok(s),
        _ => {
            let mut list = String::new();
            for (i, s) in STRING_SUBS.iter().enumerate() {
                if i > 0 {
                    list.push_str(", ");
                }
                if i == STRING_SUBS.len() - 1 {
                    list.push_str("or ");
                }
                list.push_str(s);
            }
            Err(format!(
                "unknown or ambiguous subcommand \"{input}\": must be {list}"
            ))
        }
    }
}

fn cmd_string(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err_wrong_args("string subcommand ?arg ...?");
    };
    let canon = match resolve_string_sub(&sub.to_str()) {
        Ok(c) => c,
        Err(e) => return err(e),
    };
    // `string repeat` is the one subcommand that can allocate without bound in
    // a single command, so it is charged against the value-size limit before
    // the core builds anything. Guarded here rather than in `tcl-cmd-core`
    // because the limit belongs to the interp, and guarded *in addition to*
    // `Op::STR_REPEAT` because the compiled opcode and this command funnel are
    // two paths to the same allocation — the same reason `charge_command`
    // guards both dispatch funnels.
    if canon == "repeat"
        && let [s, count] = rest
    {
        let n = count.as_int().unwrap_or(0).max(0);
        let wanted = (s.to_str().len() as u64).saturating_mul(u64::try_from(n).unwrap_or(0));
        if let Some(refusal) = vm.charge_allocation(wanted) {
            return refusal;
        }
    }
    // Portable subcommands now live in the shared command core (`tcl-cmd-core`);
    // the VM is a thin adapter that maps `Result<Value, CmdError>` onto its
    // `Completion`. Subcommands not yet in the core fall through to the legacy arms.
    if let Some(result) = tcl_cmd_core::string::dispatch_canon(vm, canon, rest) {
        return match result {
            Ok(v) => ok(v),
            Err(e) => err(e.into_message()),
        };
    }
    match canon {
        "match" => string_match(rest),
        "first" => string_first(rest),
        "last" => string_last(rest),
        "tolower" => case_convert(rest, "tolower"),
        "toupper" => case_convert(rest, "toupper"),
        "totitle" => case_convert(rest, "totitle"),
        "trim" => trim_str(rest, "trim", true, true),
        "trimleft" => trim_str(rest, "trimleft", true, false),
        "trimright" => trim_str(rest, "trimright", false, true),
        "map" => match rest {
            [pairs, s] => string_map(pairs, &s.to_str(), false),
            [opt, pairs, s] if is_nocase(&opt.to_str()) => string_map(pairs, &s.to_str(), true),
            [opt, _, _] => err(format!("bad option \"{}\": must be -nocase", opt.to_str())),
            _ => err_wrong_args("string map ?-nocase? charMap string"),
        },
        "cat" => ok(Value::string(
            rest.iter()
                .map(|v| v.to_str().to_string())
                .collect::<String>(),
        )),
        "is" => crate::cmd_string_is::string_is(vm, rest),
        "replace" => string_replace(rest),
        "insert" => string_insert(rest),
        // Resolved to a valid-but-unimplemented subcommand.
        other => err(format!("string {other} is not yet implemented in this VM")),
    }
}

/// `string replace string first last ?newstring?` — remove chars first..last
/// (inclusive), optionally inserting newstring.
/// (tclCmdMZ.c): an empty/inverted range leaves the string unchanged, but an
/// empty *original* string is replaceable (so `string replace {} -1 0 A` → A).
fn string_replace(rest: &[Value]) -> Completion<Value> {
    if rest.len() < 3 || rest.len() > 4 {
        return err_wrong_args("string replace string first last ?string?");
    }
    let (s, first, last) = (&rest[0], &rest[1], &rest[2]);
    let chars: Vec<char> = s.to_str().chars().collect();
    let len = chars.len();
    let end = isize::try_from(len).unwrap_or(isize::MAX) - 1;
    let Some(first) = resolve_index(&first.to_str(), len) else {
        return bad_index(&first.to_str());
    };
    let Some(last) = resolve_index(&last.to_str(), len) else {
        return bad_index(&last.to_str());
    };
    if last < 0 || first > end || last < first {
        return ok(Value::string(s.to_str().to_string()));
    }
    let first = first.max(0);
    let last = last.min(end);
    let lo = usize::try_from(first).unwrap_or(0);
    let hi_excl = usize::try_from(last + 1).unwrap_or(0).min(len);
    let mut out: String = chars[..lo].iter().collect();
    if let [_, _, _, repl] = rest {
        out.push_str(&repl.to_str());
    }
    out.extend(chars[hi_excl..].iter());
    ok(Value::string(out))
}

/// `string insert string index insertString` — insert before char `index`.
/// Unlike most string ops, `end` denotes the position *after* the last
/// character (so `end` appends).
fn string_insert(rest: &[Value]) -> Completion<Value> {
    let [s, idx, ins] = rest else {
        return err_wrong_args("string insert string index insertString");
    };
    let chars: Vec<char> = s.to_str().chars().collect();
    let len = chars.len();
    let at = resolve_index(&idx.to_str(), len + 1).unwrap_or(0);
    let at = if at < 0 {
        0
    } else {
        usize::try_from(at).unwrap_or(len).min(len)
    };
    let mut out: String = chars[..at].iter().collect();
    out.push_str(&ins.to_str());
    out.extend(chars[at..].iter());
    ok(Value::string(out))
}

/// Whether `opt` is `-nocase` (or a non-empty unique abbreviation of it).
fn is_nocase(opt: &str) -> bool {
    opt.len() >= 2 && "-nocase".starts_with(opt)
}

/// `string match ?-nocase? pattern string`.
fn string_match(rest: &[Value]) -> Completion<Value> {
    match rest {
        [pat, s] => ok(Value::bool(string_case_match(
            &pat.to_str(),
            &s.to_str(),
            false,
        ))),
        [opt, pat, s] if is_nocase(&opt.to_str()) => ok(Value::bool(string_case_match(
            &pat.to_str(),
            &s.to_str(),
            true,
        ))),
        [opt, _, _] => err(format!("bad option \"{}\": must be -nocase", opt.to_str())),
        _ => err_wrong_args("string match ?-nocase? pattern string"),
    }
}

/// `string toupper|tolower|totitle string ?first? ?last?` — convert the
/// characters in `[first, last]` (default the whole string).
fn case_convert(rest: &[Value], op: &str) -> Completion<Value> {
    let (s, first_spec, last_spec) = match rest {
        [s] => (s, None, None),
        [s, f] => (s, Some(f), None),
        [s, f, l] => (s, Some(f), Some(l)),
        _ => {
            return err_wrong_args(&format!("string {op} string ?first? ?last?"));
        }
    };
    let chars: Vec<char> = s.to_str().chars().collect();
    let len = chars.len();
    if len == 0 {
        return ok(Value::empty());
    }
    let first = match first_spec {
        None => 0,
        Some(f) => match resolve_index(&f.to_str(), len) {
            Some(i) => i.max(0),
            None => return bad_index(&f.to_str()),
        },
    };
    let last = match last_spec {
        Some(l) => match resolve_index(&l.to_str(), len) {
            Some(i) => i,
            None => return bad_index(&l.to_str()),
        },
        // With only a `first` index, just that one character is converted; with
        // no indices at all, the whole string.
        None if first_spec.is_some() => first,
        None => isize::try_from(len).unwrap_or(isize::MAX) - 1,
    };
    let mut out = String::with_capacity(s.to_str().len());
    for (idx, &c) in chars.iter().enumerate() {
        let i = isize::try_from(idx).unwrap_or(isize::MAX);
        if i < first || i > last {
            out.push(c);
            continue;
        }
        // Per-character Unicode *simple* case mapping, shared with the
        // `STR_UPPER`/`STR_LOWER`/`STR_TITLE` opcodes and the portable
        // `tcl_cmd_core::string::case_convert`, so command and bytecode agree
        // with C's `Tcl_UtfTo{Upper,Lower,Title}` (`string toupper ß` → `ß`,
        // not Rust's full-mapping `SS`). `totitle` titlecases the first
        // character of the range and lowercases the remainder.
        out.push(match op {
            "toupper" => tcl_cmd_core::string::simple_upper(c),
            "tolower" => tcl_cmd_core::string::simple_lower(c),
            _ if i == first => tcl_cmd_core::string::simple_title(c),
            _ => tcl_cmd_core::string::simple_title_rest(c),
        });
    }
    ok(Value::string(out))
}

fn bad_index(spec: &str) -> Completion<Value> {
    err(format!(
        "bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?"
    ))
}

/// `string first needle haystack ?startIndex?` — first occurrence at or after
/// `startIndex` (character index, or -1).
fn string_first(rest: &[Value]) -> Completion<Value> {
    let (needle, hay, start_spec) = match rest {
        [n, h] => (n, h, None),
        [n, h, s] => (n, h, Some(s)),
        _ => {
            return err_wrong_args("string first needleString haystackString ?startIndex?");
        }
    };
    let hay: Vec<char> = hay.to_str().chars().collect();
    let needle: Vec<char> = needle.to_str().chars().collect();
    let start = match start_spec {
        None => 0,
        Some(s) => match resolve_index(&s.to_str(), hay.len()) {
            Some(i) => usize::try_from(i).unwrap_or(0),
            None => return bad_index(&s.to_str()),
        },
    };
    if needle.is_empty() {
        return ok(Value::int(-1));
    }
    let mut idx = -1;
    if needle.len() <= hay.len() {
        for i in start..=hay.len() - needle.len() {
            if hay[i..i + needle.len()] == needle[..] {
                idx = ilen(i);
                break;
            }
        }
    }
    ok(Value::int(idx))
}

/// `string last needle haystack ?lastIndex?` — last occurrence starting at or
/// before `lastIndex` (character index, or -1).
fn string_last(rest: &[Value]) -> Completion<Value> {
    let (needle, hay, last_spec) = match rest {
        [n, h] => (n, h, None),
        [n, h, s] => (n, h, Some(s)),
        _ => {
            return err_wrong_args("string last needleString haystackString ?lastIndex?");
        }
    };
    let hay: Vec<char> = hay.to_str().chars().collect();
    let needle: Vec<char> = needle.to_str().chars().collect();
    // `lastIndex` is the index of the last character considered; the match must
    // end at or before it.
    let last: isize = match last_spec {
        None => isize::try_from(hay.len()).unwrap_or(isize::MAX) - 1,
        Some(s) => match resolve_index(&s.to_str(), hay.len()) {
            Some(i) => i,
            None => return bad_index(&s.to_str()),
        },
    };
    if needle.is_empty() || last < 0 {
        return ok(Value::int(-1));
    }
    let last = usize::try_from(last)
        .unwrap_or(0)
        .min(hay.len().saturating_sub(1));
    let mut idx = -1;
    if !needle.is_empty() && last + 1 >= needle.len() {
        let hi = last + 1 - needle.len();
        for i in (0..=hi).rev() {
            if hay[i..i + needle.len()] == needle[..] {
                idx = ilen(i);
                break;
            }
        }
    }
    ok(Value::int(idx))
}

/// The default `string trim` set — every Unicode space character plus NUL
/// (`tclDefaultTrimSet`, TIP #413).
const DEFAULT_TRIM_SET: &[char] = &[
    '\u{09}', '\u{0a}', '\u{0b}', '\u{0c}', '\u{0d}', ' ', '\u{00}', '\u{85}', '\u{a0}',
    '\u{1680}', '\u{180e}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{200b}', '\u{2028}', '\u{2029}',
    '\u{202f}', '\u{205f}', '\u{2060}', '\u{3000}', '\u{feff}',
];

fn trim_str(rest: &[Value], op: &str, left: bool, right: bool) -> Completion<Value> {
    let (s, chars) = match rest {
        [s] => (s.to_str(), None),
        [s, c] => (s.to_str(), Some(c.to_str())),
        _ => {
            return err_wrong_args(&format!("string {op} string ?chars?"));
        }
    };
    let custom: Option<Vec<char>> = chars.as_deref().map(|c| c.chars().collect());
    let pred = |c: char| match &custom {
        Some(set) => set.contains(&c),
        None => DEFAULT_TRIM_SET.contains(&c),
    };
    let trimmed = match (left, right) {
        (true, true) => s.trim_matches(pred),
        (true, false) => s.trim_start_matches(pred),
        (false, true) => s.trim_end_matches(pred),
        (false, false) => &s,
    };
    ok(Value::string(trimmed))
}

fn string_map(pairs: &Value, s: &str, nocase: bool) -> Completion<Value> {
    let items = match pairs.as_list() {
        Ok(i) => i,
        Err(e) => return err(e.message),
    };
    if items.len() % 2 != 0 {
        return err("char map list unbalanced");
    }
    let map: Vec<(String, String)> = items
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| (c[0].to_str().to_string(), c[1].to_str().to_string()))
        .collect();
    ok(Value::string(map_apply(&map, s, nocase)))
}

/// Apply a `string map` char-map to `s`, left to right: at each position the
/// first pair whose key matches wins and the scan resumes after it.
///
/// Shared by the `string map` command and the `strmap` opcode (which passes a
/// single, always case-sensitive pair — C `INST_STR_MAP`), so the two cannot
/// drift. An empty key never matches (it would not advance).
pub(crate) fn map_apply(map: &[(String, String)], s: &str, nocase: bool) -> String {
    // Case-insensitive matching compares lower-cased keys against a lower-cased
    // view of the remaining input, advancing by the (original) key length.
    let starts = |rest: &str, from: &str| -> bool {
        if nocase {
            rest.chars()
                .zip(from.chars())
                .all(|(a, b)| a.eq_ignore_ascii_case(&b))
                && rest.chars().count() >= from.chars().count()
        } else {
            rest.starts_with(from)
        }
    };
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    'outer: while !rest.is_empty() {
        for (from, to) in map {
            if !from.is_empty() && rest.len() >= from.len() && starts(rest, from) {
                out.push_str(to);
                rest = &rest[from.len()..];
                continue 'outer;
            }
        }
        let ch = rest.chars().next().expect("rest non-empty");
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

fn cmd_append(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((name, vals)) = args.split_first() else {
        return err_wrong_args("append varName ?value ...?");
    };
    let n = name.to_str();
    if vals.is_empty() {
        // `append x` with no values is a read: it fires the read trace, whose
        // error aborts the command exactly as for `set x`, and returns the
        // current value, erroring if the variable is unset (matching tclsh —
        // the old VM wrongly created an empty variable here). `var_get` parses
        // `a(k)`.
        if let Err(c) = vm.fire_var_traces(&n, "read") {
            return c;
        }
        return match vm.var_get(&n) {
            Some(v) => ok(v),
            None => err(format!("can't read \"{n}\": no such variable")),
        };
    }
    // The byte-exact concatenation is shared with the WASM runtime via
    // `tcl_cmd_core::var::append_bytes`; the single store fires the write trace
    // once. The VM's value model never grows in place, so the core rebuilds.
    let cur = vm.var_get(&n);
    let result = tcl_cmd_core::var::append_bytes(vm, cur, vals);
    match vm.store_var_result(&n, result) {
        Ok(stored) => ok(stored),
        Err(e) => e,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_nocase, resolve_string_sub};

    #[test]
    fn resolve_string_sub_prefers_exact_over_prefix() {
        // An exact subcommand wins even when it is itself a prefix of others:
        // `trim` resolves to `trim`, never ambiguous against
        // `trimleft`/`trimright` (Tcl's `Tcl_GetIndexFromObj` exact-match
        // short-circuit). Likewise plain `is`.
        assert_eq!(resolve_string_sub("trim"), Ok("trim"));
        assert_eq!(resolve_string_sub("is"), Ok("is"));
        assert_eq!(resolve_string_sub("length"), Ok("length"));
    }

    #[test]
    fn resolve_string_sub_accepts_unique_prefix() {
        // Unique non-empty prefixes resolve to their one canonical sub.
        assert_eq!(resolve_string_sub("le"), Ok("length"));
        assert_eq!(resolve_string_sub("eq"), Ok("equal"));
        assert_eq!(resolve_string_sub("rev"), Ok("reverse"));
        assert_eq!(resolve_string_sub("words"), Ok("wordstart"));
    }

    #[test]
    fn resolve_string_sub_rejects_ambiguous_prefix() {
        // `l` is ambiguous (last, length); `to` (tolower, totitle, toupper);
        // `wor` (wordend, wordstart). All error rather than guessing.
        assert!(resolve_string_sub("l").is_err());
        assert!(resolve_string_sub("to").is_err());
        assert!(resolve_string_sub("wor").is_err());
    }

    #[test]
    fn resolve_string_sub_rejects_empty_and_unknown() {
        // The empty string is not treated as a prefix of the first sub, and a
        // non-matching token is unknown.
        assert!(resolve_string_sub("").is_err());
        assert!(resolve_string_sub("nope").is_err());
    }

    #[test]
    fn resolve_string_sub_error_lists_canonical_subcommands() {
        let err = resolve_string_sub("zzz").unwrap_err();
        assert!(err.starts_with("unknown or ambiguous subcommand \"zzz\": must be "));
        assert!(err.contains("cat, compare, equal,"));
        // Oxford "or" before the final entry, matching Tcl's option-list error.
        assert!(err.contains("trimright, wordend, or wordstart"));
    }

    #[test]
    fn is_nocase_accepts_unique_abbreviations() {
        // `-nocase` abbreviates to any prefix of length >= 2 (a bare "-" is too
        // short to disambiguate); non-prefixes and the empty string are not.
        assert!(is_nocase("-nocase"));
        assert!(is_nocase("-n"));
        assert!(is_nocase("-noc"));
        assert!(!is_nocase("-"));
        assert!(!is_nocase("-x"));
        assert!(!is_nocase(""));
        assert!(!is_nocase("-nocasex"));
    }
}
