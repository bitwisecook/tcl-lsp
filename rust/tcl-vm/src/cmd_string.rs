//! The `string` ensemble and `append`.

use tcl_runtime_api::Completion;
use tcl_syntax::glob::string_case_match;

use crate::command::resolve_index;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("string", cmd_string);
    vm.register("append", cmd_append);
    // The compiler lowers `string <sub>` to a direct `::tcl::string::<sub>`
    // invocation (the ensemble-rewrite path); register those as forwarders onto
    // the `string` dispatcher. `BuiltinFn` is a plain `fn`, so each closure must
    // be non-capturing (a literal subcommand name).
    vm.register("::tcl::string::cat", |vm, a| string_op(vm, "cat", a));
    vm.register("::tcl::string::compare", |vm, a| string_op(vm, "compare", a));
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
    vm.register("::tcl::string::replace", |vm, a| string_op(vm, "replace", a));
    vm.register("::tcl::string::reverse", |vm, a| string_op(vm, "reverse", a));
    vm.register("::tcl::string::tolower", |vm, a| string_op(vm, "tolower", a));
    vm.register("::tcl::string::totitle", |vm, a| string_op(vm, "totitle", a));
    vm.register("::tcl::string::toupper", |vm, a| string_op(vm, "toupper", a));
    vm.register("::tcl::string::trim", |vm, a| string_op(vm, "trim", a));
    vm.register("::tcl::string::trimleft", |vm, a| string_op(vm, "trimleft", a));
    vm.register("::tcl::string::trimright", |vm, a| string_op(vm, "trimright", a));
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

#[allow(clippy::too_many_lines)]
/// The canonical `string` subcommands (Tcl 9 order), used for unique-prefix
/// resolution and the error message.
const STRING_SUBS: &[&str] = &[
    "cat", "compare", "equal", "first", "index", "insert", "is", "last", "length", "map", "match",
    "range", "repeat", "replace", "reverse", "tolower", "totitle", "toupper", "trim", "trimleft",
    "trimright", "wordend", "wordstart",
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
        return err("wrong # args: should be \"string subcommand ?arg ...?\"");
    };
    let canon = match resolve_string_sub(&sub.to_str()) {
        Ok(c) => c,
        Err(e) => return err(e),
    };
    match canon {
        "length" => match rest {
            [s] => ok(Value::int(ilen(s.to_str().chars().count()))),
            _ => err("wrong # args: should be \"string length string\""),
        },
        "index" => string_index(rest),
        "range" => string_range(rest),
        "equal" => str_compare(rest, true),
        "compare" => str_compare(rest, false),
        "match" => match rest {
            [pat, s] => ok(Value::bool(string_case_match(
                &pat.to_str(),
                &s.to_str(),
                false,
            ))),
            [nocase, pat, s] if &*nocase.to_str() == "-nocase" => ok(Value::bool(
                string_case_match(&pat.to_str(), &s.to_str(), true),
            )),
            _ => err("wrong # args: should be \"string match ?-nocase? pattern string\""),
        },
        "first" => match rest {
            [needle, hay] => {
                let h = hay.to_str();
                let n = needle.to_str();
                let idx = h.find(&*n).map_or(-1, |b| ilen(h[..b].chars().count()));
                ok(Value::int(idx))
            }
            _ => err("wrong # args: should be \"string first needleString haystackString\""),
        },
        "last" => match rest {
            [needle, hay] => {
                let h = hay.to_str();
                let n = needle.to_str();
                let idx = h.rfind(&*n).map_or(-1, |b| ilen(h[..b].chars().count()));
                ok(Value::int(idx))
            }
            _ => err("wrong # args: should be \"string last needleString haystackString\""),
        },
        "tolower" => map_str(rest, str::to_lowercase),
        "toupper" => map_str(rest, str::to_uppercase),
        "totitle" => map_str(rest, totitle),
        "reverse" => map_str(rest, |s| s.chars().rev().collect()),
        "trim" => trim_str(rest, true, true),
        "trimleft" => trim_str(rest, true, false),
        "trimright" => trim_str(rest, false, true),
        "repeat" => match rest {
            [s, n] => match n.as_int() {
                Ok(c) if c >= 0 => ok(Value::string(
                    s.to_str().repeat(usize::try_from(c).unwrap_or(0)),
                )),
                Ok(_) => ok(Value::empty()),
                Err(e) => err(e.message),
            },
            _ => err("wrong # args: should be \"string repeat string count\""),
        },
        "map" => match rest {
            [pairs, s] => string_map(pairs, &s.to_str(), false),
            [opt, pairs, s] if matches!(&*opt.to_str(), "-nocase" | "-no") => {
                string_map(pairs, &s.to_str(), true)
            }
            _ => err("wrong # args: should be \"string map ?-nocase? mapping string\""),
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
        other => err(format!(
            "string {other} is not yet implemented in this VM"
        )),
    }
}

