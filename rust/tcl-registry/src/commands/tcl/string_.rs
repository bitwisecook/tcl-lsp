//! `string` — perform one of several string operations.

use crate::prelude::*;

/// SYNC-JUN02b-6 (#519): compile-time folds for pure `string`
/// subcommands, consumed by the optimiser's O129 general-builtin
/// constant-fold path through the registry `const_fold` callbacks.
///
/// Each receives the arguments *after* the subcommand word (so
/// `[string toupper foo]` calls [`fold_toupper`] with `["foo"]`) and
/// is restricted to ASCII input: Rust's `to_ascii_uppercase` /
/// `to_ascii_lowercase` and char-reversal agree with Tcl's
/// `Tcl_UtfToUpper` / `Tcl_UtfToLower` / `string reverse` exactly on
/// ASCII, while non-ASCII case mapping diverges (Rust's full Unicode
/// `char::to_uppercase` can expand one char to several, e.g. ß → SS,
/// whereas Tcl maps 1:1). Bailing on non-ASCII is conservative —
/// never a wrong fold.
fn fold_toupper(args: &[&str]) -> Option<String> {
    match args {
        [s] if s.is_ascii() => Some(s.to_ascii_uppercase()),
        _ => None,
    }
}

fn fold_tolower(args: &[&str]) -> Option<String> {
    match args {
        [s] if s.is_ascii() => Some(s.to_ascii_lowercase()),
        _ => None,
    }
}

fn fold_reverse(args: &[&str]) -> Option<String> {
    match args {
        [s] if s.is_ascii() => Some(s.chars().rev().collect()),
        _ => None,
    }
}

fn fold_length(args: &[&str]) -> Option<String> {
    // ASCII-only: for ASCII the byte length equals the character count,
    // matching Tcl's `string length` (number of characters).  Non-ASCII
    // bails — the char count diverges across Tcl 8.x (UTF-16 units) and
    // Tcl 9 / Rust (Unicode scalars) for astral characters.
    match args {
        [s] if s.is_ascii() => Some(s.len().to_string()),
        _ => None,
    }
}

/// `string cat ?string ...?` — pure concatenation (no transformation, so
/// sound for any input; the O129 path quotes the result as one word).
/// Always folds, but the `ConstFoldFn` signature is `-> Option<String>`.
#[allow(clippy::unnecessary_wraps)]
fn fold_cat(args: &[&str]) -> Option<String> {
    Some(args.concat())
}

/// `string repeat string count` — repeat (bounded, matching Python's
/// 10000 sanity cap).  No char transformation → sound for any input.
/// A negative count fails the `usize` parse → bails (matches Python).
fn fold_repeat(args: &[&str]) -> Option<String> {
    let [s, count] = args else {
        return None;
    };
    let n: usize = count.trim().parse().ok()?;
    if n > 10_000 {
        return None;
    }
    Some(s.repeat(n))
}

/// The Tcl default trim set (`string trim` with no explicit chars):
/// space, tab, newline, CR, vertical tab, form feed.
const TRIM_WS: &[char] = &[' ', '\t', '\n', '\r', '\u{0b}', '\u{0c}'];

/// `string trim` / `trimleft` / `trimright`.  ASCII-restricted so the
/// default whitespace set and the explicit chars set match Tcl exactly.
fn fold_trim_impl(args: &[&str], left: bool, right: bool) -> Option<String> {
    let (s, chars): (&str, Vec<char>) = match args {
        [s] => (s, TRIM_WS.to_vec()),
        [s, chars] => (s, chars.chars().collect()),
        _ => return None,
    };
    if !s.is_ascii() || !chars.iter().all(char::is_ascii) {
        return None;
    }
    let pred = |c: char| chars.contains(&c);
    let out = match (left, right) {
        (true, true) => s.trim_matches(pred),
        (true, false) => s.trim_start_matches(pred),
        (false, true) => s.trim_end_matches(pred),
        (false, false) => s,
    };
    Some(out.to_owned())
}

fn fold_trim(args: &[&str]) -> Option<String> {
    fold_trim_impl(args, true, true)
}

fn fold_trimleft(args: &[&str]) -> Option<String> {
    fold_trim_impl(args, true, false)
}

fn fold_trimright(args: &[&str]) -> Option<String> {
    fold_trim_impl(args, false, true)
}

