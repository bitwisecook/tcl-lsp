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

//! `string` — perform one of several string operations.

use crate::prelude::*;
use tcl_syntax::number::{Number, parse_whole};

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "string option arg ?arg ...?",
}];

/// Compile-time folds for pure `string`
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
///
/// The registry stores const-folds as `ConstFoldFn`
/// (`fn(&[&str]) -> Option<String>`), so this signature is fixed by the
/// dispatch table even though concatenation never fails. `unnecessary_wraps`
/// is a false positive here: the `Option` is the shared callback contract,
/// not redundant wrapping we control.
#[allow(clippy::unnecessary_wraps)] // signature fixed by ConstFoldFn dispatch contract
fn fold_cat(args: &[&str]) -> Option<String> {
    Some(args.concat())
}

/// Cap on a single constant-fold's materialised output (1 MiB). `s` may
/// already be large from a nested fold, so bounding only the repeat count
/// leaves an allocation bomb; bound the product and bail above the cap.
const MAX_FOLD_OUTPUT_BYTES: usize = 1 << 20;

/// `string repeat string count` — repeat (bounded by a 10000 count cap and a
/// 1 MiB output cap).  No char transformation → sound for any input.
/// A negative count fails the `usize` parse → bails.
fn fold_repeat(args: &[&str]) -> Option<String> {
    let [s, count] = args else {
        return None;
    };
    let n: usize = count.trim().parse().ok()?;
    if n > 10_000 {
        return None;
    }
    if s.len()
        .checked_mul(n)
        .is_none_or(|bytes| bytes > MAX_FOLD_OUTPUT_BYTES)
    {
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
    if needle.is_empty() {
        return Some("-1".to_owned());
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
    if needle.is_empty() {
        return Some("-1".to_owned());
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

/// `string is class ?-strict? ?-failindex var? string`.
/// Constant-folds the **Tcl-faithful** classes
/// the optimiser can decide soundly.
///
/// Deliberately follows Tcl's classification, not a naive
/// letter-case predicate whose semantics diverge: e.g. a check that
/// passes `"abc1"` as "lower" disagrees with `string is lower abc1`,
/// which is `0` (Tcl requires *every* char to be a lowercase letter;
/// a digit fails).  The ASCII character
/// classes here apply the predicate to every char and **bail on
/// non-ASCII input** (Unicode class membership isn't modelled — a
/// missed fold, never a wrong one).  `ascii` is the membership test
/// itself, so it is defined for any input.  `boolean` / `true` /
/// `false` use the exact `Tcl_GetBoolean` keyword + unique-prefix set
/// (so `t` / `ye` / `of` resolve, `o` — ambiguous between on/off —
/// does not).  `list` folds (a backslash-free string's well-formedness is
/// unambiguous).  `integer` and `double` fold over their dialect-invariant
/// subsets (see [`string_is_integer`] / [`string_is_double`]); `entier` /
/// `wideinteger` / `dict` stay deferred — they *raise* in old dialects
/// (8.4 / 8.4 + 8.5 / pre-9.0 respectively), so no dialect-agnostic fold can
/// be sound (it would turn an error into a value).
fn fold_is(args: &[&str], version: Option<TclVersion>) -> Option<String> {
    if args.len() < 2 {
        return None;
    }
    let class = args[0];
    // Tcl grammar: `string is class ?-strict? ?-failindex var? str` — the
    // **last** argument is always the value under test; the args between the
    // class and it are options.  Verified against tclsh 8.4 → 9.0:
    // `string is integer -7` reads `-7` as the *value* (→ 1), and a lone
    // `string is integer -strict` reads `-strict` as the value (→ 0), not an
    // option.  Taking the last arg as the value (rather than treating every
    // `-`-leading arg as an option, as a naive scan would) lets a negative
    // value fold and matches real Tcl.
    let s = *args.last()?;
    let mut strict = false;
    for opt in &args[1..args.len() - 1] {
        match *opt {
            "-strict" => strict = true,
            // `-failindex var` writes a variable (never folded); an unknown
            // option is ambiguous — bail in both cases.
            _ => return None,
        }
    }

    // A class that *raises* in this dialect can't fold to any value — not even
    // the empty-string shortcut below (`string is dict ""` raises before 9.0).
    if !class_available(class, version) {
        return None;
    }

    // The empty string is a member of every available class in non-strict mode
    // and a member of none in strict mode.
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
        // `list`: well-formed Tcl list.  `split_list` returns `Some`
        // only for a valid (backslash-free) list and `None` for a
        // malformed one — but it conservatively rejects *any* backslash,
        // and a backslash-bearing string can still be a valid list
        // (`a\ b`), so a `None` there is ambiguous.  Sound resolution:
        // a backslash-free string folds (`Some` → 1, `None` → 0); a
        // backslash-bearing one bails.
        "list" => {
            if s.contains('\\') {
                return None;
            }
            crate::const_fold::split_list(s).is_some()
        }
        // Version-aware number classes (availability gated by
        // `class_available` above); each decides 1/0/bail for `version`.
        "integer" => return is_integer_class(s, version),
        "wideinteger" => return is_wide_class(s, version),
        "entier" => return is_entier_class(s, version),
        "dict" => return is_dict_class(s),
        "double" => return string_is_double(s),
        _ => return None, // unknown class
    };
    Some(if member { "1" } else { "0" }.to_owned())
}

/// Whether `string is class` is a *defined* operation in the target version
/// (it doesn't *raise*).  `wideinteger` requires 8.5+, `entier` 8.6+, and
/// `dict` is **9.0-only** (matching the gating below); an unknown version
/// (`None`) treats all three as unavailable, since one of the dialects it
/// could stand for raises.  Every other class exists in every release.
fn class_available(class: &str, version: Option<TclVersion>) -> bool {
    match class {
        "wideinteger" => version.is_some_and(|v| v >= TclVersion::V8_5),
        "entier" => version.is_some_and(|v| v >= TclVersion::V8_6),
        "dict" => version >= Some(TclVersion::V9_0),
        _ => true,
    }
}

/// The magnitude bounds the `string is` integer classes use across releases.
const U32_MAX: u128 = 4_294_967_295;
const I64_MAX: u128 = 9_223_372_036_854_775_807;
const U64_MAX: u128 = 18_446_744_073_709_551_615;

/// Classification of a `string is <intclass>` value as a plain decimal.
enum IntForm {
    /// Definitely not an integer in any form (→ `"0"` for an available class).
    NotInteger,
    /// A leading-zero / `0x` / `0o` / `0b` / non-ASCII form whose validity or
    /// magnitude is version-dependent — bail.
    Ambiguous,
    /// A plain decimal: `mag` is its absolute value, or `None` when it exceeds
    /// `u128` (astronomically large).
    Decimal { neg: bool, mag: Option<u128> },
}

/// Classify `s` as a plain decimal integer.  Restricting to plain decimals
/// (no leading zero, no `0x`/`0o`/`0b`) makes the *form* validity
/// version-independent — only the magnitude cap varies — so the per-class
/// handlers compare `mag` against their version's bound.
fn classify_int_form(s: &str) -> IntForm {
    if !s.is_ascii() {
        return IntForm::Ambiguous;
    }
    let t = s.trim();
    if t.is_empty() {
        return IntForm::NotInteger; // whitespace-only
    }
    let (neg, body) = if let Some(rest) = t.strip_prefix('-') {
        (true, rest)
    } else {
        (false, t.strip_prefix('+').unwrap_or(t))
    };
    let b = body.as_bytes();
    if b.is_empty() {
        return IntForm::NotInteger; // a lone sign
    }
    if b.contains(&b'_') {
        return IntForm::Ambiguous; // Tcl 9 digit separators
    }
    if b[0] == b'0'
        && b.len() >= 2
        && matches!(b[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B' | b'd' | b'D')
    {
        return IntForm::Ambiguous; // Tcl 9 radix prefixes / Tcl 8.x divergence
    }
    if !b.iter().all(u8::is_ascii_digit) {
        return IntForm::NotInteger; // a non-digit with no recognised prefix
    }
    if b.len() > 1 && b[0] == b'0' {
        return IntForm::Ambiguous; // leading zero → octal interpretation in 8.x
    }
    IntForm::Decimal {
        neg,
        mag: body.parse::<u128>().ok(),
    }
}

fn tcl9_integer_number(s: &str) -> Option<Number> {
    if !s.is_ascii() {
        return None;
    }
    parse_whole(s)
}

/// `string is integer`: known Tcl 9 uses the shared Tcl 9 number parser;
/// older/unknown dialects stay on the conservative plain-decimal subset.
/// A plain decimal of magnitude ≤ `2³²-1` is valid in every release
/// (→ `1`); larger or Tcl-9-only forms bail unless the dialect is known.
fn is_integer_class(s: &str, version: Option<TclVersion>) -> Option<String> {
    if version >= Some(TclVersion::V9_0) {
        return Some(
            if matches!(
                tcl9_integer_number(s),
                Some(Number::Int(_) | Number::Big { .. })
            ) {
                "1"
            } else {
                "0"
            }
            .to_owned(),
        );
    }
    match classify_int_form(s) {
        IntForm::Ambiguous => None,
        IntForm::NotInteger => Some("0".to_owned()),
        IntForm::Decimal { mag, .. } => {
            let within_32 = mag.is_some_and(|m| m <= U32_MAX);
            match version {
                Some(TclVersion::V9_0 | TclVersion::V9_1) => Some("1".to_owned()),
                Some(_) => Some(if within_32 { "1" } else { "0" }.to_owned()),
                None => within_32.then(|| "1".to_owned()),
            }
        }
    }
}

/// `string is wideinteger` (available 8.5+, gated by [`class_available`]):
/// known Tcl 9 uses the shared Tcl 9 number parser and the signed-64-bit
/// bound. 8.5/8.6 accept the unsigned-64-bit range; negative values bail
/// there because that signed/unsigned boundary differs by dialect.
fn is_wide_class(s: &str, version: Option<TclVersion>) -> Option<String> {
    if version >= Some(TclVersion::V9_0) {
        return Some(
            if matches!(tcl9_integer_number(s), Some(Number::Int(_))) {
                "1"
            } else {
                "0"
            }
            .to_owned(),
        );
    }
    match classify_int_form(s) {
        IntForm::Ambiguous => None,
        IntForm::NotInteger => Some("0".to_owned()),
        IntForm::Decimal { neg, mag } => {
            if neg {
                return None;
            }
            let cap = if version >= Some(TclVersion::V9_0) {
                I64_MAX
            } else {
                U64_MAX
            };
            Some(
                if mag.is_some_and(|m| m <= cap) {
                    "1"
                } else {
                    "0"
                }
                .to_owned(),
            )
        }
    }
}

/// `string is entier` (available 8.6+, gated by [`class_available`]):
/// known Tcl 9 uses the shared Tcl 9 number parser. Older known dialects
/// accept the conservative plain-decimal subset at arbitrary precision.
fn is_entier_class(s: &str, version: Option<TclVersion>) -> Option<String> {
    if version >= Some(TclVersion::V9_0) {
        return Some(
            if matches!(
                tcl9_integer_number(s),
                Some(Number::Int(_) | Number::Big { .. })
            ) {
                "1"
            } else {
                "0"
            }
            .to_owned(),
        );
    }
    match classify_int_form(s) {
        IntForm::Ambiguous => None,
        IntForm::NotInteger => Some("0".to_owned()),
        IntForm::Decimal { .. } => Some("1".to_owned()),
    }
}

/// `string is dict` (9.0 only, gated by [`class_available`]): an even-length
/// well-formed list.  A backslash bails (the same ambiguity as `list`).
fn is_dict_class(s: &str) -> Option<String> {
    if s.contains('\\') {
        return None;
    }
    Some(
        match crate::const_fold::split_list(s) {
            Some(elems) if elems.len() % 2 == 0 => "1",
            _ => "0", // odd length, or malformed → not a dict
        }
        .to_owned(),
    )
}

/// `string is double` over the **dialect-invariant** subset.  `s` is the
/// non-empty value under test (the empty string is handled by [`fold_is`]).
///
/// `string is double` is remarkably consistent across Tcl 8.4 → 9.0, and on
/// the decimal / scientific / `Inf` / `NaN` forms Rust's `f64::from_str`
/// (after trimming) agrees with it **exactly** — verified over a broad matrix
/// against `tclsh8.4`/`8.5`/`8.6`/`9.0` during development, e.g. `5.e3`, `.5`,
/// `1.`, `+.5`, `1E-10`, `inf`, `nan` → `1`; `1e`, `e5`, `.`, `1.2.3` → `0`.
/// The few forms where they part ways are version-divergent anyway, so they
/// **bail**:
///
/// * a `_` digit separator — accepted only by Tcl 9 (`1_000` → `0` on 8.x,
///   `1` on 9.0), rejected by Rust;
/// * a `0x` / `0o` / `0b` prefix — `0x…` is a valid double in every version
///   but Rust's parser rejects it, `0o…` / `0b…` raise in 8.4, and a
///   hex-float (`0x1.8p3`) flips 8.4 (`1`) ↔ 8.5+ (`0`);
/// * a `(` — the `nan(payload)` form, which Tcl accepts but Rust rejects;
/// * a non-ASCII value (so the `.trim()` whitespace set matches Tcl's).
fn string_is_double(s: &str) -> Option<String> {
    if !s.is_ascii() {
        return None;
    }
    let t = s.trim();
    if t.contains('_') || t.contains('(') {
        return None;
    }
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let b = body.as_bytes();
    if b.len() >= 2 && b[0] == b'0' && matches!(b[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B') {
        return None;
    }
    // For every remaining ASCII form, Rust's `f64` parser and Tcl's
    // `string is double` agree across 8.4 → 9.0.
    Some(if t.parse::<f64>().is_ok() { "1" } else { "0" }.to_owned())
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

/// Character classes accepted by `string is <class>`.
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
        byte_array_effect: ByteArrayEffect::Coerces,
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
        options: const {
            &[
                OptionSpec {
                    name: "-nocase",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-length",
                    value: OptionValue::value("int"),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
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
        options: const {
            &[
                OptionSpec {
                    name: "-nocase",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-length",
                    value: OptionValue::value("int"),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
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
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        byte_array_effect: ByteArrayEffect::Transparent,
        arity: Arity::exact(2),
        detail: "Return character at index.",
        synopsis: "string index string charIndex",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_index),
        arg_types: &[
            (
                0,
                // The subject installs the `tclStringType` intrep
                // (`SetStringFromAny` via the char-indexing path), replacing
                // a List/Dict/Int/… rep — tclsh-verified (a list becomes
                // `string`). A pure byte array short-circuits before the
                // conversion and keeps its rep.
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::exact(3),
        detail: "Insert string at index.",
        synopsis: "string insert string index insertString",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90_PLUS),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
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
        const_fold_versioned: Some(fold_is),
        options: const {
            &[
                OptionSpec {
                    name: "-strict",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-failindex",
                    value: OptionValue::value("varname"),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        // First sub-arg (index 0 after `is`) is the character
        // class — complete it from the fixed class set, which is
        // exhaustive (a non-member is a runtime `bad class` error → W127).
        // C Tcl accepts a unique prefix (`boo` → `boolean`).
        arg_values: &[(0, IS_CLASSES)],
        closed_value_args: &[0],
        arg_values_accept_prefix: true,
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
                transparent_from: &[],
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
        // `Tcl_GetCharLength` installs the `tclStringType` intrep
        // (`SetStringFromAny`), replacing a List/Dict/Int/… rep —
        // tclsh-verified (`set i 5; incr i; string length $i` leaves `i` a
        // `string`). A pure byte array short-circuits to its byte count and
        // keeps its rep, so it is transparent here even though the
        // subcommand is no byte-array value-transform
        // (`byte_array_effect: None` — the result is a count, not bytes).
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
                transparent_from: &[TclType::ByteArray],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "map",
        byte_array_effect: ByteArrayEffect::Coerces,
        arity: Arity::at_least(2),
        detail: "Map substrings via key-value pairs.",
        synopsis: "string map ?-nocase? mapping string",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_string_map),
        options: const {
            &[OptionSpec {
                name: "-nocase",
                value: OptionValue::flag(),
                detail: "",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Dict),
                shimmers: true,
                transparent_from: &[],
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
        options: const {
            &[OptionSpec {
                name: "-nocase",
                value: OptionValue::flag(),
                detail: "",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "range",
        byte_array_effect: ByteArrayEffect::Transparent,
        arity: Arity::exact(3),
        detail: "Return substring by index range.",
        synopsis: "string range string first last",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_range),
        arg_types: &[
            (
                0,
                // Subject: installs the string intrep except for the pure
                // byte-array fast path — see `string index` arg 0.
                ArgTypeHint {
                    expected: Some(TclType::String),
                    shimmers: true,
                    transparent_from: &[TclType::ByteArray],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "repeat",
        byte_array_effect: ByteArrayEffect::Coerces,
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
                transparent_from: &[],
            },
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        byte_array_effect: ByteArrayEffect::Coerces,
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
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "reverse",
        byte_array_effect: ByteArrayEffect::Transparent,
        arity: Arity::exact(1),
        detail: "Reverse character order.",
        synopsis: "string reverse string",
        pure: true,
        return_type: Some(TclType::String),
        const_fold: Some(fold_reverse),
        // Added in Tcl 8.5.
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tolower",
        byte_array_effect: ByteArrayEffect::CaseFolds,
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
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "totitle",
        byte_array_effect: ByteArrayEffect::CaseFolds,
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
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "toupper",
        byte_array_effect: ByteArrayEffect::CaseFolds,
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
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trim",
        byte_array_effect: ByteArrayEffect::Transparent,
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
        byte_array_effect: ByteArrayEffect::Transparent,
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
        byte_array_effect: ByteArrayEffect::Transparent,
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
                transparent_from: &[],
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
                transparent_from: &[],
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
        hover: Some(HoverSnippet {
            summary: "Perform one of several string operations.",
            synopsis: &["string option arg ?arg ...?"],
            snippet: "Use subcommands like `length`, `match`, `is`, `compare`, `map`, `range`, etc.",
            source: "Tcl man page string.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::fold_is;
    use crate::CommandRegistry;
    use crate::hooks::TclVersion;

    #[test]
    fn string_is_folds_tcl_faithful_classes() {
        // Helper: `string is <args...>`.
        // The char / boolean / list classes are version-invariant; pin them
        // under 9.0 (the number classes have their own version-aware tests).
        let is = |args: &[&str]| fold_is(args, Some(TclVersion::V9_0));

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

        // A divergence worth pinning: a naive "all lowercase" check would
        // pass `"abc1"`, but Tcl `string is lower` is 0
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

        // `list`: a well-formed list folds; a malformed one is `0`; a
        // backslash-bearing string bails (split_list rejects backslash,
        // but it can still be a valid list).
        assert_eq!(is(&["list", "a b c"]).as_deref(), Some("1"));
        assert_eq!(is(&["list", "{a b} c"]).as_deref(), Some("1"));
        assert_eq!(is(&["list", "a {b"]).as_deref(), Some("0"), "unbalanced");
        assert_eq!(is(&["list", "a\\ b"]), None, "backslash bails");

        // Empty string: non-strict member of every class, strict member
        // of none.
        assert_eq!(is(&["alpha", ""]).as_deref(), Some("1"));
        assert_eq!(is(&["alpha", "-strict", ""]).as_deref(), Some("0"));
        assert_eq!(
            is(&["integer", ""]).as_deref(),
            Some("1"),
            "even a deferred class"
        );

        // `-failindex` writes a var → never folds; an unknown class bails.
        assert_eq!(is(&["alpha", "-failindex", "v", "abc"]), None);
        assert_eq!(is(&["nonclass", "x"]), None, "unknown class bails");
        // Under 9.0 the formerly-deferred classes fold (their version-gating
        // and caps are covered by `string_is_number_classes_are_version_aware`).
        assert_eq!(is(&["wideinteger", "42"]).as_deref(), Some("1"));
        assert_eq!(is(&["entier", "42"]).as_deref(), Some("1"));
        assert_eq!(is(&["dict", "a 1"]).as_deref(), Some("1"));
    }

    #[test]
    fn string_is_double_folds_dialect_invariant_subset() {
        // `double` is version-invariant today; pin under 9.0.
        let is_dbl = |v: &str| fold_is(&["double", v], Some(TclVersion::V9_0));

        // Decimal / scientific / Inf / NaN forms fold to "1" — Rust's f64
        // parser agrees with Tcl 8.4 → 9.0 across this matrix.
        for v in [
            "3.14", "-2.5", "+.5", ".5", "5.", "5.e3", "42", "0", "1e10", "1E-10", "1e+5", "0.0",
            "00.5", "inf", "Inf", "infinity", "nan", "NaN", "-inf", " 1.5 ", "-3.14",
        ] {
            assert_eq!(is_dbl(v).as_deref(), Some("1"), "{v:?} is a double");
        }
        // Clear non-doubles fold to "0".
        for v in ["abc", "1e", "e5", "1.5e", ".", "1.2.3", "1d0", "1f", " "] {
            assert_eq!(is_dbl(v).as_deref(), Some("0"), "{v:?} is not a double");
        }
        // Version-divergent / Rust-mismatching forms bail.
        assert_eq!(is_dbl("1_000"), None, "Tcl 9 digit separator");
        assert_eq!(is_dbl("0x1f"), None, "hex valid in Tcl, rejected by Rust");
        assert_eq!(is_dbl("0o17"), None, "0o raises in 8.4");
        assert_eq!(is_dbl("0x1.8p3"), None, "hex-float flips 8.4 ↔ 8.5+");
        assert_eq!(
            is_dbl("nan(0)"),
            None,
            "nan payload: Tcl accepts, Rust rejects"
        );
        assert_eq!(is_dbl("caf\u{e9}"), None, "non-ASCII bails");
    }

    #[test]
    fn string_is_integer_folds_dialect_invariant_subset() {
        // `None` version → the dialect-invariant subset every release shares.
        let is_int = |v: &str| fold_is(&["integer", v], None);

        // Non-leading-zero decimals within the 32-bit-unsigned bound every
        // 8.4 → 9.0 shares fold to "1" (verified against all four tclsh).
        assert_eq!(is_int("42").as_deref(), Some("1"));
        assert_eq!(is_int("-7").as_deref(), Some("1"));
        assert_eq!(is_int("+5").as_deref(), Some("1"));
        assert_eq!(is_int("0").as_deref(), Some("1"));
        assert_eq!(
            is_int(" 42 ").as_deref(),
            Some("1"),
            "surrounding ws trimmed"
        );
        assert_eq!(is_int("2147483648").as_deref(), Some("1"), "past 2^31");
        assert_eq!(
            is_int("4294967295").as_deref(),
            Some("1"),
            "2^32-1 boundary"
        );
        assert_eq!(is_int("-4294967295").as_deref(), Some("1"));

        // Clear non-integers fold to "0" — no Tcl version reads them as one.
        assert_eq!(is_int("abc").as_deref(), Some("0"));
        assert_eq!(is_int("1.5").as_deref(), Some("0"));
        assert_eq!(is_int("1e5").as_deref(), Some("0"));
        assert_eq!(is_int("12x").as_deref(), Some("0"));
        assert_eq!(is_int("1 2").as_deref(), Some("0"), "internal space");
        assert_eq!(is_int(".5").as_deref(), Some("0"));
        assert_eq!(is_int("-").as_deref(), Some("0"), "lone sign");
        assert_eq!(is_int("0abc").as_deref(), Some("0"));
        assert_eq!(is_int(" ").as_deref(), Some("0"), "whitespace-only");

        // Divergence / ambiguity bails (a miss is never wrong).
        assert_eq!(
            is_int("4294967296"),
            None,
            "past the 32-bit cap (8.x reject)"
        );
        assert_eq!(is_int("9999999999999999999999"), None, "huge → diverges");
        assert_eq!(is_int("010"), None, "leading zero: octal in 8.x");
        assert_eq!(is_int("08"), None, "leading zero, invalid octal in 8.x");
        assert_eq!(is_int("0x1f"), None, "hex needs digit validation");
        assert_eq!(is_int("0o17"), None, "0o raises in 8.4");
        assert_eq!(is_int("0b101"), None, "0b raises in 8.4");
        assert_eq!(is_int("caf\u{e9}"), None, "non-ASCII bails");

        // Option grammar: the last arg is the value, the rest are options
        // (verified against tclsh 8.4 → 9.0).
        assert_eq!(
            fold_is(&["integer", "-strict", "42"], None).as_deref(),
            Some("1"),
            "-strict option, value 42"
        );
        assert_eq!(
            fold_is(&["integer", "-strict", "-7"], None).as_deref(),
            Some("1"),
            "-strict option, negative value"
        );
        assert_eq!(
            fold_is(&["integer", "-strict"], None).as_deref(),
            Some("0"),
            "lone `-strict` is the value, not an option"
        );
        assert_eq!(
            fold_is(&["integer", "-failindex", "v", "42"], None),
            None,
            "-failindex writes a var → bail"
        );
    }

    #[test]
    fn string_is_number_classes_are_version_aware() {
        use TclVersion::{V8_4, V8_5, V8_6, V9_0};
        let is = |class: &str, v: &str, ver| fold_is(&[class, v], ver);

        // `integer`: 8.x cap at 2^32-1, 9.0 unbounded.  All verified vs the
        // four tclsh.
        assert_eq!(
            is("integer", "4294967296", Some(V8_6)).as_deref(),
            Some("0")
        );
        assert_eq!(
            is("integer", "4294967296", Some(V9_0)).as_deref(),
            Some("1")
        );
        assert_eq!(is("integer", "1__0", Some(V9_0)).as_deref(), Some("1"));
        assert_eq!(is("integer", "0xff__ff", Some(V9_0)).as_deref(), Some("1"));
        assert_eq!(is("integer", "0d1__0", Some(V9_0)).as_deref(), Some("1"));
        assert_eq!(is("integer", "0x_f", Some(V9_0)).as_deref(), Some("0"));
        assert_eq!(is("integer", "1__0", None), None, "Tcl 9-only separators");
        assert_eq!(is("integer", "42", Some(V8_4)).as_deref(), Some("1"));
        assert_eq!(is("integer", "abc", Some(V9_0)).as_deref(), Some("0"));

        // `wideinteger`: raises on 8.4 (bail), unsigned-64 on 8.5/8.6,
        // signed-64 on 9.0.
        assert_eq!(is("wideinteger", "42", Some(V8_4)), None, "raises in 8.4");
        assert_eq!(is("wideinteger", "42", None), None, "unknown version");
        assert_eq!(
            is("wideinteger", "9223372036854775808", Some(V8_6)).as_deref(),
            Some("1"),
            "2^63 fits unsigned-64 on 8.6"
        );
        assert_eq!(
            is("wideinteger", "9223372036854775808", Some(V9_0)).as_deref(),
            Some("0"),
            "2^63 overflows signed-64 on 9.0"
        );
        assert_eq!(
            is("wideinteger", "-9223372036854775808", Some(V9_0)).as_deref(),
            Some("1"),
            "i64::MIN fits signed wide on 9.0"
        );
        assert_eq!(
            is("wideinteger", "-9223372036854775809", Some(V9_0)).as_deref(),
            Some("0"),
            "below i64::MIN overflows signed wide on 9.0"
        );
        assert_eq!(
            is("wideinteger", "0xff__ff", Some(V9_0)).as_deref(),
            Some("1")
        );
        assert_eq!(
            is("wideinteger", "18446744073709551616", Some(V8_6)).as_deref(),
            Some("0"),
            "2^64 overflows unsigned-64"
        );

        // `entier`: raises on 8.4/8.5, unbounded on 8.6/9.0.
        assert_eq!(is("entier", "42", Some(V8_5)), None, "raises in 8.5");
        assert_eq!(
            is("entier", "99999999999999999999999999", Some(V8_6)).as_deref(),
            Some("1"),
            "arbitrary precision"
        );
        assert_eq!(is("entier", "1__0", Some(V9_0)).as_deref(), Some("1"));
        assert_eq!(is("entier", "0xff__ff", Some(V9_0)).as_deref(), Some("1"));
        assert_eq!(is("entier", "0x_f", Some(V9_0)).as_deref(), Some("0"));
        assert_eq!(is("entier", "abc", Some(V9_0)).as_deref(), Some("0"));

        // `dict`: 9.0 only — even-length valid list.
        assert_eq!(is("dict", "a 1 b 2", Some(V8_6)), None, "raises before 9.0");
        assert_eq!(is("dict", "a 1 b 2", Some(V9_0)).as_deref(), Some("1"));
        assert_eq!(is("dict", "a 1 b", Some(V9_0)).as_deref(), Some("0"), "odd");
        assert_eq!(is("dict", "a b c", Some(V9_0)).as_deref(), Some("0"));
        // The empty-string shortcut is also version-gated.
        assert_eq!(is("dict", "", Some(V8_6)), None, "dict \"\" raises pre-9.0");
        assert_eq!(is("dict", "", Some(V9_0)).as_deref(), Some("1"));

        // RUST_ISSUE_083: tcl9.1 must fold identically to 9.0 (it is a 9.0+
        // superset), not degrade to the dialect-invariant 8.x subset. Assert
        // parity with 9.0 across the version-sensitive classes.
        let v91 = TclVersion::from_dialect(Some("tcl9.1"));
        assert_eq!(v91, Some(TclVersion::V9_1));
        for (class, val) in [
            ("integer", "4294967296"),
            ("integer", "1__0"),
            ("wideinteger", "9223372036854775808"),
            ("entier", "0xff__ff"),
            ("dict", "a 1 b 2"),
        ] {
            assert_eq!(
                is(class, val, v91),
                is(class, val, Some(V9_0)),
                "9.1 must match 9.0 for `string is {class} {val}`",
            );
        }
    }

    #[test]
    fn string_is_subcommand_carries_const_fold() {
        // `string is` is version-aware, so it registers a `const_fold_versioned`.
        let reg = CommandRegistry::build_default();
        let sub = reg
            .get("string")
            .and_then(|s| s.subcommand("is"))
            .expect("string is subcommand");
        assert!(sub.const_fold_versioned.is_some());
        assert_eq!(
            sub.run_const_fold(&["alpha", "abc"], Some("tcl9.0"))
                .as_deref(),
            Some("1")
        );
    }

    #[test]
    fn pure_string_subcommands_carry_const_fold() {
        // toupper / tolower / reverse expose a
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
        // The cat / repeat / trim* / totitle folds.
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
        // index / range / replace / first / last /
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
        assert_eq!(f("first")(&["", "abc"]).as_deref(), Some("-1"));
        assert_eq!(f("first")(&["", "abc", "1"]).as_deref(), Some("-1"));
        assert_eq!(f("first")(&["z", "abc"]).as_deref(), Some("-1"));
        assert_eq!(f("last")(&["b", "abcb"]).as_deref(), Some("3"));
        assert_eq!(f("last")(&["", "abc"]).as_deref(), Some("-1"));
        assert_eq!(f("last")(&["", "abc", "1"]).as_deref(), Some("-1"));
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
        // `string map` greedy left-to-right replace.
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

    // `fold_cat` is wired through the `ConstFoldFn` callback contract
    // (`-> Option<String>`) but pure concatenation never fails. Positive:
    // several args concatenate. Edge: zero args fold to the empty string —
    // it always returns `Some`, so the `Option` is a dispatch artefact.
    #[test]
    fn fold_cat_is_infallible() {
        assert_eq!(super::fold_cat(&["a", "b", "c"]).as_deref(), Some("abc"));
        assert_eq!(super::fold_cat(&[]).as_deref(), Some(""));
        // Non-ASCII is fine for cat (no per-char transform).
        assert_eq!(
            super::fold_cat(&["caf", "\u{e9}"]).as_deref(),
            Some("caf\u{e9}")
        );
        assert!(super::fold_cat(&["x"]).is_some());
    }
}
