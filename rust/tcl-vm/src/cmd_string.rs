//! The `string` ensemble and `append`.

use tcl_runtime_api::Completion;
use tcl_syntax::glob::string_case_match;

use crate::command::resolve_index;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("string", cmd_string);
    vm.register("append", cmd_append);
}

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[allow(clippy::too_many_lines)]
fn cmd_string(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"string subcommand ?arg ...?\"");
    };
    match &*sub.to_str() {
        "length" => match rest {
            [s] => ok(Value::int(ilen(s.to_str().chars().count()))),
            _ => err("wrong # args: should be \"string length string\""),
        },
        "index" => match rest {
            [s, i] => {
                let chars: Vec<char> = s.to_str().chars().collect();
                match resolve_index(&i.to_str(), chars.len())
                    .and_then(|x| usize::try_from(x).ok())
                    .and_then(|x| chars.get(x))
                {
                    Some(c) => ok(Value::string(c.to_string())),
                    None => ok(Value::empty()),
                }
            }
            _ => err("wrong # args: should be \"string index string charIndex\""),
        },
        "range" => match rest {
            [s, first, last] => {
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
            _ => err("wrong # args: should be \"string range string first last\""),
        },
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
            [pairs, s] => string_map(pairs, &s.to_str()),
            _ => err("wrong # args: should be \"string map ?-nocase? mapping string\""),
        },
        "cat" => ok(Value::string(
            rest.iter()
                .map(|v| v.to_str().to_string())
                .collect::<String>(),
        )),
        "is" => match rest {
            [class, s] => ok(Value::bool(string_is(&class.to_str(), &s.to_str()))),
            _ => err("wrong # args: should be \"string is class ?-strict? str\""),
        },
        other => err(format!("unknown or ambiguous subcommand \"{other}\"")),
    }
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

fn string_map(pairs: &Value, s: &str) -> Completion<Value> {
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
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    'outer: while !rest.is_empty() {
        for (from, to) in &map {
            if !from.is_empty() && rest.starts_with(from.as_str()) {
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

fn string_is(class: &str, s: &str) -> bool {
    if s.is_empty() {
        return true; // Tcl: empty string is in every class (without -strict).
    }
    match class {
        "integer" | "wideinteger" => s.trim().parse::<i64>().is_ok(),
        "double" | "real" => s.trim().parse::<f64>().is_ok(),
        "boolean" | "true" | "false" => matches!(
            s.to_ascii_lowercase().as_str(),
            "0" | "1" | "true" | "false" | "yes" | "no" | "on" | "off"
        ),
        "alpha" => s.chars().all(char::is_alphabetic),
        "alnum" => s.chars().all(char::is_alphanumeric),
        "digit" => s.chars().all(|c| c.is_ascii_digit()),
        "xdigit" => s.chars().all(|c| c.is_ascii_hexdigit()),
        "space" => s.chars().all(char::is_whitespace),
        "upper" => s.chars().all(char::is_uppercase),
        "lower" => s.chars().all(char::is_lowercase),
        "punct" => s.chars().all(|c| c.is_ascii_punctuation()),
        "ascii" => s.is_ascii(),
        _ => false,
    }
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