/// `string totitle string` — the no-index form only (first char upper,
/// rest lower).  ASCII-restricted.
fn fold_totitle(args: &[&str]) -> Option<String> {
    let [s] = args else {
        return None;
    };
    if !s.is_ascii() {
        return None;
    }
    if s.is_empty() {
        return Some(String::new());
    }
    let (first, rest) = s.split_at(1); // ASCII → first char is one byte
    Some(format!(
        "{}{}",
        first.to_ascii_uppercase(),
        rest.to_ascii_lowercase()
    ))
}

use crate::const_fold::{clamp_range, parse_index};

/// `string index string charIndex`.  ASCII-restricted (byte index ==
/// char index for ASCII).
fn fold_index(args: &[&str]) -> Option<String> {
    let [s, idx_str] = args else {
        return None;
    };
    let (s, idx_str) = (*s, *idx_str);
    if !s.is_ascii() {
        return None;
    }
    let idx = parse_index(idx_str, s.len())?;
    Some(match usize::try_from(idx) {
        Ok(i) if i < s.len() => s[i..=i].to_owned(),
        _ => String::new(), // negative or out of range → ""
    })
}

/// `string range string first last`.  ASCII-restricted.
fn fold_range(args: &[&str]) -> Option<String> {
    let [s, first_s, last_s] = args else {
        return None;
    };
    let s = *s;
    if !s.is_ascii() {
        return None;
    }
    let first = parse_index(first_s, s.len())?;
    let last = parse_index(last_s, s.len())?;
    match clamp_range(first, last, s.len()) {
        Some((lo, hi)) => Some(s[lo..=hi].to_owned()),
        None => Some(String::new()),
    }
}

/// `string replace string first last ?newString?`.  ASCII-restricted.
fn fold_replace(args: &[&str]) -> Option<String> {
    let (s, first_s, last_s, repl) = match args {
        [s, f, l] => (*s, *f, *l, ""),
        [s, f, l, r] => (*s, *f, *l, *r),
        _ => return None,
    };
    if !s.is_ascii() {
        return None;
    }
    let first = parse_index(first_s, s.len())?;
    let last = parse_index(last_s, s.len())?;
    match clamp_range(first, last, s.len()) {
        Some((lo, hi)) => Some(format!("{}{}{}", &s[..lo], repl, &s[hi + 1..])),
        None => Some(s.to_owned()),
    }
}

/// `string first needleString haystackString ?startIndex?`.
/// ASCII-restricted.  Returns the byte/char index, or `-1`.
fn fold_first(args: &[&str]) -> Option<String> {
    let (needle, haystack, start) = match args {
        [n, h] => (*n, *h, 0usize),
        [n, h, st] => {
            if !h.is_ascii() {
                return None;
            }
            // A negative start clamps to 0.
            (
                *n,
                *h,
                usize::try_from(parse_index(st, h.len())?).unwrap_or(0),
            )
        }
        _ => return None,
    };
    if !needle.is_ascii() || !haystack.is_ascii() {
        return None;
    }
    let pos = haystack
        .get(start..)
        .and_then(|tail| tail.find(needle))
        .map(|i| i + start);
    Some(pos.map_or_else(|| "-1".to_owned(), |i| i.to_string()))
}

/// `string last needleString haystackString ?lastIndex?`.
/// ASCII-restricted.  Searches `haystack[0..end)` from the right.
fn fold_last(args: &[&str]) -> Option<String> {
    let (needle, haystack, end_idx) = match args {
        [n, h] => (*n, *h, None),
        [n, h, last] => {
            if !h.is_ascii() {
                return None;
            }
            (*n, *h, Some(parse_index(last, h.len())? + 1))
        }
        _ => return None,
    };
    if !needle.is_ascii() || !haystack.is_ascii() {
        return None;
    }
    let end = match end_idx {
        None => haystack.len(),
        // Clamp the (lastIndex + 1) exclusive end into `[0, len]`.
        Some(e) => usize::try_from(e).unwrap_or(0).min(haystack.len()),
    };
    let pos = haystack.get(..end).and_then(|head| head.rfind(needle));
    Some(pos.map_or_else(|| "-1".to_owned(), |i| i.to_string()))
}

