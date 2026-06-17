//! `string is class ?-strict? ?-failindex var? str` — character-class and
//! value-class membership tests.
//!
//! Mirrors `StringIsCmd` (tclCmdMZ.c): classes may be abbreviated, `-failindex`
//! reports the *character* index of the first failure, and an empty string is in
//! every class unless `-strict` is given.

use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// Canonical class names, in the order Tcl lists them in error messages.
const CLASSES: &[&str] = &[
    "alnum", "alpha", "ascii", "control", "boolean", "dict", "digit", "double", "entier", "false",
    "graph", "integer", "list", "lower", "print", "punct", "space", "true", "upper", "wideinteger",
    "wordchar", "xdigit",
];

fn oxford(items: &[&str]) -> String {
    let mut out = String::new();
    for (i, e) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        if items.len() > 1 && i == items.len() - 1 {
            out.push_str("or ");
        }
        out.push_str(e);
    }
    out
}

/// Cast a character index to `i64` for a fail-index result.
fn ci(i: usize) -> i64 {
    i64::try_from(i).unwrap_or(i64::MAX)
}

/// Resolve a (possibly abbreviated) class name; `Err` carries the bad/ambiguous
/// message.
fn resolve_class(input: &str) -> Result<&'static str, String> {
    if let Some(&c) = CLASSES.iter().find(|&&c| c == input) {
        return Ok(c);
    }
    let hits: Vec<&&str> = CLASSES.iter().filter(|&&c| c.starts_with(input)).collect();
    match hits.as_slice() {
        [one] => Ok(**one),
        [] => Err(format!(
            "bad class \"{input}\": must be {}",
            oxford(CLASSES)
        )),
        _ => Err(format!(
            "ambiguous class \"{input}\": must be {}",
            oxford(CLASSES)
        )),
    }
}

/// `string is …`. `rest` is everything after the `is` subcommand.
pub(crate) fn string_is(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    const WRONG: &str = "wrong # args: should be \"string is class ?-strict? ?-failindex var? str\"";
    if rest.len() < 2 {
        return err(WRONG);
    }
    let class = match resolve_class(&rest[0].to_str()) {
        Ok(c) => c,
        Err(e) => return err(e),
    };
    let mut strict = false;
    let mut fail_var: Option<Value> = None;
    let mut i = 1;
    while i < rest.len() - 1 {
        let opt = rest[i].to_str();
        if "-strict".starts_with(&*opt) && opt.len() >= 2 {
            strict = true;
            i += 1;
        } else if "-failindex".starts_with(&*opt) && opt.len() >= 2 {
            let Some(v) = rest.get(i + 1) else {
                return err(format!(
                    "wrong # args: should be \"string is {class} ?-strict? ?-failindex var? str\""
                ));
            };
            fail_var = Some(v.clone());
            i += 2;
        } else {
            break;
        }
    }
    if rest.len() - i > 1 {
        // Too many arguments — Tcl uses the generic "class" form here.
        return err(WRONG);
    }
    if rest.len() - i < 1 {
        return err(format!(
            "wrong # args: should be \"string is {class} ?-strict? ?-failindex var? str\""
        ));
    }
    let s = rest[i].to_str();
    let chars: Vec<char> = s.chars().collect();
    let (member, fail) = class_check(class, &chars, strict);
    if !member
        && let Some(var) = fail_var
        && let Err(e) = vm.set_var(&var.to_str(), Value::int(fail))
    {
        return e;
    }
    ok(Value::bool(member))
}

/// Returns `(is_member, fail_index)`. `fail_index` is a character offset (or -1
/// for the "valid list but wrong shape" cases, e.g. an odd-length dict).
fn class_check(class: &str, chars: &[char], strict: bool) -> (bool, i64) {
    if chars.is_empty() {
        // The empty string is the valid empty list/dict even under `-strict`;
        // for the other classes `-strict` rejects it.
        if matches!(class, "list" | "dict") {
            return (true, -1);
        }
        return (!strict, 0);
    }
    match class {
        // Only `wideinteger` is bounded to 64 bits; `integer`/`entier` accept
        // arbitrary-precision values in Tcl 9.
        "wideinteger" => scan_integer(chars, true),
        "integer" | "entier" => scan_integer(chars, false),
        "double" => scan_double(chars),
        "boolean" | "true" | "false" => (check_boolean(chars, class), 0),
        "list" => scan_list(chars).0,
        "dict" => scan_dict(chars),
        _ => char_class(class, chars),
    }
}

