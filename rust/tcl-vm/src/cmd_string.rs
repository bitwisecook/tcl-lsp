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
    vm.register_stock_builtin("append", cmd_append);
    register_string(vm);
}

fn register_string(vm: &mut Vm) {
    let Some(namespace) = vm
        .native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::String)
    else {
        vm.register_stock_builtin("string", cmd_string);
        return;
    };
    let subs = crate::environment::release_subcommands(
        vm.actual_native_execution_profile().name,
        "string",
        STRING_SUBS,
    );
    vm.register_stock_namespace_ensemble("string", namespace, STRING_MEMBERS, subs);
}

/// Repinning retains user replacements and selects the actual private surface.
pub(crate) fn refresh_profile(vm: &mut Vm) {
    if vm.stock_native_identity("string").as_deref() != Some("string") {
        return;
    }
    for &(member, _) in STRING_MEMBERS {
        let target = format!("::tcl::string::{member}");
        if vm.stock_native_identity(&target).as_deref() == target.strip_prefix("::") {
            vm.remove_registered_command(target.trim_start_matches("::"));
        }
    }
    register_string(vm);
    if vm
        .native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::String)
        .is_none()
    {
        vm.retire_unused_stock_ensemble_namespace("string");
    }
}

const STRING_MEMBERS: &[(&str, crate::command::BuiltinFn)] = &[
    ("cat", |vm, args| string_op(vm, "cat", args)),
    ("compare", |vm, args| string_op(vm, "compare", args)),
    ("equal", |vm, args| string_op(vm, "equal", args)),
    ("first", |vm, args| string_op(vm, "first", args)),
    ("index", |vm, args| string_op(vm, "index", args)),
    ("insert", |vm, args| string_op(vm, "insert", args)),
    ("is", |vm, args| string_op(vm, "is", args)),
    ("last", |vm, args| string_op(vm, "last", args)),
    ("length", |vm, args| string_op(vm, "length", args)),
    ("map", |vm, args| string_op(vm, "map", args)),
    ("match", |vm, args| string_op(vm, "match", args)),
    ("range", |vm, args| string_op(vm, "range", args)),
    ("repeat", |vm, args| string_op(vm, "repeat", args)),
    ("replace", |vm, args| string_op(vm, "replace", args)),
    ("reverse", |vm, args| string_op(vm, "reverse", args)),
    ("tolower", |vm, args| string_op(vm, "tolower", args)),
    ("totitle", |vm, args| string_op(vm, "totitle", args)),
    ("toupper", |vm, args| string_op(vm, "toupper", args)),
    ("trim", |vm, args| string_op(vm, "trim", args)),
    ("trimleft", |vm, args| string_op(vm, "trimleft", args)),
    ("trimright", |vm, args| string_op(vm, "trimright", args)),
    ("wordend", |vm, args| string_op(vm, "wordend", args)),
    ("wordstart", |vm, args| string_op(vm, "wordstart", args)),
];

/// Dispatch a `::tcl::string::<sub>` forwarder by prepending the subcommand and
/// running the normal `string` handler.
fn string_op(vm: &mut Vm, sub: &str, args: &[Value]) -> Completion<Value> {
    let mut full = Vec::with_capacity(args.len() + 1);
    full.push(Value::string(sub));
    full.extend_from_slice(args);
    cmd_string(vm, &full)
}

/// Reach the actual host length handler independently of authored TMM policy.
pub(crate) fn physical_string_length(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    string_op(vm, "length", args)
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

/// Resolve a (possibly abbreviated) `string` subcommand to its canonical name
/// through the shared ensemble owner (`string` is a `TclMakeEnsemble` command),
/// or the ensemble's own miss sentence — including its comma before `or`, which
/// `prefix::choice_list` words differently.
fn resolve_string_sub<'a>(subs: &[&'a str], input: &str) -> Result<&'a str, String> {
    match tcl_cmd_core::ensemble::resolve_subcommand(subs, input.as_bytes(), true) {
        Some(index) => Ok(subs[index]),
        None => Err(
            String::from_utf8_lossy(&tcl_cmd_core::ensemble::unknown_subcommand_message(
                subs,
                input.as_bytes(),
                true,
                b"::tcl::string",
            ))
            .into_owned(),
        ),
    }
}

