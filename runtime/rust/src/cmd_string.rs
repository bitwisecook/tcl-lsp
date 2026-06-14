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

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};

/// Register `append` + the `string` ensemble.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"append", append);
    interp.register_builtin(b"string", string_cmd);
    // `::tcl::string::insert` is the real command the `string insert` ensemble
    // maps to; some tests invoke it directly.
    interp.register_builtin(b"::tcl::string::insert", tcl_string_insert);
    // `tcl::prefix` — prefix matching against a table (`tclIndexObj.c`).
    interp.register_builtin(b"::tcl::prefix", tcl_prefix);
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
        return match interp.var_get(&name) {
            Some(o) => {
                interp.set_result(o);
                Code::Ok
            }
            None => no_such_var(interp, &name),
        };
    }

    // Pick the target: in place if it's an unshared plain string; else a fresh
    // plain string seeded from the current value (or empty).
    let (target, is_new) = match interp.var_get(&name) {
        Some(o) if obj::is_plain_string(o) && !obj::is_shared(o) => (o, false),
        Some(o) => (obj::new_string_bytes(&obj_bytes(o)), true), // typed/shared → copy
        None => (obj::new_string_bytes(b""), true),
    };

    for &v in values {
        let bytes = obj_bytes(v);
        obj::string_append_inplace(target, &bytes);
    }

    if is_new && interp.var_set(&name, target).is_err() {
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
        b"wordstart" => str_word(interp, argv, true),
        b"wordend" => str_word(interp, argv, false),
        _ => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(&sub);
            m.extend_from_slice(b"\": must be cat, compare, equal, first, index, insert, is, last, length, map, match, range, repeat, replace, reverse, tolower, totitle, toupper, trim, trimleft, trimright, wordend, or wordstart");
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

/// `string replace string first last ?newstring?` — replace the character range
/// `[first,last]` with `newstring` (or delete it). Out-of-range / `first>last`
/// returns the string unchanged (`StringRplcCmd`).
fn str_replace(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 5 && argv.len() != 6 {
        return wrong_args(interp, b"string replace string first last ?string?");
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
        return wrong_args(interp, b"string insert string index insertString");
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
        return wrong_args(interp, b"tcl::string::insert string index insertString");
    }
    let shifted = [argv[0], argv[0], argv[1], argv[2], argv[3]];
    str_insert(interp, &shifted)
}

/// Render an option set as Tcl's `a, b, or c` / `a or b` / `a` enumeration.
fn enum_must_be(items: &[Vec<u8>]) -> Vec<u8> {
    let mut m = Vec::new();
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            if items.len() > 2 {
                m.extend_from_slice(b", ");
            } else {
                m.push(b' ');
            }
            if i == items.len() - 1 {
                m.extend_from_slice(b"or ");
            }
        }
        m.extend_from_slice(it);
    }
    m
}

/// `tcl::prefix match|all|longest …` — prefix matching against a table
/// (`Tcl_PrefixMatchObjCmd` / `Tcl_PrefixAllObjCmd` / `Tcl_PrefixLongestObjCmd`).
fn tcl_prefix(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"tcl::prefix subcommand ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        b"match" => tcl_prefix_match(interp, argv),
        b"all" => tcl_prefix_all(interp, argv),
        b"longest" => tcl_prefix_longest(interp, argv),
        other => {
            let subs: Vec<Vec<u8>> = [b"all".as_ref(), b"longest", b"match"]
                .iter()
                .map(|s| s.to_vec())
                .collect();
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be ");
            m.extend_from_slice(&enum_must_be(&subs));
            interp.set_error(&m)
        }
    }
}

/// The table elements with `s` as a (string) prefix.
fn prefix_matches(table: &[Vec<u8>], s: &[u8], exact: bool) -> Vec<usize> {
    table
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            if exact {
                e.as_slice() == s
            } else {
                e.starts_with(s)
            }
        })
        .map(|(i, _)| i)
        .collect()
}