/// Per-character classes: returns the index of the first failing character.
fn char_class(class: &str, chars: &[char]) -> (bool, i64) {
    let pred: fn(char) -> bool = match class {
        "alnum" => |c| c.is_alphanumeric(),
        "alpha" => char::is_alphabetic,
        "ascii" => |c| (c as u32) < 0x80,
        "control" => char::is_control,
        "digit" => |c| c.is_ascii_digit(),
        "graph" => |c| !c.is_control() && !c.is_whitespace() && c != '\u{0}',
        "lower" => char::is_lowercase,
        "print" => |c| !c.is_control(),
        "punct" => |c| {
            !c.is_alphanumeric() && !c.is_whitespace() && !c.is_control() && !c.is_ascii_digit()
        },
        "space" => char::is_whitespace,
        "upper" => char::is_uppercase,
        "wordchar" => |c| c.is_alphanumeric() || c == '_',
        "xdigit" => |c| c.is_ascii_hexdigit(),
        _ => return (false, 0),
    };
    match chars.iter().position(|&c| !pred(c)) {
        Some(idx) => (false, ci(idx)),
        None => (true, -1),
    }
}

fn check_boolean(chars: &[char], class: &str) -> bool {
    // Tcl accepts `0`/`1` and any *unique* case-insensitive prefix of the
    // boolean words (so `f`→false, `tru`→true, but `o` is ambiguous on/off).
    let lc: String = chars.iter().collect::<String>().to_ascii_lowercase();
    let s = lc.as_str();
    let truthy = s == "1" || ["true", "yes", "on"].iter().any(|w| w.starts_with(s));
    let falsy = s == "0" || ["false", "no", "off"].iter().any(|w| w.starts_with(s));
    match class {
        "true" => truthy && !falsy,
        "false" => falsy && !truthy,
        _ => truthy ^ falsy,
    }
}

/// Scan a Tcl integer, returning `(valid, fail_index)`. `fail_index` is the
/// character offset where parsing first failed. When `bounded`, a syntactically
/// valid value that overflows a signed 64-bit integer is rejected (index -1).
fn scan_integer(chars: &[char], bounded: bool) -> (bool, i64) {
    let n = chars.len();
    let mut i = 0;
    while i < n && chars[i].is_whitespace() {
        i += 1;
    }
    let neg = i < n && chars[i] == '-';
    if i < n && (chars[i] == '+' || chars[i] == '-') {
        i += 1;
    }
    let mut radix = 10;
    if i + 1 < n && chars[i] == '0' {
        match chars[i + 1].to_ascii_lowercase() {
            'x' => radix = 16,
            'o' => radix = 8,
            'b' => radix = 2,
            'd' => radix = 10,
            _ => radix = 0, // plain leading 0 — still decimal, but keep the 0
        }
        if radix != 0 {
            i += 2;
        } else {
            radix = 10;
        }
    }
    let digits_start = i;
    while i < n && is_radix_digit(chars[i], radix) {
        i += 1;
    }
    let digits_end = i;
    while i < n && chars[i].is_whitespace() {
        i += 1;
    }
    let has_digits = digits_end > digits_start;
    if !has_digits || i != n {
        // No number at all ⇒ the failure is at the start; otherwise it is the
        // first character past the valid digits.
        return (false, if has_digits { ci(digits_end.min(n)) } else { 0 });
    }
    if bounded {
        let digit_str: String = chars[digits_start..digits_end].iter().collect();
        let fits = match u64::from_str_radix(&digit_str, radix) {
            Ok(m) if neg => m <= 1u64 << 63,
            Ok(m) => m < 1u64 << 63,
            Err(_) => false,
        };
        if !fits {
            return (false, -1);
        }
    }
    (true, -1)
}

fn is_radix_digit(c: char, radix: u32) -> bool {
    match radix {
        16 => c.is_ascii_hexdigit(),
        8 => ('0'..='7').contains(&c),
        2 => c == '0' || c == '1',
        _ => c.is_ascii_digit(),
    }
}