fn cmd_string(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err_wrong_args(vm, "string subcommand ?arg ...?");
    };
    // `insert` arrives in Tcl 9 and `bytelength` leaves with it, so the table
    // is the emulated release's: on 8.6 `string in` is `index`, on 9.0 it is
    // ambiguous with `insert`.
    let subs = crate::environment::release_subcommands(
        vm.runtime_version().dialect_profile_name(),
        "string",
        STRING_SUBS,
    );
    let canon = match resolve_string_sub(subs, &sub.to_str()) {
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
        return string_repeat(vm, s, count);
    }
    // Portable subcommands now live in the shared command core (`tcl-cmd-core`);
    // the VM is a thin adapter that maps `Result<Value, CmdError>` onto its
    // `Completion`. Subcommands not yet in the core fall through to the legacy arms.
    if let Some(result) = tcl_cmd_core::string::dispatch_canon(vm, canon, rest) {
        return match result {
            Ok(v) => ok(v),
            Err(e) => crate::command::completion_from_cmd_error(vm, e),
        };
    }
    match canon {
        "match" => string_match(vm, rest),
        "first" => string_first(vm, rest),
        "last" => string_last(vm, rest),
        "tolower" => case_convert(vm, rest, "tolower"),
        "toupper" => case_convert(vm, rest, "toupper"),
        "totitle" => case_convert(vm, rest, "totitle"),
        "trim" => trim_str(vm, rest, "trim", true, true),
        "trimleft" => trim_str(vm, rest, "trimleft", true, false),
        "trimright" => trim_str(vm, rest, "trimright", false, true),
        "map" => match rest {
            [pairs, s] => string_map(vm, pairs, &s.to_str(), false),
            [opt, pairs, s] if is_nocase(&opt.to_str()) => string_map(vm, pairs, &s.to_str(), true),
            [opt, _, _] => err(format!("bad option \"{}\": must be -nocase", opt.to_str())),
            _ => err_wrong_args(vm, "string map ?-nocase? charMap string"),
        },
        "cat" => ok(Value::string(
            rest.iter()
                .map(|v| v.to_str().to_string())
                .collect::<String>(),
        )),
        "is" => crate::cmd_string_is::string_is(vm, rest),
        "replace" => string_replace(vm, rest),
        "insert" => string_insert(vm, rest),
        // Resolved to a valid-but-unimplemented subcommand.
        other => err(format!("string {other} is not yet implemented in this VM")),
    }
}