fn tcl_prefix_match(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    // tcl::prefix match ?options? table string
    let mut exact = false;
    let mut message: Vec<u8> = b"option".to_vec();
    let mut error_opts: Option<Vec<u8>> = None; // Some("") ⇒ return "" instead of erroring
    let mut i = 2;
    while i + 2 < argv.len() {
        match obj_bytes(argv[i]).as_slice() {
            b"-exact" => {
                exact = true;
                i += 1;
            }
            b"-message" if i + 1 < argv.len() - 2 => {
                message = obj_bytes(argv[i + 1]);
                i += 2;
            }
            b"-error" if i + 1 < argv.len() - 2 => {
                error_opts = Some(obj_bytes(argv[i + 1]));
                i += 2;
            }
            opt => {
                let mut m = b"bad option \"".to_vec();
                m.extend_from_slice(opt);
                m.extend_from_slice(b"\": must be -error, -exact, or -message");
                return interp.set_error(&m);
            }
        }
    }
    if argv.len() - i != 2 {
        return wrong_args(interp, b"tcl::prefix match ?options? table string");
    }
    let table = match crate::parse::split_list(&obj_bytes(argv[i])) {
        Ok(t) => t,
        Err(e) => return interp.set_error(e.message()),
    };
    let s = obj_bytes(argv[i + 1]);
    let hits = prefix_matches(&table, &s, exact);
    // Exact match always wins even if it is also a prefix of others.
    if let Some(&h) = hits.iter().find(|&&h| table[h] == s) {
        interp.set_result(new_string(&table[h]));
        return Code::Ok;
    }
    if hits.len() == 1 {
        interp.set_result(new_string(&table[hits[0]]));
        return Code::Ok;
    }
    // No unique match: error (or return "" if -error {} was given).
    if let Some(opts) = &error_opts {
        if opts.is_empty() {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
    }
    let verb: &[u8] = if hits.is_empty() {
        b"bad "
    } else {
        b"ambiguous "
    };
    let mut m = verb.to_vec();
    m.extend_from_slice(&message);
    m.extend_from_slice(b" \"");
    m.extend_from_slice(&s);
    if table.is_empty() {
        // Empty table: "no valid options" (pluralised message).
        m.extend_from_slice(b"\": no valid ");
        m.extend_from_slice(&message);
        m.push(b's');
    } else {
        m.extend_from_slice(b"\": must be ");
        m.extend_from_slice(&enum_must_be(&table));
    }
    interp.set_error(&m)
}

fn tcl_prefix_all(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() != 4 {
        return wrong_args(interp, b"tcl::prefix all table string");
    }
    let table = match crate::parse::split_list(&obj_bytes(argv[2])) {
        Ok(t) => t,
        Err(e) => return interp.set_error(e.message()),
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
        return wrong_args(interp, b"tcl::prefix longest table string");
    }
    let table = match crate::parse::split_list(&obj_bytes(argv[2])) {
        Ok(t) => t,
        Err(e) => return interp.set_error(e.message()),
    };
    let s = obj_bytes(argv[3]);
    let hits: Vec<&Vec<u8>> = table.iter().filter(|e| e.starts_with(&s)).collect();
    // Longest common prefix of all matching elements (at least `s`).
    let mut longest: Vec<u8> = match hits.first() {
        Some(f) => (*f).clone(),
        None => {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
    };
    for e in &hits[1..] {
        let common = longest
            .iter()
            .zip(e.iter())
            .take_while(|(a, b)| a == b)
            .count();
        longest.truncate(common);
    }
    interp.set_result_bytes(&longest);
    Code::Ok
}

/// `string wordstart|wordend string charIndex` — the bounds of the word
/// containing `charIndex` (`StringStartCmd`/`StringEndCmd`). A word is a maximal
/// run of word-characters, or a single non-word character.
fn str_word(interp: &mut Interp, argv: &[*mut TclObj], start: bool) -> Code {
    if argv.len() != 4 {
        return wrong_args(
            interp,
            if start {
                b"string wordstart string index"
            } else {
                b"string wordend string index"
            },
        );
    }
    let chars: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[2]))
        .chars()
        .collect();
    let n = chars.len();
    let index = match index_spec(&obj_bytes(argv[3]), n) {
        Some(i) => i,
        None => return bad_index(interp, &obj_bytes(argv[3])),
    };
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let result: usize = if start {
        if n == 0 {
            0
        } else {
            let i = index.min(n as isize - 1);
            if i <= 0 {
                0
            } else {
                let i = i as usize;
                if !is_word(chars[i]) {
                    i
                } else {
                    let mut j = i;
                    while j > 0 && is_word(chars[j - 1]) {
                        j -= 1;
                    }
                    j
                }
            }
        }
    } else {
        let i = index.max(0);
        if i >= n as isize {
            n
        } else {
            let i = i as usize;
            let mut cur = i;
            while cur < n && is_word(chars[cur]) {
                cur += 1;
            }
            if cur == i {
                cur + 1
            } else {
                cur
            }
        }
    };
    interp.set_result(obj::new_wide_int_obj(result as i64));
    Code::Ok
}

