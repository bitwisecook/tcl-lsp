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

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};

/// Register `append` + the `string` ensemble.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"append", append);
    interp.register_builtin(b"string", string_cmd);
}

// -- append ----------------------------------------------------------------

/// `append varName ?value ...?` — append to the string in `varName` (creating
/// it if unset), growing the buffer in place (amortised O(1)) when the value is
/// an unshared plain string, else copy-on-write. Returns the new value.
fn append(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"append varName ?value ...?");
    }
    let name = obj_bytes(argv[1]);
    let values = &argv[2..];

    if values.is_empty() {
        // `append x` with no values just reads the variable.
        return match interp.frames.get(&name) {
            Some(o) => {
                interp.set_result(o);
                Code::Ok
            }
            None => no_such_var(interp, &name),
        };
    }

    // Pick the target: in place if it's an unshared plain string; else a fresh
    // plain string seeded from the current value (or empty).
    let (target, is_new) = match interp.frames.get(&name) {
        Some(o) if obj::is_plain_string(o) && !obj::is_shared(o) => (o, false),
        Some(o) => (obj::new_string_bytes(&obj_bytes(o)), true), // typed/shared → copy
        None => (obj::new_string_bytes(b""), true),
    };

    for &v in values {
        let bytes = obj_bytes(v);
        obj::string_append_inplace(target, &bytes);
    }

    if is_new && interp.frames.set(&name, target).is_err() {
        drop_fresh(target);
        return cant_set(interp, &name);
    }
    interp.set_result(target);
    Code::Ok
}

// -- string ensemble -------------------------------------------------------

fn string_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"string subcommand ?arg ...?");
    }
    let sub = obj_bytes(argv[1]);
    match sub.as_slice() {
        b"length" => str_length(interp, argv),
        b"index" => str_index(interp, argv),
        b"range" => str_range(interp, argv),
        b"equal" => str_equal(interp, argv),
        b"compare" => str_compare(interp, argv),
        b"cat" => str_cat(interp, argv),
        b"repeat" => str_repeat(interp, argv),
        b"reverse" => str_reverse(interp, argv),
        b"toupper" => str_case(interp, argv, true),
        b"tolower" => str_case(interp, argv, false),
        b"trim" => str_trim(interp, argv, true, true),
        b"trimleft" => str_trim(interp, argv, true, false),
        b"trimright" => str_trim(interp, argv, false, true),
        b"first" => str_first_last(interp, argv, true),
        b"last" => str_first_last(interp, argv, false),
        _ => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(&sub);
            m.extend_from_slice(b"\"");
            interp.set_error(&m)
        }
    }
}

fn str_length(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 3 {
        return wrong_args(interp, b"string length string");
    }
    let n = char_count(&obj_bytes(argv[2]));
    interp.set_result(obj::new_wide_int_obj(n as i64));
    Code::Ok
}

fn str_index(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"string index string charIndex");
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
        return wrong_args(interp, b"string range string first last");
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

fn str_equal(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"string equal string1 string2");
    }
    let eq = obj_bytes(argv[2]) == obj_bytes(argv[3]);
    interp.set_result_bytes(if eq { b"1" } else { b"0" });
    Code::Ok
}

fn str_compare(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"string compare string1 string2");
    }
    let c = match obj_bytes(argv[2]).cmp(&obj_bytes(argv[3])) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };
    interp.set_result(obj::new_wide_int_obj(c));
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
        return wrong_args(interp, b"string repeat string count");
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
        return wrong_args(interp, b"string reverse string");
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

fn str_case(interp: &mut Interp, argv: &[*mut TclObj], upper: bool) -> Code {
    if argv.len() != 3 {
        return wrong_args(
            interp,
            if upper {
                b"string toupper string"
            } else {
                b"string tolower string"
            },
        );
    }
    // ASCII case only for now (Unicode case mapping is deferred).
    let mut s = obj_bytes(argv[2]);
    for b in &mut s {
        *b = if upper {
            b.to_ascii_uppercase()
        } else {
            b.to_ascii_lowercase()
        };
    }
    interp.set_result_bytes(&s);
    Code::Ok
}

fn str_trim(interp: &mut Interp, argv: &[*mut TclObj], left: bool, right: bool) -> Code {
    if argv.len() < 3 || argv.len() > 4 {
        return wrong_args(interp, b"string trim string ?chars?");
    }
    let s = obj_bytes(argv[2]);
    let trim_set: Option<Vec<u8>> = if argv.len() == 4 {
        Some(obj_bytes(argv[3]))
    } else {
        None
    };
    let is_trim = |b: u8| match &trim_set {
        Some(set) => set.contains(&b),
        None => matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c),
    };
    let mut start = 0;
    let mut end = s.len();
    if left {
        while start < end && is_trim(s[start]) {
            start += 1;
        }
    }
    if right {
        while end > start && is_trim(s[end - 1]) {
            end -= 1;
        }
    }
    interp.set_result_bytes(&s[start..end]);
    Code::Ok
}

fn str_first_last(interp: &mut Interp, argv: &[*mut TclObj], first: bool) -> Code {
    if argv.len() < 4 || argv.len() > 5 {
        return wrong_args(
            interp,
            if first {
                b"string first needleString haystackString ?startIndex?"
            } else {
                b"string last needleString haystackString ?lastIndex?"
            },
        );
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

    // Byte search restricted by the bound, then convert to a char index.
    let byte_pos = if first {
        // `startIndex`: search at or after it (clamp negatives to 0).
        let start_char = bound.map_or(0, |i| i.max(0) as usize);
        let start_byte = char_to_byte(&hay, start_char);
        find_sub(&hay[start_byte..], &needle).map(|bp| bp + start_byte)
    } else {
        // `lastIndex`: the match must *start* at or before it.
        match bound {
            Some(i) if i < 0 => None, // nothing can start before a negative index
            _ => {
                let last_char = bound.map_or(n, |i| i as usize);
                let start_max = char_to_byte(&hay, last_char);
                let slice_end = (start_max + needle.len()).min(hay.len());
                rfind_sub(&hay[..slice_end], &needle)
            }
        }
    };
    let result = byte_pos.map_or(-1, |bp| char_count(&hay[..bp]) as i64);
    interp.set_result(obj::new_wide_int_obj(result));
    Code::Ok
}

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
fn index_spec(spec: &[u8], len: usize) -> Option<isize> {
    let len = len as isize;
    if spec == b"end" {
        return Some(len - 1);
    }
    if let Some(rest) = spec.strip_prefix(b"end") {
        match rest.first() {
            Some(b'-') => return parse_isize(&rest[1..]).map(|n| len - 1 - n),
            Some(b'+') => return parse_isize(&rest[1..]).map(|n| len - 1 + n),
            _ => return None,
        }
    }
    parse_isize(spec)
}

// -- error helpers ---------------------------------------------------------

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}
fn no_such_var(interp: &mut Interp, name: &[u8]) -> Code {
    let mut m = b"can't read \"".to_vec();
    m.extend_from_slice(name);
    m.extend_from_slice(b"\": no such variable");
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

    #[test]
    fn string_length_index_range() {
        assert_eq!(ok(b"string length hello"), b"5");
        assert_eq!(ok(b"string length {}"), b"0");
        assert_eq!(ok(b"string index hello 1"), b"e");
        assert_eq!(ok(b"string index hello end"), b"o");
        assert_eq!(ok(b"string index hello 9"), b"");
        assert_eq!(ok(b"string range hello 1 3"), b"ell");
        assert_eq!(ok(b"string range hello 2 end"), b"llo");
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