/// `string index string charIndex` — the character at `charIndex`, or empty.
fn string_index(rest: &[Value]) -> Completion<Value> {
    let [s, i] = rest else {
        return err("wrong # args: should be \"string index string charIndex\"");
    };
    let chars: Vec<char> = s.to_str().chars().collect();
    match resolve_index(&i.to_str(), chars.len())
        .and_then(|x| usize::try_from(x).ok())
        .and_then(|x| chars.get(x))
    {
        Some(c) => ok(Value::string(c.to_string())),
        None => ok(Value::empty()),
    }
}

/// `string range string first last` — the substring `first..=last` (clamped).
fn string_range(rest: &[Value]) -> Completion<Value> {
    let [s, first, last] = rest else {
        return err("wrong # args: should be \"string range string first last\"");
    };
    let chars: Vec<char> = s.to_str().chars().collect();
    let len = chars.len();
    let lo = resolve_index(&first.to_str(), len).unwrap_or(0).max(0);
    let hi = resolve_index(&last.to_str(), len).unwrap_or(-1);
    let lo = usize::try_from(lo).unwrap_or(0);
    if hi < 0 || lo >= len {
        return ok(Value::empty());
    }
    let hi = usize::try_from(hi).unwrap_or(0).min(len - 1);
    if lo > hi {
        return ok(Value::empty());
    }
    ok(Value::string(chars[lo..=hi].iter().collect::<String>()))
}

/// `string replace string first last ?newstring?` — remove chars first..last
/// (inclusive), optionally inserting newstring. Out-of-range or first>last
/// leaves the string unchanged.
fn string_replace(rest: &[Value]) -> Completion<Value> {
    if rest.len() < 3 || rest.len() > 4 {
        return err("wrong # args: should be \"string replace string first last ?string?\"");
    }
    let (s, first, last) = (&rest[0], &rest[1], &rest[2]);
    let chars: Vec<char> = s.to_str().chars().collect();
    let len = chars.len();
    let lo = resolve_index(&first.to_str(), len).unwrap_or(0).max(0);
    let hi = resolve_index(&last.to_str(), len).unwrap_or(-1);
    let lo_u = usize::try_from(lo).unwrap_or(0);
    if hi < 0 || lo_u >= len || hi < lo {
        return ok(Value::string(s.to_str().to_string()));
    }
    let hi_u = usize::try_from(hi).unwrap_or(0).min(len - 1);
    let mut out: String = chars[..lo_u].iter().collect();
    if let [_, _, _, repl] = rest {
        out.push_str(&repl.to_str());
    }
    out.extend(chars[hi_u + 1..].iter());
    ok(Value::string(out))
}

/// `string insert string index insertString` — insert before char `index`.
/// Unlike most string ops, `end` denotes the position *after* the last
/// character (so `end` appends).
fn string_insert(rest: &[Value]) -> Completion<Value> {
    let [s, idx, ins] = rest else {
        return err("wrong # args: should be \"string insert string index insertString\"");
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

fn str_compare(rest: &[Value], equal: bool) -> Completion<Value> {
    // Ignore leading options (-nocase/-length) beyond the two operands for M3.
    let ops: Vec<&Value> = rest
        .iter()
        .filter(|v| !v.to_str().starts_with('-'))
        .collect();
    let [a, b] = ops.as_slice() else {
        return err("wrong # args: should be \"string equal/compare ?options? s1 s2\"");
    };
    let ord = (*a.to_str()).cmp(&b.to_str());
    if equal {
        ok(Value::bool(ord.is_eq()))
    } else {
        ok(Value::int(match ord {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }))
    }
}

fn map_str(rest: &[Value], f: impl Fn(&str) -> String) -> Completion<Value> {
    match rest {
        [s] => ok(Value::string(f(&s.to_str()))),
        _ => err("wrong # args: should be \"string <op> string\""),
    }
}

fn totitle(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for (i, c) in s.chars().enumerate() {
        if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn trim_str(rest: &[Value], left: bool, right: bool) -> Completion<Value> {
    let (s, chars) = match rest {
        [s] => (s.to_str(), None),
        [s, c] => (s.to_str(), Some(c.to_str())),
        _ => return err("wrong # args: should be \"string trim string ?chars?\""),
    };
    let set: Vec<char> = chars
        .as_deref()
        .map_or_else(|| vec![' ', '\t', '\n', '\r'], |c| c.chars().collect());
    let pred = |c: char| set.contains(&c);
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
        .chunks_exact(2)
        .map(|c| (c[0].to_str().to_string(), c[1].to_str().to_string()))
        .collect();
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
        for (from, to) in &map {
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
    ok(Value::string(out))
}

fn cmd_append(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((name, vals)) = args.split_first() else {
        return err("wrong # args: should be \"append varName ?value ...?\"");
    };
    let n = name.to_str();
    let mut s = vm
        .var_get(&n)
        .map_or_else(String::new, |v| v.to_str().to_string());
    for v in vals {
        s.push_str(&v.to_str());
    }
    let result = Value::string(s);
    if let Err(e) = vm.var_set(&n, result.clone()) {
        return e;
    }
    ok(result)
}