/// `string compare ?-nocase? ?-length N? string1 string2`.
/// ASCII-restricted.  Returns `-1` / `0` / `1`.
fn fold_compare(args: &[&str]) -> Option<String> {
    let mut nocase = false;
    let mut length: Option<usize> = None;
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        match args[i] {
            "-nocase" => {
                nocase = true;
                i += 1;
            }
            "-length" if i + 1 < args.len() => {
                length = Some(args[i + 1].parse().ok()?);
                i += 2;
            }
            "--" => {
                i += 1;
                break;
            }
            _ => return None,
        }
    }
    if args.len() - i != 2 {
        return None;
    }
    let (mut s1, mut s2) = (args[i].to_owned(), args[i + 1].to_owned());
    if !s1.is_ascii() || !s2.is_ascii() {
        return None;
    }
    if nocase {
        s1 = s1.to_ascii_lowercase();
        s2 = s2.to_ascii_lowercase();
    }
    if let Some(n) = length {
        s1.truncate(n.min(s1.len()));
        s2.truncate(n.min(s2.len()));
    }
    Some(
        match s1.cmp(&s2) {
            std::cmp::Ordering::Less => "-1",
            std::cmp::Ordering::Equal => "0",
            std::cmp::Ordering::Greater => "1",
        }
        .to_owned(),
    )
}

/// `string equal ?-nocase? ?-length N? string1 string2`.
fn fold_equal(args: &[&str]) -> Option<String> {
    match fold_compare(args)?.as_str() {
        "0" => Some("1".to_owned()),
        _ => Some("0".to_owned()),
    }
}

/// `string map ?-nocase? mapping string`.  ASCII-restricted (byte-exact
/// greedy left-to-right replacement matching Tcl's `string map`; the
/// `mapping` is a list of old/new pairs, first matching pair wins).
fn fold_string_map(args: &[&str]) -> Option<String> {
    let (mapping_str, s) = match args {
        [m, s] => (*m, *s),
        ["-nocase", m, s] => {
            // -nocase handled below via case-insensitive byte compare.
            return fold_string_map_impl(m, s, true);
        }
        _ => return None,
    };
    fold_string_map_impl(mapping_str, s, false)
}

fn fold_string_map_impl(mapping_str: &str, s: &str, nocase: bool) -> Option<String> {
    if !mapping_str.is_ascii() || !s.is_ascii() {
        return None;
    }
    let pairs = crate::const_fold::split_list(mapping_str)?;
    if pairs.len() % 2 != 0 {
        return None;
    }
    let reps: Vec<(&str, &str)> = pairs
        .chunks_exact(2)
        .map(|kv| (kv[0].as_str(), kv[1].as_str()))
        .collect();
    let sb = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut pos = 0;
    while pos < sb.len() {
        let mut matched = false;
        for (old, new) in &reps {
            let ob = old.as_bytes();
            if ob.is_empty() || pos + ob.len() > sb.len() {
                continue;
            }
            let window = &sb[pos..pos + ob.len()];
            let hit = if nocase {
                window.eq_ignore_ascii_case(ob)
            } else {
                window == ob
            };
            if hit {
                out.push_str(new);
                pos += ob.len();
                matched = true;
                break;
            }
        }
        if !matched {
            out.push(sb[pos] as char); // ASCII byte → char
            pos += 1;
        }
    }
    Some(out)
}