/// Shared `string equal`/`string compare` engine: `?-nocase? ?-length int?
/// string1 string2`. `-length` limits the comparison to the first N characters.
fn str_compare_equal(interp: &mut Interp, argv: &[*mut TclObj], equal: bool) -> Code {
    let usage: &[u8] = if equal {
        b"string equal ?-nocase? ?-length int? string1 string2"
    } else {
        b"string compare ?-nocase? ?-length int? string1 string2"
    };
    if argv.len() < 4 {
        return wrong_args(interp, usage);
    }
    let mut nocase = false;
    let mut length: Option<usize> = None;
    // Options occupy everything before the trailing two strings.
    let mut i = 2;
    while i < argv.len() - 2 {
        match obj_bytes(argv[i]).as_slice() {
            b"-nocase" => {
                nocase = true;
                i += 1;
            }
            b"-length" => {
                match parse_isize(&obj_bytes(argv[i + 1])) {
                    Some(n) => length = Some(n.max(0) as usize),
                    None => return not_integer(interp, &obj_bytes(argv[i + 1])),
                }
                i += 2;
            }
            _ => return wrong_args(interp, usage),
        }
    }
    if argv.len() - i != 2 {
        return wrong_args(interp, usage);
    }
    let mut a: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[i]))
        .chars()
        .collect();
    let mut b: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[i + 1]))
        .chars()
        .collect();
    if nocase {
        a = a.iter().flat_map(|c| c.to_lowercase()).collect();
        b = b.iter().flat_map(|c| c.to_lowercase()).collect();
    }
    if let Some(n) = length {
        a.truncate(n);
        b.truncate(n);
    }
    let ord = a.cmp(&b);
    if equal {
        interp.set_result_bytes(if ord.is_eq() { b"1" } else { b"0" });
    } else {
        let c = match ord {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
        interp.set_result(obj::new_wide_int_obj(c));
    }
    Code::Ok
}

fn str_equal(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    str_compare_equal(interp, argv, true)
}