/// Scan a Tcl double (also accepts integers). Only ASCII whitespace surrounds a
/// number (so a trailing NBSP makes it invalid); the fail index is the end of
/// the longest valid floating-point prefix, or 0 when there is no number.
fn scan_double(chars: &[char]) -> (bool, i64) {
    let ascii_ws = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0b}' | '\u{0c}');
    let n = chars.len();
    let mut i = 0;
    while i < n && ascii_ws(chars[i]) {
        i += 1;
    }
    let core_start = i;
    if i < n && (chars[i] == '+' || chars[i] == '-') {
        i += 1;
    }
    let mut has_digits = false;
    while i < n && chars[i].is_ascii_digit() {
        i += 1;
        has_digits = true;
    }
    if i < n && chars[i] == '.' {
        i += 1;
        while i < n && chars[i].is_ascii_digit() {
            i += 1;
            has_digits = true;
        }
    }
    // A valid exponent extends the prefix; an `e` with no following digits ends
    // it (so `1.0e4e4` stops at the second `e`).
    if has_digits && i < n && (chars[i] == 'e' || chars[i] == 'E') {
        let mut j = i + 1;
        if j < n && (chars[j] == '+' || chars[j] == '-') {
            j += 1;
        }
        if j < n && chars[j].is_ascii_digit() {
            i = j;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
    }
    let core_end = i;
    while i < n && ascii_ws(chars[i]) {
        i += 1;
    }
    if has_digits && i == n {
        let core: String = chars[core_start..core_end].iter().collect();
        if core.parse::<f64>().is_ok() {
            return (true, -1);
        }
    }
    (false, if has_digits { ci(core_end) } else { 0 })
}

fn is_list_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0b}' | '\u{0c}')
}

/// Validate a Tcl list, returning `(valid, fail_index)` and the element count.
fn scan_list_counted(chars: &[char]) -> ((bool, i64), usize) {
    let n = chars.len();
    let mut i = 0;
    let mut count = 0;
    loop {
        while i < n && is_list_space(chars[i]) {
            i += 1;
        }
        if i >= n {
            return ((true, -1), count);
        }
        let start = i;
        count += 1;
        if chars[i] == '{' {
            let mut depth = 1;
            i += 1;
            while i < n && depth > 0 {
                match chars[i] {
                    '\\' => i = (i + 2).min(n),
                    '{' => {
                        depth += 1;
                        i += 1;
                    }
                    '}' => {
                        depth -= 1;
                        i += 1;
                    }
                    _ => i += 1,
                }
            }
            if depth != 0 || (i < n && !is_list_space(chars[i])) {
                return ((false, ci(start)), count);
            }
        } else if chars[i] == '"' {
            i += 1;
            while i < n && chars[i] != '"' {
                i = if chars[i] == '\\' { (i + 2).min(n) } else { i + 1 };
            }
            if i >= n || (i + 1 < n && !is_list_space(chars[i + 1])) {
                return ((false, ci(start)), count);
            }
            i += 1;
        } else {
            while i < n && !is_list_space(chars[i]) {
                i = if chars[i] == '\\' { (i + 2).min(n) } else { i + 1 };
            }
        }
    }
}

fn scan_list(chars: &[char]) -> ((bool, i64), usize) {
    scan_list_counted(chars)
}

fn scan_dict(chars: &[char]) -> (bool, i64) {
    let ((valid, fail), count) = scan_list_counted(chars);
    if !valid {
        return (false, fail);
    }
    if count % 2 == 0 {
        (true, -1)
    } else {
        // A valid list with an odd number of elements is not a dict; Tcl reports
        // a fail index of -1 for this shape error.
        (false, -1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(class: &str, s: &str, strict: bool) -> (bool, i64) {
        class_check(class, &s.chars().collect::<Vec<_>>(), strict)
    }

    #[test]
    fn char_classes_failindex() {
        assert_eq!(check("alpha", "abc5def", false), (false, 3));
        assert_eq!(check("alnum", "abc1.23", false), (false, 4));
        assert_eq!(check("alpha", "abc", false), (true, -1));
        assert_eq!(check("space", "a b", false), (false, 0));
        assert_eq!(check("lower", "abCd", false), (false, 2));
    }

    #[test]
    fn empty_and_strict() {
        assert_eq!(check("alpha", "", false), (true, 0));
        assert_eq!(check("alpha", "", true), (false, 0));
    }

    #[test]
    fn integer_failindex() {
        assert_eq!(check("integer", "123abc", false), (false, 3));
        assert_eq!(check("integer", "1.0", false), (false, 1));
        assert_eq!(check("integer", "0o36963", false), (false, 4));
        assert_eq!(check("integer", "123", false), (true, -1));
        assert_eq!(check("integer", "0x1f", false), (true, -1));
    }

    #[test]
    fn list_and_dict() {
        assert_eq!(check("list", "a b c", false), (true, -1));
        assert_eq!(check("list", "a {b c", false), (false, 2));
        assert_eq!(check("list", "a b {b c}d e", false), (false, 4));
        assert_eq!(check("dict", "a b c d", false), (true, -1));
        assert_eq!(check("dict", "a b c", false), (false, -1));
    }
}