/// `string is class ?-strict? ?-failindex var? string` (SYNC-JUN02d-1
/// follow-up, #525 B-tail).  Constant-folds the **Tcl-faithful** classes
/// the optimiser can decide soundly.
///
/// Deliberately *not* a transcription of Python's `str`-method fold,
/// whose semantics diverge from Tcl: e.g. `str.islower("abc1")` is
/// `True` but `string is lower abc1` is `0` (Tcl requires *every* char
/// to be a lowercase letter; a digit fails).  The ASCII character
/// classes here apply the predicate to every char and **bail on
/// non-ASCII input** (Unicode class membership isn't modelled — a
/// missed fold, never a wrong one).  `ascii` is the membership test
/// itself, so it is defined for any input.  `boolean` / `true` /
/// `false` use the exact `Tcl_GetBoolean` keyword + unique-prefix set
/// (so `t` / `ye` / `of` resolve, `o` — ambiguous between on/off —
/// does not).  The number (`integer` / `entier` / `wideinteger` /
/// `double`) and `list` / `dict` classes are **deferred** (Tcl number /
/// list syntax + range edges need differential pinning) — they bail.
fn fold_is(args: &[&str]) -> Option<String> {
    if args.len() < 2 {
        return None;
    }
    let class = args[0];
    let mut strict = false;
    let mut i = 1;
    while i < args.len() && args[i].starts_with('-') {
        match args[i] {
            "-strict" => {
                strict = true;
                i += 1;
            }
            // `-failindex var` writes a variable (never folded), and an
            // unknown option / a `-`-leading string arg is ambiguous —
            // bail in every case.
            _ => return None,
        }
    }
    // Exactly one positional must remain: the string under test.
    if i + 1 != args.len() {
        return None;
    }
    let s = args[i];

    // The empty string is a member of every class in non-strict mode and
    // a member of none in strict mode.
    if s.is_empty() {
        return Some(if strict { "0" } else { "1" }.to_owned());
    }

    let member = match class {
        "alpha" => ascii_all(s, |c| c.is_ascii_alphabetic())?,
        "alnum" => ascii_all(s, |c| c.is_ascii_alphanumeric())?,
        "digit" => ascii_all(s, |c| c.is_ascii_digit())?,
        "lower" => ascii_all(s, |c| c.is_ascii_lowercase())?,
        "upper" => ascii_all(s, |c| c.is_ascii_uppercase())?,
        "xdigit" => ascii_all(s, |c| c.is_ascii_hexdigit())?,
        "space" => ascii_all(s, is_tcl_space)?,
        "control" => ascii_all(s, |c| c.is_ascii_control())?,
        "graph" => ascii_all(s, |c| c.is_ascii_graphic())?,
        "print" => ascii_all(s, |c| c.is_ascii_graphic() || c == ' ')?,
        "punct" => ascii_all(s, |c| c.is_ascii_punctuation())?,
        "wordchar" => ascii_all(s, |c| c.is_ascii_alphanumeric() || c == '_')?,
        // `ascii` is the test itself — defined for any input.
        "ascii" => s.is_ascii(),
        "boolean" => tcl_bool(s).is_some(),
        "true" => tcl_bool(s) == Some(true),
        "false" => tcl_bool(s) == Some(false),
        // Number / list / dict classes — deferred (need differential
        // pinning against tclsh9); leave the call unfolded.
        _ => return None,
    };
    Some(if member { "1" } else { "0" }.to_owned())
}

/// Apply an ASCII char predicate to every char of `s`, bailing (`None`)
/// on any non-ASCII char — Unicode class membership isn't modelled here.
fn ascii_all(s: &str, pred: impl Fn(char) -> bool) -> Option<bool> {
    if !s.is_ascii() {
        return None;
    }
    Some(s.chars().all(pred))
}

/// Tcl's whitespace set for `string is space` (`Tcl_UniCharIsSpace` on
/// ASCII): space, tab, newline, CR, vertical tab, form feed.  (Rust's
/// `char::is_ascii_whitespace` omits the vertical tab.)
fn is_tcl_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0b}' | '\u{0c}')
}

/// `Tcl_GetBoolean`: `0` / `1` plus the case-insensitive *unique*
/// prefixes of `true` / `false` / `yes` / `no` / `on` / `off`.  Returns
/// the boolean value, or `None` when the string is not a valid boolean.
/// (`o` is ambiguous between `on` and `off`, so it is *not* a boolean.)
fn tcl_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "t" | "tr" | "tru" | "true" | "y" | "ye" | "yes" | "on" => Some(true),
        "0" | "f" | "fa" | "fal" | "fals" | "false" | "n" | "no" | "of" | "off" => Some(false),
        _ => None,
    }
}