/// Normalize once before charging the actual byte allocation. In particular,
/// Jim's expression count cannot bypass the budget with an integer parse miss.
fn string_repeat(vm: &mut Vm, s: &Value, count: &Value) -> Completion<Value> {
    let n = match tcl_cmd_core::string::prepare_repeat_count(vm, count) {
        Ok(n) => n,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let wanted = u64::try_from(s.string_bytes().len())
        .unwrap_or(u64::MAX)
        .saturating_mul(u64::try_from(n.max(0)).unwrap_or(u64::MAX));
    if let Some(refusal) = vm.charge_allocation(wanted) {
        return refusal;
    }
    match tcl_cmd_core::string::repeat_with_count(vm, s, n) {
        Ok(value) => ok(value),
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

/// `string replace string first last ?newstring?` — remove chars first..last
/// (inclusive), optionally inserting newstring.
/// (tclCmdMZ.c): an empty/inverted range leaves the string unchanged, but an
/// empty *original* string is replaceable (so `string replace {} -1 0 A` → A).
fn string_replace(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    if rest.len() < 3 || rest.len() > 4 {
        return err_wrong_args(vm, "string replace string first last ?string?");
    }
    let (s, first, last) = (&rest[0], &rest[1], &rest[2]);
    let chars: Vec<char> = s.to_str().chars().collect();
    let len = chars.len();
    let end = isize::try_from(len).unwrap_or(isize::MAX) - 1;
    let Some(first) = resolve_index(vm, &first.to_str(), len) else {
        return bad_index(vm, &first.to_str());
    };
    let Some(last) = resolve_index(vm, &last.to_str(), len) else {
        return bad_index(vm, &last.to_str());
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
fn string_insert(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [s, idx, ins] = rest else {
        return err_wrong_args(vm, "string insert string index insertString");
    };
    let chars: Vec<char> = s.to_str().chars().collect();
    let len = chars.len();
    let Some(at) = resolve_index(vm, &idx.to_str(), len + 1) else {
        return bad_index(vm, &idx.to_str());
    };
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
fn string_match(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
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
        _ => err_wrong_args(vm, "string match ?-nocase? pattern string"),
    }
}

/// `string toupper|tolower|totitle string ?first? ?last?` — convert the
/// characters in `[first, last]` (default the whole string).
fn case_convert(vm: &mut Vm, rest: &[Value], op: &str) -> Completion<Value> {
    let (s, first_spec, last_spec) = match rest {
        [s] => (s, None, None),
        [s, f] => (s, Some(f), None),
        [s, f, l] => (s, Some(f), Some(l)),
        _ => {
            return err_wrong_args(vm, &format!("string {op} string ?first? ?last?"));
        }
    };
    let chars: Vec<char> = s.to_str().chars().collect();
    let len = chars.len();
    if len == 0 {
        return ok(Value::empty());
    }
    let first = match first_spec {
        None => 0,
        Some(f) => match resolve_index(vm, &f.to_str(), len) {
            Some(i) => i.max(0),
            None => return bad_index(vm, &f.to_str()),
        },
    };
    let last = match last_spec {
        Some(l) => match resolve_index(vm, &l.to_str(), len) {
            Some(i) => i,
            None => return bad_index(vm, &l.to_str()),
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

fn bad_index(vm: &Vm, spec: &str) -> Completion<Value> {
    crate::command::bad_index(vm, spec)
}

/// `string first needle haystack ?startIndex?` — first occurrence at or after
/// `startIndex` (character index, or -1).
fn string_first(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (needle, hay, start_spec) = match rest {
        [n, h] => (n, h, None),
        [n, h, s] => (n, h, Some(s)),
        _ => {
            return err_wrong_args(vm, "string first needleString haystackString ?startIndex?");
        }
    };
    let hay: Vec<char> = hay.to_str().chars().collect();
    let needle: Vec<char> = needle.to_str().chars().collect();
    let start = match start_spec {
        None => 0,
        Some(s) => match resolve_index(vm, &s.to_str(), hay.len()) {
            Some(i) => usize::try_from(i).unwrap_or(0),
            None => return bad_index(vm, &s.to_str()),
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
fn string_last(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let (needle, hay, last_spec) = match rest {
        [n, h] => (n, h, None),
        [n, h, s] => (n, h, Some(s)),
        _ => {
            return err_wrong_args(vm, "string last needleString haystackString ?lastIndex?");
        }
    };
    let hay: Vec<char> = hay.to_str().chars().collect();
    let needle: Vec<char> = needle.to_str().chars().collect();
    // `lastIndex` is the index of the last character considered; the match must
    // end at or before it.
    let last: isize = match last_spec {
        None => isize::try_from(hay.len()).unwrap_or(isize::MAX) - 1,
        Some(s) => match resolve_index(vm, &s.to_str(), hay.len()) {
            Some(i) => i,
            None => return bad_index(vm, &s.to_str()),
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

fn trim_str(vm: &mut Vm, rest: &[Value], op: &str, left: bool, right: bool) -> Completion<Value> {
    let (s, chars) = match rest {
        [s] => (s.to_str(), None),
        [s, c] => (s.to_str(), Some(c.to_str())),
        _ => {
            return err_wrong_args(vm, &format!("string {op} string ?chars?"));
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

fn string_map(vm: &mut Vm, pairs: &Value, s: &str, nocase: bool) -> Completion<Value> {
    let items = match pairs.as_list() {
        Ok(i) => i,
        Err(e) => return crate::command::completion_from_tcl_error(vm, e),
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
        return err_wrong_args(vm, "append varName ?value ...?");
    };
    let name = match vm.native_name_operand_bytes(name) {
        Ok(name) => name,
        Err(error) => {
            return vm.refuse_host_command(format!("native append name is unavailable: {error:?}"));
        }
    };
    if vals.is_empty() {
        return match vm.read_variable_result_bytes(&name, None) {
            Ok(value) => ok(value),
            Err(completion) => completion,
        };
    }
    match vm.append_captured_bytes(&name, None, vals) {
        Ok(stored) => ok(stored),
        Err(completion) => completion,
    }
}

#[cfg(test)]
mod tests {
    use super::{STRING_SUBS, is_nocase};

    #[test]
    fn jim_repeat_charges_the_normalized_expression_count_and_exact_bytes() {
        use crate::{interp::Vm, value::Value};
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = Vm::new();
        vm.set_dialect_profile(profile);
        vm.set_value_size_limit_value(Some(2));
        let source = Value::from_string_bytes([0xff].as_slice());
        let allowed = super::string_repeat(&mut vm, &source, &Value::string("1+1"));
        assert!(allowed.code.is_ok());
        assert_eq!(allowed.result.string_bytes().as_ref(), &[0xff, 0xff]);

        let refused = super::string_repeat(&mut vm, &source, &Value::string("1+2"));
        assert_eq!(refused.code, tcl_runtime_api::Code::Error);
        assert_eq!(
            refused.result.to_str().as_ref(),
            "value size limit exceeded"
        );

        let invalid = super::string_repeat(&mut vm, &source, &Value::string("bogus"));
        assert_eq!(invalid.code, tcl_runtime_api::Code::Error);
        assert_ne!(
            invalid.result.to_str().as_ref(),
            "value size limit exceeded"
        );
    }

    /// The unit tests below exercise the resolver over the engine's full
    /// (Tcl 9) table; the release filter that narrows it per pin is covered
    /// end-to-end in `tests/cmd_info_prefix_e2e.rs`.
    fn resolve_string_sub(input: &str) -> Result<&'static str, String> {
        super::resolve_string_sub(STRING_SUBS, input)
    }

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