fn str_compare(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    str_compare_equal(interp, argv, false)
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

/// `string toupper`/`tolower`/`totitle`: the case mapping to apply.
#[derive(Clone, Copy)]
enum CaseMode {
    Upper,
    Lower,
    /// First char of the range → upper, the rest → lower.
    Title,
}

/// `string toupper|tolower|totitle string ?first? ?last?` — case-map a character
/// range (the whole string by default). ASCII case only for now (non-ASCII
/// bytes pass through unchanged — Unicode case mapping is deferred); the range
/// uses character indices via the shared `index_spec`/`char_to_byte` helpers.
fn str_case(interp: &mut Interp, argv: &[*mut TclObj], mode: CaseMode) -> Code {
    let usage: &[u8] = match mode {
        CaseMode::Upper => b"string toupper string ?first? ?last?",
        CaseMode::Lower => b"string tolower string ?first? ?last?",
        CaseMode::Title => b"string totitle string ?first? ?last?",
    };
    if argv.len() < 3 || argv.len() > 5 {
        return wrong_args(interp, usage);
    }
    let mut s = obj_bytes(argv[2]);
    let n = char_count(&s);
    if n == 0 {
        interp.set_result_bytes(&s);
        return Code::Ok;
    }
    // Default range is the whole string; `?first?`/`?last?` narrow it.
    let lo = match argv.get(3) {
        Some(&a) => match index_spec(&obj_bytes(a), n) {
            Some(i) => i.max(0) as usize,
            None => return bad_index(interp, &obj_bytes(a)),
        },
        None => 0,
    };
    let hi = match argv.get(4) {
        Some(&a) => match index_spec(&obj_bytes(a), n) {
            Some(i) => i,
            None => return bad_index(interp, &obj_bytes(a)),
        },
        None => n as isize - 1,
    };
    if hi < 0 || lo >= n || (hi as usize) < lo {
        interp.set_result_bytes(&s); // empty range ⇒ unchanged
        return Code::Ok;
    }
    let hi = (hi as usize).min(n - 1);
    let b0 = char_to_byte(&s, lo);
    let b1 = char_to_byte(&s, hi + 1);
    // For `totitle`, only the first character of the range is upper-cased.
    let title_first_end = if matches!(mode, CaseMode::Title) {
        char_to_byte(&s, lo + 1)
    } else {
        b0
    };
    for (off, byte) in s[b0..b1].iter_mut().enumerate() {
        let upper = match mode {
            CaseMode::Upper => true,
            CaseMode::Lower => false,
            CaseMode::Title => b0 + off < title_first_end,
        };
        *byte = if upper {
            byte.to_ascii_uppercase()
        } else {
            byte.to_ascii_lowercase()
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

/// `string match ?-nocase? pattern string` — glob match (the shared
/// `tcl_syntax::glob` engine, so the dialect never drifts from the compiler /
/// `switch -glob` / `lsearch -glob`).
fn str_match(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let mut rest = &argv[2..];
    let mut nocase = false;
    if !rest.is_empty() && obj_bytes(rest[0]) == b"-nocase" {
        nocase = true;
        rest = &rest[1..];
    }
    if rest.len() != 2 {
        return wrong_args(interp, b"string match ?-nocase? pattern string");
    }
    let pat = obj_bytes(rest[0]);
    let s = obj_bytes(rest[1]);
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
    let mut rest = &argv[2..];
    let mut nocase = false;
    if !rest.is_empty() && obj_bytes(rest[0]) == b"-nocase" {
        nocase = true;
        rest = &rest[1..];
    }
    if rest.len() != 2 {
        return wrong_args(interp, b"string map ?-nocase? charMap string");
    }
    let mapping = obj_bytes(rest[0]);
    let s = obj_bytes(rest[1]);
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
const IS_CLASSES: &[&[u8]] = &[
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

/// `string is` option table (for the `-strict`/`-failindex` prefix match).
const IS_OPTIONS: &[&[u8]] = &[b"-strict", b"-failindex"];

/// The outcome of an exact/unambiguous-prefix table lookup (`Tcl_GetIndexFromObj`).
enum Lookup {
    /// Resolved to `table[index]` (exact match, or a unique prefix).
    Found(usize),
    /// The prefix matched more than one entry.
    Ambiguous,
    /// No entry matched.
    None,
}

/// Resolve `arg` against `table` like `Tcl_GetIndexFromObj` (flags 0): an exact
/// match wins, else a *unique* prefix; ambiguous and no-match are distinguished
/// so the caller can emit `bad`/`ambiguous` as C does.
fn index_lookup(table: &[&[u8]], arg: &[u8]) -> Lookup {
    if let Some(i) = table.iter().position(|s| *s == arg) {
        return Lookup::Found(i);
    }
    // A leading "-" with nothing after is not treated as an option/class prefix
    // by Tcl_GetIndexFromObj; an empty arg matches nothing.
    if arg.is_empty() {
        return Lookup::None;
    }
    let mut found = None;
    for (i, s) in table.iter().enumerate() {
        if s.starts_with(arg) {
            if found.is_some() {
                return Lookup::Ambiguous;
            }
            found = Some(i);
        }
    }
    match found {
        Some(i) => Lookup::Found(i),
        None => Lookup::None,
    }
}

/// `string is class ?-strict? ?-failindex var? string`. Returns 1/0; with
/// `-failindex`, stores the first failing character index in `var` — but **only
/// when the result is 0** (`StringIsCmd`). Empty input is a class member unless
/// `-strict` (except `list`/`dict`, which ignore `-strict`).
fn str_is(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    const USAGE: &[u8] = b"string is class ?-strict? ?-failindex var? str";
    if argv.len() < 4 || argv.len() > 7 {
        return wrong_args(interp, USAGE);
    }
    let class_arg = obj_bytes(argv[2]);
    let class: &[u8] = match index_lookup(IS_CLASSES, &class_arg) {
        Lookup::Found(i) => IS_CLASSES[i],
        Lookup::Ambiguous => return interp.set_error(&class_err(b"ambiguous class", &class_arg)),
        Lookup::None => return interp.set_error(&class_err(b"bad class", &class_arg)),
    };

    // The last argument is always the string under test; the args between the
    // class and it are options (`Tcl_GetIndexFromObj` over IS_OPTIONS).
    let last = argv.len() - 1;
    let mut strict = false;
    let mut failvar: Option<Vec<u8>> = None;
    let mut k = 3;
    while k < last {
        let opt = obj_bytes(argv[k]);
        match index_lookup(IS_OPTIONS, &opt) {
            Lookup::Found(0) => {
                strict = true;
                k += 1;
            }
            Lookup::Found(_) => {
                if k + 1 >= last {
                    // C names the resolved class here (`string is double …`), not
                    // the generic "class" the arg-count check uses.
                    let mut usage = b"string is ".to_vec();
                    usage.extend_from_slice(class);
                    usage.extend_from_slice(b" ?-strict? ?-failindex var? str");
                    return wrong_args(interp, &usage);
                }
                failvar = Some(obj_bytes(argv[k + 1]));
                k += 2;
            }
            Lookup::Ambiguous => return interp.set_error(&option_err(b"ambiguous option", &opt)),
            Lookup::None => return interp.set_error(&option_err(b"bad option", &opt)),
        }
    }

    let s = obj_bytes(argv[last]);
    let (ok, fail_index) = class_check(class, &s, strict);

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

/// `bad class "X": must be …` / `ambiguous class "X": must be …`.
fn class_err(kind: &[u8], arg: &[u8]) -> Vec<u8> {
    let mut m = kind.to_vec();
    m.extend_from_slice(b" \"");
    m.extend_from_slice(arg);
    m.extend_from_slice(b"\": must be ");
    let classes: Vec<Vec<u8>> = IS_CLASSES.iter().map(|s| s.to_vec()).collect();
    m.extend_from_slice(&crate::ensemble::must_be(&classes));
    m
}

/// `bad option "X": must be -strict or -failindex` (two-item form: no comma).
fn option_err(kind: &[u8], arg: &[u8]) -> Vec<u8> {
    let mut m = kind.to_vec();
    m.extend_from_slice(b" \"");
    m.extend_from_slice(arg);
    m.extend_from_slice(b"\": must be -strict or -failindex");
    m
}

/// Run the class membership test. Returns `(is_member, fail_index)`; the fail
/// index (a **character** offset, or -1) is only meaningful when not a member.
fn class_check(class: &[u8], s: &[u8], strict: bool) -> (bool, i64) {
    match class {
        b"integer" | b"wideinteger" | b"entier" | b"double" => is_number_class(class, s, strict),
        b"boolean" | b"true" | b"false" => is_boolean_class(class, s, strict),
        b"list" => {
            // `list`/`dict` ignore strictness (empty is a well-formed list).
            if crate::parse::split_list(s).is_ok() {
                (true, -1)
            } else {
                (false, char_count(s) as i64)
            }
        }
        b"dict" => {
            if crate::parse::split_list(s).is_ok_and(|l| l.len() % 2 == 0) {
                (true, -1)
            } else {
                (false, char_count(s) as i64)
            }
        }
        _ => {
            // Per-character class: first failing char index.
            if s.is_empty() {
                return (!strict, 0);
            }
            for (failat, c) in String::from_utf8_lossy(s).chars().enumerate() {
                if !char_class_ok(class, c) {
                    return (false, failat as i64);
                }
            }
            (true, -1)
        }
    }
}

/// Tcl numeric whitespace (matches `tcl_syntax::number`'s internal set): space,
/// tab, newline, VT, FF, CR. `TclParseNumber` consumes a surrounding run of it.
#[inline]
fn is_num_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `string is integer|wideinteger|entier|double`. Defers to the shared
/// `TclParseNumber` port: surrounding whitespace is allowed, the fail index is
/// where parsing stopped, and `wideinteger` reports -1 on a value that parses
/// but overflows a wide.
fn is_number_class(class: &[u8], s: &[u8], strict: bool) -> (bool, i64) {
    use tcl_syntax::number::{self, Number, ParseFlags};

    if s.is_empty() {
        return (!strict, 0);
    }
    let Ok(st) = std::str::from_utf8(s) else {
        return (false, 0);
    };
    let is_double = class == b"double";
    let flags = ParseFlags {
        integer_only: !is_double,
        ..Default::default()
    };
    let Some(p) = number::parse(st, flags) else {
        return (false, 0); // no numeric prefix at all → fail at the start
    };
    // For the integer classes, reject the `Inf`/`NaN` alpha forms that the
    // shared parser still classifies (C's `TCL_PARSE_INTEGER_ONLY` would not).
    let is_int_val = matches!(p.number, Number::Int(_) | Number::Big { .. });
    if !is_double && !is_int_val {
        return (false, 0);
    }
    // `TclParseNumber`'s end-pointer consumes a trailing whitespace run.
    let mut stop = p.end;
    while stop < s.len() && is_num_ws(s[stop]) {
        stop += 1;
    }
    if stop < s.len() {
        // Some prefix parsed, but not the whole string. The fail index equals
        // the byte stop (everything up to it is ASCII ws/digits, so byte == char).
        return (false, stop as i64);
    }
    // The entire string is a number.
    if class == b"wideinteger" {
        match p.number {
            Number::Int(_) => (true, -1),
            _ => (false, -1), // a valid integer, but it overflows a wide
        }
    } else {
        (true, -1)
    }
}

/// `string is boolean|true|false`. A non-member fails at index 0 (the whole
/// string is rejected as one unit).
fn is_boolean_class(class: &[u8], s: &[u8], strict: bool) -> (bool, i64) {
    match parse_boolean(s) {
        Some(v) => {
            let member = match class {
                b"true" => v,
                b"false" => !v,
                _ => true,
            };
            (member, 0)
        }
        None => {
            if strict {
                (false, 0)
            } else {
                // Non-strict: empty is a member; anything else is not.
                (s.is_empty(), 0)
            }
        }
    }
}

/// `Tcl_GetBoolean`'s `ParseBoolean` (`tclObj.c`): `0`/`1`, or a case-insensitive
/// unambiguous prefix of `true`/`false`/`yes`/`no`/`on`/`off`. Returns the
/// boolean value, or `None` when the string is not a valid boolean.
fn parse_boolean(s: &[u8]) -> Option<bool> {
    let len = s.len();
    if !(1..=5).contains(&len) {
        return None; // "false" is the longest valid spelling
    }
    match s[0] {
        b'0' => return (len == 1).then_some(false),
        b'1' => return (len == 1).then_some(true),
        _ => {}
    }
    // Lower-case, rejecting any byte outside the boolean-keyword letter set.
    let mut lower = [0u8; 5];
    for (i, &c) in s.iter().enumerate() {
        let lc = c.to_ascii_lowercase();
        if !matches!(
            lc,
            b'a' | b'e' | b'f' | b'l' | b'n' | b'o' | b'r' | b's' | b't' | b'u' | b'y'
        ) {
            return None;
        }
        lower[i] = lc;
    }
    let lc = &lower[..len];
    // `strncmp(lc, keyword, len) == 0` ⇔ `keyword.starts_with(lc)`.
    match lc[0] {
        b'y' => b"yes".starts_with(lc).then_some(true),
        b'n' => b"no".starts_with(lc).then_some(false),
        b't' => b"true".starts_with(lc).then_some(true),
        b'f' => b"false".starts_with(lc).then_some(false),
        b'o' if len >= 2 && b"on".starts_with(lc) => Some(true),
        b'o' if len >= 2 && b"off".starts_with(lc) => Some(false),
        _ => None,
    }
}

/// Per-character `string is` class membership.
fn char_class_ok(class: &[u8], c: char) -> bool {
    match class {
        b"alnum" => c.is_alphanumeric(),
        b"alpha" => c.is_alphabetic(),
        b"ascii" => (c as u32) < 0x80,
        b"control" => c.is_control(),
        b"digit" => c.is_numeric(),
        b"graph" => !c.is_whitespace() && !c.is_control() && (c as u32) != 0x20,
        b"lower" => c.is_lowercase(),
        b"print" => !c.is_control(),
        b"punct" => c.is_ascii_punctuation(),
        b"space" => c.is_whitespace(),
        b"upper" => c.is_uppercase(),
        b"wordchar" => c.is_alphanumeric() || c == '_',
        b"xdigit" => c.is_ascii_hexdigit(),
        _ => false,
    }
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
// Index specs (`end`, `int±int`, `end±int`, …) share the full
// `TclGetIntForIndex` grammar with the list commands — reuse one parser.
use crate::cmd_list::index_spec;

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