/// Character classes accepted by `string is <class>`.  Mirrors
/// `_IS_CLASSES` in `core/commands/registry/tcl/string.py`.
static IS_CLASSES: &[ArgValue] = &[
    ArgValue {
        value: "alnum",
        detail: "Any Unicode alphabet or digit character.",
    },
    ArgValue {
        value: "alpha",
        detail: "Any Unicode alphabet character.",
    },
    ArgValue {
        value: "ascii",
        detail: "Any character with a value less than U+0080 (7-bit ASCII).",
    },
    ArgValue {
        value: "boolean",
        detail: "Any valid boolean value (true/false/yes/no/on/off/0/1).",
    },
    ArgValue {
        value: "control",
        detail: "Any Unicode control character.",
    },
    ArgValue {
        value: "dict",
        detail: "Any proper dict structure, with optional surrounding whitespace.",
    },
    ArgValue {
        value: "digit",
        detail: "Any Unicode digit character.",
    },
    ArgValue {
        value: "double",
        detail: "Any valid floating-point number.",
    },
    ArgValue {
        value: "entier",
        detail: "Synonym for integer.",
    },
    ArgValue {
        value: "false",
        detail: "Any valid boolean false value.",
    },
    ArgValue {
        value: "graph",
        detail: "Any Unicode printing character, except space.",
    },
    ArgValue {
        value: "integer",
        detail: "Any valid integer of arbitrary size.",
    },
    ArgValue {
        value: "list",
        detail: "Any proper list structure, with optional surrounding whitespace.",
    },
    ArgValue {
        value: "lower",
        detail: "Any Unicode lower case alphabet character.",
    },
    ArgValue {
        value: "print",
        detail: "Any Unicode printing character, including space.",
    },
    ArgValue {
        value: "punct",
        detail: "Any Unicode punctuation character.",
    },
    ArgValue {
        value: "space",
        detail: "Any Unicode whitespace character.",
    },
    ArgValue {
        value: "true",
        detail: "Any valid boolean true value.",
    },
    ArgValue {
        value: "upper",
        detail: "Any upper case alphabet character.",
    },
    ArgValue {
        value: "wideinteger",
        detail: "Any valid wide integer.",
    },
    ArgValue {
        value: "wordchar",
        detail: "Any Unicode word character (alphanumeric + connector punctuation).",
    },
    ArgValue {
        value: "xdigit",
        detail: "Any hexadecimal digit character (0-9, A-F, a-f).",
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "bytelength",
        arity: Arity::exact(1),
        detail: "Return number of bytes used to represent the string in memory.",
        synopsis: "string bytelength string",
        pure: true,
        return_type: Some(TclType::Int),
        dialects: Some(
            DialectSet::TCL84
                .union(DialectSet::TCL85)
                .union(DialectSet::TCL86),
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cat",
        arity: Arity::any(),
        detail: "Concatenate strings.",
        synopsis: "string cat ?string1? ?string2 ...?",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_cat),
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "compare",
        arity: Arity::at_least(2),
        detail: "Compare two strings lexicographically.",
        synopsis: "string compare ?-nocase? ?-length length? string1 string2",
        pure: true,
        return_type: Some(TclType::Int),
        const_fold: Some(fold_compare),
        options: &[
            OptionSpec {
                name: "-nocase",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-length",
                takes_value: true,
                value_hint: "int",
                detail: "",
                dialects: None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "equal",
        arity: Arity::at_least(2),
        detail: "Test string equality.",
        synopsis: "string equal ?-nocase? ?-length length? string1 string2",
        pure: true,
        return_type: Some(TclType::Boolean),
        const_fold: Some(fold_equal),
        options: &[
            OptionSpec {
                name: "-nocase",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-length",
                takes_value: true,
                value_hint: "int",
                detail: "",
                dialects: None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "first",
        arity: Arity::new(2, 3),
        detail: "Find first occurrence of needle in haystack.",
        synopsis: "string first needleString haystackString ?startIndex?",
        pure: true,
        return_type: Some(TclType::Int),
        const_fold: Some(fold_first),
        arg_types: &[(
            2,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(2),
        detail: "Return character at index.",
        synopsis: "string index string charIndex",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_index),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::exact(3),
        detail: "Insert string at index.",
        synopsis: "string insert string index insertString",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "is",
        arity: Arity::at_least(2),
        detail: "Test if string is a member of a character class.",
        synopsis: "string is class ?-strict? ?-failindex varname? string",
        pure: true,
        return_type: Some(TclType::Boolean),
        const_fold: Some(fold_is),
        options: &[
            OptionSpec {
                name: "-strict",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-failindex",
                takes_value: true,
                value_hint: "varname",
                detail: "",
                dialects: None,
            },
        ],
        // First sub-arg (index 0 after `is`) is the character
        // class — complete it from the fixed class set.
        arg_values: &[(0, IS_CLASSES)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "last",
        arity: Arity::new(2, 3),
        detail: "Find last occurrence of needle in haystack.",
        synopsis: "string last needleString haystackString ?lastIndex?",
        pure: true,
        return_type: Some(TclType::Int),
        const_fold: Some(fold_last),
        arg_types: &[(
            2,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "length",
        arity: Arity::exact(1),
        detail: "Return number of characters.",
        synopsis: "string length string",
        pure: true,
        return_type: Some(TclType::Int),
        const_fold: Some(fold_length),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "map",
        arity: Arity::at_least(2),
        detail: "Map substrings via key-value pairs.",
        synopsis: "string map ?-nocase? mapping string",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_string_map),
        options: &[OptionSpec {
            name: "-nocase",
            takes_value: false,
            value_hint: "",
            detail: "",
            dialects: None,
        }],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "match",
        arity: Arity::at_least(2),
        detail: "Test glob-style pattern match.",
        synopsis: "string match ?-nocase? pattern string",
        pure: true,
        return_type: Some(TclType::Boolean),
        options: &[OptionSpec {
            name: "-nocase",
            takes_value: false,
            value_hint: "",
            detail: "",
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "range",
        arity: Arity::exact(3),
        detail: "Return substring by index range.",
        synopsis: "string range string first last",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_range),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "repeat",
        arity: Arity::exact(2),
        detail: "Repeat string N times.",
        synopsis: "string repeat string count",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_repeat),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::new(3, 4),
        detail: "Replace range with new string.",
        synopsis: "string replace string first last ?newString?",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_replace),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "reverse",
        arity: Arity::exact(1),
        detail: "Reverse character order.",
        synopsis: "string reverse string",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_reverse),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tolower",
        arity: Arity::new(1, 3),
        detail: "Convert to lower case.",
        synopsis: "string tolower string ?first? ?last?",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_tolower),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "totitle",
        arity: Arity::new(1, 3),
        detail: "Convert to title case.",
        synopsis: "string totitle string ?first? ?last?",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_totitle),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "toupper",
        arity: Arity::new(1, 3),
        detail: "Convert to upper case.",
        synopsis: "string toupper string ?first? ?last?",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_toupper),
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trim",
        arity: Arity::new(1, 2),
        detail: "Trim leading and trailing characters.",
        synopsis: "string trim string ?chars?",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_trim),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trimleft",
        arity: Arity::new(1, 2),
        detail: "Trim leading characters.",
        synopsis: "string trimleft string ?chars?",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_trimleft),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trimright",
        arity: Arity::new(1, 2),
        detail: "Trim trailing characters.",
        synopsis: "string trimright string ?chars?",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_trimright),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "wordend",
        arity: Arity::exact(2),
        detail: "Index of character after end of word.",
        synopsis: "string wordend string charIndex",
        pure: true,
        return_type: Some(TclType::Int),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "wordstart",
        arity: Arity::exact(2),
        detail: "Index of first character of word.",
        synopsis: "string wordstart string charIndex",
        pure: true,
        return_type: Some(TclType::Int),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `string`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "string",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet::brief(
            "Perform one of several string operations.",
            &["string option arg ?arg ...?"],
            "Tcl string(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::fold_is;
    use crate::CommandRegistry;

    #[test]
    fn string_is_folds_tcl_faithful_classes() {
        // Helper: `string is <args...>`.
        let is = |args: &[&str]| fold_is(args);

        // ASCII char classes — all-chars-in-class.
        assert_eq!(is(&["alpha", "abc"]).as_deref(), Some("1"));
        assert_eq!(is(&["alpha", "abc1"]).as_deref(), Some("0"));
        assert_eq!(is(&["alnum", "abc1"]).as_deref(), Some("1"));
        assert_eq!(is(&["digit", "123"]).as_deref(), Some("1"));
        assert_eq!(is(&["digit", "12a"]).as_deref(), Some("0"));
        assert_eq!(is(&["xdigit", "1aF"]).as_deref(), Some("1"));
        assert_eq!(is(&["space", "  \t"]).as_deref(), Some("1"));
        assert_eq!(is(&["wordchar", "a_b9"]).as_deref(), Some("1"));
        assert_eq!(is(&["wordchar", "a-b"]).as_deref(), Some("0"));

        // The Tcl-vs-Python divergence the deferral called out:
        // `str.islower("abc1")` is True, but Tcl `string is lower` is 0
        // (a digit is not a lowercase *letter*).
        assert_eq!(is(&["lower", "abc1"]).as_deref(), Some("0"));
        assert_eq!(is(&["lower", "abc"]).as_deref(), Some("1"));
        assert_eq!(is(&["upper", "ABC"]).as_deref(), Some("1"));
        assert_eq!(is(&["upper", "ABc"]).as_deref(), Some("0"));

        // `ascii` is the membership test itself (defined for any input).
        assert_eq!(is(&["ascii", "abc"]).as_deref(), Some("1"));
        assert_eq!(is(&["ascii", "caf\u{e9}"]).as_deref(), Some("0"));
        // Other char classes bail on non-ASCII (membership not modelled).
        assert_eq!(is(&["alpha", "caf\u{e9}"]), None);

        // Boolean keyword + unique-prefix set.
        assert_eq!(is(&["boolean", "yes"]).as_deref(), Some("1"));
        assert_eq!(is(&["boolean", "t"]).as_deref(), Some("1"));
        assert_eq!(is(&["boolean", "of"]).as_deref(), Some("1"));
        assert_eq!(
            is(&["boolean", "o"]).as_deref(),
            Some("0"),
            "ambiguous on/off"
        );
        assert_eq!(is(&["boolean", "maybe"]).as_deref(), Some("0"));
        assert_eq!(is(&["true", "TRUE"]).as_deref(), Some("1"));
        assert_eq!(is(&["true", "no"]).as_deref(), Some("0"));
        assert_eq!(is(&["false", "off"]).as_deref(), Some("1"));

        // Empty string: non-strict member of every class, strict member
        // of none.
        assert_eq!(is(&["alpha", ""]).as_deref(), Some("1"));
        assert_eq!(is(&["alpha", "-strict", ""]).as_deref(), Some("0"));
        assert_eq!(
            is(&["integer", ""]).as_deref(),
            Some("1"),
            "even a deferred class"
        );

        // `-failindex` writes a var → never folds; deferred classes and
        // unknown options bail.
        assert_eq!(is(&["alpha", "-failindex", "v", "abc"]), None);
        assert_eq!(is(&["integer", "42"]), None, "number classes deferred");
        assert_eq!(is(&["double", "1.5"]), None, "double deferred");
        assert_eq!(is(&["nonclass", "x"]), None, "unknown class bails");
    }

    #[test]
    fn string_is_subcommand_carries_const_fold() {
        let f = CommandRegistry::build_default()
            .get("string")
            .and_then(|s| s.subcommand("is"))
            .and_then(|s| s.const_fold)
            .expect("string is const_fold");
        assert_eq!(f(&["alpha", "abc"]).as_deref(), Some("1"));
    }

    #[test]
    fn pure_string_subcommands_carry_const_fold() {
        // SYNC-JUN02b-6 (#519): toupper / tolower / reverse expose a
        // const_fold callback for the optimiser's O129 path.
        let reg = CommandRegistry::build_default();
        let spec = reg.get("string").expect("string spec");

        let toupper = spec
            .subcommand("toupper")
            .and_then(|s| s.const_fold)
            .expect("toupper const_fold");
        assert_eq!(toupper(&["foo"]).as_deref(), Some("FOO"));
        // The range form (`string toupper s first last`) must not fold.
        assert_eq!(toupper(&["foo", "0", "0"]), None);
        // Non-ASCII bails (conservative — Rust/Tcl case maps diverge).
        assert_eq!(toupper(&["caf\u{e9}"]), None);

        let tolower = spec
            .subcommand("tolower")
            .and_then(|s| s.const_fold)
            .expect("tolower const_fold");
        assert_eq!(tolower(&["FOO"]).as_deref(), Some("foo"));

        let reverse = spec
            .subcommand("reverse")
            .and_then(|s| s.const_fold)
            .expect("reverse const_fold");
        assert_eq!(reverse(&["abc"]).as_deref(), Some("cba"));

        let length = spec
            .subcommand("length")
            .and_then(|s| s.const_fold)
            .expect("length const_fold");
        assert_eq!(length(&["abcde"]).as_deref(), Some("5"));
        assert_eq!(length(&[""]).as_deref(), Some("0"));
        assert_eq!(length(&["caf\u{e9}"]), None, "non-ASCII bails");
    }

    #[test]
    fn string_value_folds_match_tcl() {
        // SYNC-JUN02d-1 (#525): the cat / repeat / trim* / totitle folds.
        let reg = CommandRegistry::build_default();
        let spec = reg.get("string").expect("string spec");
        let f = |sub: &str| spec.subcommand(sub).and_then(|s| s.const_fold).unwrap();

        assert_eq!(f("cat")(&["a", "b", "c"]).as_deref(), Some("abc"));
        assert_eq!(f("cat")(&[]).as_deref(), Some(""));
        assert_eq!(f("repeat")(&["ab", "3"]).as_deref(), Some("ababab"));
        assert_eq!(f("repeat")(&["x", "0"]).as_deref(), Some(""));
        assert_eq!(f("repeat")(&["x", "-1"]), None, "negative count bails");
        assert_eq!(f("repeat")(&["x", "999999"]), None, "over-cap bails");
        assert_eq!(f("trim")(&["  hi  "]).as_deref(), Some("hi"));
        assert_eq!(f("trim")(&["xxhixx", "x"]).as_deref(), Some("hi"));
        assert_eq!(f("trimleft")(&["  hi  "]).as_deref(), Some("hi  "));
        assert_eq!(f("trimright")(&["  hi  "]).as_deref(), Some("  hi"));
        assert_eq!(f("totitle")(&["hELLO"]).as_deref(), Some("Hello"));
        assert_eq!(f("totitle")(&[""]).as_deref(), Some(""));
        assert_eq!(f("totitle")(&["caf\u{e9}"]), None, "non-ASCII bails");
    }

    #[test]
    fn string_index_comparison_folds_match_tcl() {
        // SYNC-JUN02d-1 (#525): index / range / replace / first / last /
        // compare / equal, with `end` / `end-N` index parsing.
        let reg = CommandRegistry::build_default();
        let spec = reg.get("string").expect("string spec");
        let f = |sub: &str| spec.subcommand(sub).and_then(|s| s.const_fold).unwrap();

        assert_eq!(f("index")(&["abc", "1"]).as_deref(), Some("b"));
        assert_eq!(f("index")(&["abc", "end"]).as_deref(), Some("c"));
        assert_eq!(
            f("index")(&["abc", "9"]).as_deref(),
            Some(""),
            "OOB → empty"
        );
        assert_eq!(f("range")(&["abcde", "1", "3"]).as_deref(), Some("bcd"));
        assert_eq!(f("range")(&["abcde", "2", "end"]).as_deref(), Some("cde"));
        assert_eq!(f("range")(&["abcde", "3", "1"]).as_deref(), Some(""));
        assert_eq!(f("replace")(&["abcde", "1", "3"]).as_deref(), Some("ae"));
        assert_eq!(
            f("replace")(&["abcde", "1", "3", "XY"]).as_deref(),
            Some("aXYe")
        );
        assert_eq!(f("first")(&["b", "abcb"]).as_deref(), Some("1"));
        assert_eq!(f("first")(&["b", "abcb", "2"]).as_deref(), Some("3"));
        assert_eq!(f("first")(&["z", "abc"]).as_deref(), Some("-1"));
        assert_eq!(f("last")(&["b", "abcb"]).as_deref(), Some("3"));
        assert_eq!(f("last")(&["z", "abc"]).as_deref(), Some("-1"));
        assert_eq!(f("compare")(&["abc", "abc"]).as_deref(), Some("0"));
        assert_eq!(f("compare")(&["abc", "abd"]).as_deref(), Some("-1"));
        assert_eq!(f("compare")(&["abd", "abc"]).as_deref(), Some("1"));
        assert_eq!(
            f("compare")(&["-nocase", "ABC", "abc"]).as_deref(),
            Some("0")
        );
        assert_eq!(
            f("compare")(&["-length", "2", "abX", "abY"]).as_deref(),
            Some("0")
        );
        assert_eq!(f("equal")(&["abc", "abc"]).as_deref(), Some("1"));
        assert_eq!(f("equal")(&["abc", "abd"]).as_deref(), Some("0"));
    }

    #[test]
    fn string_map_folds_match_tcl() {
        // SYNC-JUN02d-1 (#525): `string map` greedy left-to-right replace.
        let reg = CommandRegistry::build_default();
        let m = reg
            .get("string")
            .and_then(|s| s.subcommand("map"))
            .and_then(|s| s.const_fold)
            .expect("map const_fold");
        assert_eq!(m(&["a b", "aaa"]).as_deref(), Some("bbb"));
        assert_eq!(m(&["ab AB", "xabx"]).as_deref(), Some("xABx"));
        assert_eq!(m(&["-nocase", "abc X", "ABCdef"]).as_deref(), Some("Xdef"));
        // Odd mapping → no fold; non-ASCII → no fold.
        assert_eq!(m(&["a b c", "x"]), None);
        assert_eq!(m(&["a b", "caf\u{e9}"]), None);
    }
}
