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

//! Portable `string` command logic, generic over [`ValueOps`].
//!
//! Each helper takes already-sliced argument values and returns
//! `Result<O::Value, CmdError>`. All indexing is by **character** (the contract
//! in [`tcl_syntax::value`]), so a byte-oriented runtime conforms via its
//! `ValueOps` impl and these bodies stay correct everywhere.
//!
//! [`dispatch`] covers the portable subcommands shared across both
//! runtimes (`length`/`index`/`range`/`map`/`match`/`replace`/`insert`/
//! `first`/`last`/`trim*`/`wordstart`/`wordend`/`to{upper,lower,title}`/
//! `reverse`/`repeat`/`cat`). It returns `None` for any subcommand a given
//! runtime still implements itself, so a host can fall back to its own
//! body for the few not routed here.

use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use crate::index;

/// `string length str` — the character count.
pub fn length<O: ValueOps>(ops: &mut O, s: &O::Value) -> O::Value {
    let n = ops.char_len(s);
    ops.new_int(i64::try_from(n).unwrap_or(i64::MAX))
}

/// `string index str charIndex` — the one-character string at `idx`, or empty
/// when out of range.
pub fn index<O: ValueOps>(ops: &mut O, s: &O::Value, idx: &O::Value) -> Result<O::Value, CmdError> {
    let chars: Vec<char> = ops.as_str(s).chars().collect();
    let i = index::resolve(&ops.as_str(idx), chars.len())?;
    if i < 0 {
        return Ok(ops.empty());
    }
    match usize::try_from(i).ok().and_then(|i| chars.get(i)) {
        Some(c) => Ok(ops.new_string(c.to_string())),
        None => Ok(ops.empty()),
    }
}

/// `string range str first last` — the inclusive character slice, clamped.
pub fn range<O: ValueOps>(
    ops: &mut O,
    s: &O::Value,
    first: &O::Value,
    last: &O::Value,
) -> Result<O::Value, CmdError> {
    let chars: Vec<char> = ops.as_str(s).chars().collect();
    let len = chars.len();
    let lo = index::resolve(&ops.as_str(first), len)?.max(0);
    let hi = index::resolve(&ops.as_str(last), len)?;
    let Ok(lo) = usize::try_from(lo) else {
        return Ok(ops.empty());
    };
    if hi < 0 || lo >= len || lo > usize::try_from(hi).unwrap_or(usize::MAX) {
        return Ok(ops.empty());
    }
    let hi = usize::try_from(hi).unwrap_or(usize::MAX).min(len - 1);
    let out: String = chars[lo..=hi].iter().collect();
    Ok(ops.new_string(out))
}

/// `string reverse str`.
pub fn reverse<O: ValueOps>(ops: &mut O, s: &O::Value) -> O::Value {
    let out: String = ops.as_str(s).chars().rev().collect();
    ops.new_string(out)
}

/// `string repeat str count` — `count` (clamped at 0) copies.
pub fn repeat<O: ValueOps>(
    ops: &mut O,
    s: &O::Value,
    count: &O::Value,
) -> Result<O::Value, CmdError> {
    let n = ops.as_int(count)?;
    if n <= 0 {
        return Ok(ops.empty());
    }
    let src = ops.as_str(s);
    let n = usize::try_from(n).unwrap_or(0);
    Ok(ops.new_string(src.repeat(n)))
}

/// The result form of [`compare`].
#[derive(Clone, Copy)]
pub enum CompareMode {
    /// `string equal` returns a Tcl boolean.
    Equal,
    /// `string compare` returns `-1`, `0`, or `1`.
    Compare,
}

/// `string equal`/`string compare` with `?-nocase? ?-length int?`.
///
/// Option order and prefix acceptance match Tcl's `StringCmpOpts`; integer
/// conversion is delegated to [`ValueOps::string_compare_length`] so each
/// target preserves its value-tower-specific diagnostic.
pub fn compare<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
    mode: CompareMode,
) -> Result<O::Value, CmdError> {
    let name = match mode {
        CompareMode::Equal => "equal",
        CompareMode::Compare => "compare",
    };
    let usage = format!("string {name} ?-nocase? ?-length int? string1 string2");
    if !(2..=5).contains(&args.len()) {
        return Err(CmdError::wrong_args(&usage));
    }

    // A deliberate non-`OptionTable` site (#1607): C's `StringCmpOpts`
    // hand-rolls `strncmp(…, length > 1)` instead of calling
    // `Tcl_GetIndexFromObj`, so `""` and a lone `-` are `bad`, never
    // `ambiguous`, and a one-character word never abbreviates. Routing this
    // through the shared matcher would change all three verdicts.
    //
    // tclsh 8.6.16 / 9.0.4:
    //   string compare "" a b  -> bad option "": must be -nocase or -length
    //   string compare -  a b  -> bad option "-": must be -nocase or -length
    //   string compare -n a A  -> 0
    let mut nocase = false;
    let mut length = None;
    let mut i = 0;
    let option_end = args.len() - 2;
    while i < option_end {
        let option = ops.as_str(&args[i]);
        if option.len() > 1 && "-nocase".starts_with(&*option) {
            nocase = true;
            i += 1;
        } else if option.len() > 1 && "-length".starts_with(&*option) {
            if i + 1 >= option_end {
                return Err(CmdError::wrong_args(&usage));
            }
            length = ops.string_compare_length(&args[i + 1])?;
            i += 2;
        } else {
            return Err(CmdError::new(format!(
                "bad option \"{option}\": must be -nocase or -length"
            )));
        }
    }

    let key = |ops: &mut O, value: &O::Value| {
        let mut chars: Vec<char> = ops.as_str(value).chars().collect();
        if nocase {
            chars = chars.iter().flat_map(|c| c.to_lowercase()).collect();
        }
        if let Some(n) = length {
            chars.truncate(n);
        }
        chars
    };
    let ordering = key(ops, &args[i]).cmp(&key(ops, &args[i + 1]));
    Ok(match mode {
        CompareMode::Equal => ops.new_bool(ordering.is_eq()),
        CompareMode::Compare => ops.new_int(match ordering {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Greater => 1,
        }),
    })
}

/// Which case conversion [`case_convert`] performs.
#[derive(Clone, Copy)]
pub enum CaseMode {
    /// `string toupper`.
    Upper,
    /// `string tolower`.
    Lower,
    /// `string totitle` — first char of the range uppercased, rest lowercased.
    Title,
}

/// `string toupper`/`tolower`/`totitle string ?first? ?last?` — convert the case
/// of the characters in `[first, last]` (default the whole string; with only
/// `first`, that single character), leaving the rest unchanged.
pub fn case_convert<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
    mode: CaseMode,
    usage: &str,
) -> Result<O::Value, CmdError> {
    let (s, first_spec, last_spec) = match args {
        [s] => (s, None, None),
        [s, f] => (s, Some(f), None),
        [s, f, l] => (s, Some(f), Some(l)),
        _ => return Err(CmdError::wrong_args(usage)),
    };
    let chars: Vec<char> = ops.as_str(s).chars().collect();
    let len = chars.len();
    if len == 0 {
        return Ok(ops.empty());
    }
    let first = match first_spec {
        None => 0,
        Some(f) => index::resolve(&ops.as_str(f), len)?.max(0),
    };
    let last = match last_spec {
        Some(l) => index::resolve(&ops.as_str(l), len)?,
        None if first_spec.is_some() => first,
        None => i64::try_from(len).unwrap_or(i64::MAX) - 1,
    };
    let mut out = String::new();
    for (idx, &c) in chars.iter().enumerate() {
        let i = i64::try_from(idx).unwrap_or(i64::MAX);
        if i < first || i > last {
            out.push(c);
            continue;
        }
        // Per-character *simple* mapping throughout ([`simple_upper`] &co), the
        // way C's `Tcl_UtfTo{Upper,Lower,Title}` do it: `string toupper ß` is
        // `ß`, not `SS`. `string totitle`'s first character uses the Unicode
        // *titlecase* mapping (`Tcl_UniCharToTitle`), which differs from
        // uppercase only for the Latin DŽ/LJ/NJ/DZ digraphs (string-17.7), and
        // its remaining characters go through `Tcl_UtfToTitle`'s
        // lowercase-with-Georgian-exception rule.
        out.push(match mode {
            CaseMode::Upper => simple_upper(c),
            CaseMode::Lower => simple_lower(c),
            CaseMode::Title if i == first => simple_title(c),
            CaseMode::Title => simple_title_rest(c),
        });
    }
    Ok(ops.new_string(out))
}

/// C's `Tcl_UtfTo{Upper,Lower,Title}` length guard: the mapped character
/// replaces the original only when its UTF-8 encoding is no longer than the
/// original's — "only copy the upper case char to dst if its size is <= the
/// original char" (`tclUtf.c:1334-1346`, and the same test in the lower/title
/// loops). It keeps e.g. `ɐ` (U+0250, 2 bytes) unchanged rather than mapping it
/// to `Ɐ` (U+2C6F, 3 bytes).
fn fit(orig: char, mapped: char) -> char {
    if orig.len_utf8() < mapped.len_utf8() {
        orig
    } else {
        mapped
    }
}

/// The Unicode **simple** (1:1) case mapping of `c`, given its *full* mapping
/// iterator.
///
/// C's case conversions are strictly per-code-point — `Tcl_UtfToUpper` calls
/// `Tcl_UniCharToUpper` (`tclUtf.c:1777-1789`), a delta-encoded table with one
/// code point in and one out — whereas Rust's [`char::to_uppercase`] /
/// [`char::to_lowercase`] implement *full* mapping and can expand (`ß` → `SS`,
/// `İ` → `i` + U+0307).
///
/// An expanding mapping usually means the character has **no** simple mapping
/// (Unicode leaves `UnicodeData.txt`'s mapping field empty and puts the
/// multi-character form in `SpecialCasing.txt`), so it is left alone: `ß`,
/// `ﬁ`, `ŉ` and friends are identity under `Tcl_UniCharToUpper`. The one
/// exception is handled by [`simple_lower_exception`] — inferring identity from
/// expansion alone is wrong for it.
fn simple(c: char, mut full: impl Iterator<Item = char>) -> char {
    match (full.next(), full.next()) {
        (Some(m), None) => fit(c, m),
        _ => c,
    }
}

/// The lone character whose *full* lowercase mapping expands even though it has
/// a real 1:1 simple mapping: `İ` (U+0130) lowercases to `i` (U+0069) under
/// `Tcl_UniCharToLower`, while Rust's full mapping yields `i` + U+0307
/// (combining dot above).
///
/// U+0130 is the **only** such code point — it is the only one in Unicode whose
/// full lowercase expands at all (every other expanding lowercase in
/// `SpecialCasing.txt` is conditional on locale or final-sigma context, which C
/// does not implement either), so this exception is complete rather than a
/// sample. The expanding *uppercase* and *titlecase* mappings all do have empty
/// simple-mapping fields, so [`simple`]'s identity inference is correct there.
const fn simple_lower_exception(c: char) -> Option<char> {
    if c == '\u{0130}' { Some('i') } else { None }
}

/// `Tcl_UniCharToUpper` — the simple uppercase mapping of one character.
#[must_use]
pub fn simple_upper(c: char) -> char {
    simple(c, c.to_uppercase())
}

/// `Tcl_UniCharToLower` — the simple lowercase mapping of one character.
#[must_use]
pub fn simple_lower(c: char) -> char {
    if let Some(m) = simple_lower_exception(c) {
        return fit(c, m);
    }
    simple(c, c.to_lowercase())
}

/// `Tcl_UniCharToTitle` — the simple titlecase mapping of one character, which
/// equals the uppercase mapping except for the digraphs [`titlecase_digraph`]
/// knows about.
#[must_use]
pub fn simple_title(c: char) -> char {
    titlecase_digraph(c).map_or_else(|| simple_upper(c), |t| fit(c, t))
}

/// `Tcl_UtfToTitle`'s mapping for the characters *after* the first: lowercase,
/// except the Georgian Mtavruli block (U+1C90..U+1CBF), which C deliberately
/// leaves alone — "Special exception for Georgian Asomtavruli chars, no
/// titlecase" (`tclUtf.c:1452-1456`).
#[must_use]
pub fn simple_title_rest(c: char) -> char {
    if ('\u{1C90}'..='\u{1CBF}').contains(&c) {
        c
    } else {
        simple_lower(c)
    }
}

/// The Unicode titlecase mapping for the characters whose `Titlecase_Mapping`
/// differs from their `Uppercase_Mapping` — the Latin DŽ/LJ/NJ/DZ digraphs (all
/// three case forms of each map to the single title form). Every other character
/// titlecases to its uppercase mapping, so those return `None` (the caller falls
/// back to [`simple_upper`]).
fn titlecase_digraph(c: char) -> Option<char> {
    Some(match c {
        '\u{01C4}' | '\u{01C5}' | '\u{01C6}' => '\u{01C5}', // DŽ / Dž / dž → Dž
        '\u{01C7}' | '\u{01C8}' | '\u{01C9}' => '\u{01C8}', // LJ / Lj / lj → Lj
        '\u{01CA}' | '\u{01CB}' | '\u{01CC}' => '\u{01CB}', // NJ / Nj / nj → Nj
        '\u{01F1}' | '\u{01F2}' | '\u{01F3}' => '\u{01F2}', // DZ / Dz / dz → Dz
        _ => return None,
    })
}

/// `string replace string first last ?newstring?` — remove the characters in
/// `[first, last]` (inclusive), optionally inserting `newstring`. An empty or
/// inverted range leaves the string unchanged.
pub fn replace<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    if args.len() < 3 || args.len() > 4 {
        return Err(CmdError::wrong_args(
            "string replace string first last ?string?",
        ));
    }
    let chars: Vec<char> = ops.as_str(&args[0]).chars().collect();
    let len = chars.len();
    let end = i64::try_from(len).unwrap_or(i64::MAX) - 1;
    let first = index::resolve(&ops.as_str(&args[1]), len)?;
    let last = index::resolve(&ops.as_str(&args[2]), len)?;
    if last < 0 || first > end || last < first {
        let unchanged: String = chars.iter().collect();
        return Ok(ops.new_string(unchanged));
    }
    let first = first.max(0);
    let last = last.min(end);
    let lo = usize::try_from(first).unwrap_or(0);
    let hi_excl = usize::try_from(last + 1).unwrap_or(0).min(len);
    let mut out: String = chars[..lo].iter().collect();
    if args.len() == 4 {
        out.push_str(&ops.as_str(&args[3]));
    }
    out.extend(chars[hi_excl..].iter());
    Ok(ops.new_string(out))
}

/// `string insert string index insertString` — insert `insertString` before the
/// character at `index`. `end` denotes the position *after* the last character
/// (so it appends); the index resolves against `len + 1`.
pub fn insert<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    let [s, idx, ins] = args else {
        return Err(CmdError::wrong_args(
            "string insert string index insertString",
        ));
    };
    let chars: Vec<char> = ops.as_str(s).chars().collect();
    let len = chars.len();
    let at = index::resolve(&ops.as_str(idx), len + 1)?;
    let at = if at < 0 {
        0
    } else {
        usize::try_from(at).unwrap_or(len).min(len)
    };
    let mut out: String = chars[..at].iter().collect();
    out.push_str(&ops.as_str(ins));
    out.extend(chars[at..].iter());
    Ok(ops.new_string(out))
}

/// `string cat ?arg ...?` — concatenate the string reps of all arguments.
pub fn cat<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> O::Value {
    let mut out = String::new();
    for a in args {
        out.push_str(&ops.as_str(a));
    }
    ops.new_string(out)
}

/// The default `string trim` set — every Unicode space character plus NUL
/// (`tclDefaultTrimSet`, TIP #413).
const DEFAULT_TRIM_SET: &[char] = &[
    '\u{09}', '\u{0a}', '\u{0b}', '\u{0c}', '\u{0d}', ' ', '\u{00}', '\u{85}', '\u{a0}',
    '\u{1680}', '\u{180e}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{200b}', '\u{2028}', '\u{2029}',
    '\u{202f}', '\u{205f}', '\u{2060}', '\u{3000}', '\u{feff}',
];

/// `string trim`/`trimleft`/`trimright` — strip `chars` (default the TIP #413
/// whitespace set) from the requested ends.
pub fn trim<O: ValueOps>(
    ops: &mut O,
    s: &O::Value,
    chars: Option<&O::Value>,
    left: bool,
    right: bool,
) -> O::Value {
    let string = ops.as_str(s).to_string();
    let custom: Option<Vec<char>> = chars.map(|c| ops.as_str(c).chars().collect());
    let pred = |c: char| match &custom {
        Some(set) => set.contains(&c),
        None => DEFAULT_TRIM_SET.contains(&c),
    };
    let trimmed = match (left, right) {
        (true, true) => string.trim_matches(pred),
        (true, false) => string.trim_start_matches(pred),
        (false, true) => string.trim_end_matches(pred),
        (false, false) => string.as_str(),
    };
    ops.new_string(trimmed.to_string())
}

/// `string first needleString haystackString ?startIndex?` — the character index
/// of the first occurrence of `needle` at or after `start` (default 0), or -1.
pub fn first<O: ValueOps>(
    ops: &mut O,
    needle: &O::Value,
    haystack: &O::Value,
    start: Option<&O::Value>,
) -> Result<O::Value, CmdError> {
    let hay: Vec<char> = ops.as_str(haystack).chars().collect();
    let needle: Vec<char> = ops.as_str(needle).chars().collect();
    let start = match start {
        None => 0,
        Some(s) => usize::try_from(index::resolve(&ops.as_str(s), hay.len())?.max(0)).unwrap_or(0),
    };
    if needle.is_empty() {
        return Ok(ops.new_int(-1));
    }
    let mut idx: i64 = -1;
    if needle.len() <= hay.len() {
        for i in start..=hay.len() - needle.len() {
            if hay[i..i + needle.len()] == needle[..] {
                idx = i64::try_from(i).unwrap_or(i64::MAX);
                break;
            }
        }
    }
    Ok(ops.new_int(idx))
}

/// `string last needleString haystackString ?lastIndex?` — the character index of
/// the last occurrence ending at or before `lastIndex` (default end), or -1.
pub fn last<O: ValueOps>(
    ops: &mut O,
    needle: &O::Value,
    haystack: &O::Value,
    last_index: Option<&O::Value>,
) -> Result<O::Value, CmdError> {
    let hay: Vec<char> = ops.as_str(haystack).chars().collect();
    let needle: Vec<char> = ops.as_str(needle).chars().collect();
    let last: i64 = match last_index {
        None => i64::try_from(hay.len()).unwrap_or(i64::MAX) - 1,
        Some(s) => index::resolve(&ops.as_str(s), hay.len())?,
    };
    // An empty haystack can hold no non-empty needle — return -1 before the
    // clamp below, whose `saturating_sub(1)` would otherwise yield index 0 and
    // panic when the search slices `hay[0..needle.len()]` (e.g.
    // `string last a {} 0`).
    if needle.is_empty() || last < 0 || hay.is_empty() {
        return Ok(ops.new_int(-1));
    }
    let last = usize::try_from(last)
        .unwrap_or(0)
        .min(hay.len().saturating_sub(1));
    let mut idx: i64 = -1;
    // `needle.len() <= hay.len()` guards the `hay[i..i + needle.len()]` slice:
    // without it, an empty (or too-short) haystack with an explicit index
    // (`string last a "" 0`) drives `last` to 0, passes `last + 1 >= needle`,
    // and slices past the end. `string first` has the same guard.
    if needle.len() <= hay.len() && last + 1 >= needle.len() {
        let hi = last + 1 - needle.len();
        for i in (0..=hi).rev() {
            if hay[i..i + needle.len()] == needle[..] {
                idx = i64::try_from(i).unwrap_or(i64::MAX);
                break;
            }
        }
    }
    Ok(ops.new_int(idx))
}

/// `string match ?-nocase? pattern string` — glob match, returning a boolean.
pub fn string_match<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    let (nocase, pat, s) = match args {
        [p, s] => (false, p, s),
        [opt, p, s] if is_nocase(&ops.as_str(opt)) => (true, p, s),
        // Three arguments whose first is not `-nocase` is a bad option, not a
        // wrong count (C's `StringMatchCmd`): `string match -bogus a b` and
        // `string match -- a a` both report `bad option "…": must be -nocase`
        // (string-11.x), since `--` is not an accepted option here.
        [opt, _, _] => {
            return Err(CmdError::new(format!(
                "bad option \"{}\": must be -nocase",
                ops.as_str(opt)
            )));
        }
        _ => {
            return Err(CmdError::wrong_args(
                "string match ?-nocase? pattern string",
            ));
        }
    };
    let pattern = ops.as_str(pat).to_string();
    let text = ops.as_str(s).to_string();
    Ok(ops.new_bool(tcl_syntax::glob::string_case_match(&pattern, &text, nocase)))
}

/// Whether `opt` is `-nocase` or an unambiguous prefix of it (`-n`, `-no`, …).
///
/// Deliberately NOT the shared [`crate::prefix`] matcher: C's
/// `StringMatchCmd`/`StringMapCmd` hand-roll `(length > 1) && strncmp(string,
/// "-nocase", length)` instead of calling `Tcl_GetIndexFromObj`, so a lone
/// `-` is a bad option here (`string match - a b` errors in tclsh) even
/// though the table rule would accept it as a unique prefix of a one-entry
/// table.
fn is_nocase(opt: &str) -> bool {
    opt.len() >= 2 && "-nocase".starts_with(opt)
}

/// `string map ?-nocase? charMap string` — replace substrings per the
/// `key value ...` map, scanning left-to-right and taking the first matching key
/// at each position (advancing past it), else copying one character.
/// Full Unicode simple-lowercase fold of a character sequence, used by
/// `string map -nocase` to compare keys against the source the same way
/// `string equal -nocase` and `tolower` do.
fn fold_chars(cs: &[char]) -> Vec<char> {
    cs.iter().flat_map(|c| c.to_lowercase()).collect()
}

pub fn map<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    let (nocase, pairs, text) = match args {
        [m, s] => (false, m, s),
        [opt, m, s] if is_nocase(&ops.as_str(opt)) => (true, m, s),
        // Three arguments whose first is not `-nocase` is a bad option, not a
        // wrong count (C's `StringMapCmd`): `string map {a b} abba oops`
        // reports `bad option "a b"` (string-10.2).
        [opt, _, _] => {
            return Err(CmdError::new(format!(
                "bad option \"{}\": must be -nocase",
                ops.as_str(opt)
            )));
        }
        _ => return Err(CmdError::wrong_args("string map ?-nocase? charMap string")),
    };
    let items = ops.list_elements(pairs)?;
    if items.len() % 2 != 0 {
        return Err(CmdError::new("char map list unbalanced"));
    }
    let mut map: Vec<(String, String)> = Vec::with_capacity(items.len() / 2);
    for c in items.as_chunks::<2>().0 {
        map.push((ops.as_str(&c[0]).to_string(), ops.as_str(&c[1]).to_string()));
    }
    let string = ops.as_str(text).to_string();

    // Case-insensitive matching folds full Unicode case (like `string equal
    // -nocase` and `tolower`), not just ASCII, and matches/advances by
    // *character* so a key whose case fold changes byte length stays aligned
    // with C's `Tcl_UniCharNcasecmp` over `length2` characters.
    let keys: Vec<(Vec<char>, Vec<char>, &str)> = map
        .iter()
        .filter(|(from, _)| !from.is_empty())
        .map(|(from, to)| {
            let chars: Vec<char> = from.chars().collect();
            let folded = if nocase {
                fold_chars(&chars)
            } else {
                Vec::new()
            };
            (chars, folded, to.as_str())
        })
        .collect();
    let src: Vec<char> = string.chars().collect();
    let mut out = String::with_capacity(string.len());
    let mut i = 0;
    'outer: while i < src.len() {
        for (key, folded_key, to) in &keys {
            let klen = key.len();
            if i + klen > src.len() {
                continue;
            }
            let region = &src[i..i + klen];
            let hit = if nocase {
                fold_chars(region) == *folded_key
            } else {
                region == key.as_slice()
            };
            if hit {
                out.push_str(to);
                i += klen;
                continue 'outer;
            }
        }
        out.push(src[i]);
        i += 1;
    }
    Ok(ops.new_string(out))
}

/// Arity-check (`string ?chars?`) wrapper shared by the three `trim` arms.
fn trim_dispatch<O: ValueOps>(
    ops: &mut O,
    rest: &[O::Value],
    usage: &str,
    left: bool,
    right: bool,
) -> Result<O::Value, CmdError> {
    match rest {
        [s] => Ok(trim(ops, s, None, left, right)),
        [s, c] => Ok(trim(ops, s, Some(c), left, right)),
        _ => Err(CmdError::wrong_args(usage)),
    }
}

/// Dispatch a `string` subcommand whose name `sub` is **already canonical** (the
/// runtime has resolved any unique-prefix abbreviation), with `rest` being the
/// arguments after the subcommand.
///
/// Returns `Some(result)` when the subcommand is handled here, or `None` when it
/// is not handled here — letting the calling runtime fall back to its own
/// implementation. The `Some(Err(..))` case is a genuine command error (arity,
/// bad index, …).
pub fn dispatch_canon<O: ValueOps>(
    ops: &mut O,
    sub: &str,
    rest: &[O::Value],
) -> Option<Result<O::Value, CmdError>> {
    let arity = |n: usize, usage: &str| -> Option<Result<O::Value, CmdError>> {
        if rest.len() == n {
            None
        } else {
            Some(Err(CmdError::wrong_args(usage)))
        }
    };
    match sub {
        "length" => arity(1, "string length string").or_else(|| Some(Ok(length(ops, &rest[0])))),
        "index" => arity(2, "string index string charIndex")
            .or_else(|| Some(index(ops, &rest[0], &rest[1]))),
        "range" => arity(3, "string range string first last")
            .or_else(|| Some(range(ops, &rest[0], &rest[1], &rest[2]))),
        "reverse" => arity(1, "string reverse string").or_else(|| Some(Ok(reverse(ops, &rest[0])))),
        "repeat" => {
            arity(2, "string repeat string count").or_else(|| Some(repeat(ops, &rest[0], &rest[1])))
        }
        "equal" => Some(compare(ops, rest, CompareMode::Equal)),
        "compare" => Some(compare(ops, rest, CompareMode::Compare)),
        "cat" => Some(Ok(cat(ops, rest))),
        "match" => Some(string_match(ops, rest)),
        "map" => Some(map(ops, rest)),
        "toupper" => Some(case_convert(
            ops,
            rest,
            CaseMode::Upper,
            "string toupper string ?first? ?last?",
        )),
        "tolower" => Some(case_convert(
            ops,
            rest,
            CaseMode::Lower,
            "string tolower string ?first? ?last?",
        )),
        "totitle" => Some(case_convert(
            ops,
            rest,
            CaseMode::Title,
            "string totitle string ?first? ?last?",
        )),
        "replace" => Some(replace(ops, rest)),
        "insert" => Some(insert(ops, rest)),
        "first" => match rest {
            [n, h] => Some(first(ops, n, h, None)),
            [n, h, s] => Some(first(ops, n, h, Some(s))),
            _ => Some(Err(CmdError::wrong_args(
                "string first needleString haystackString ?startIndex?",
            ))),
        },
        "last" => match rest {
            [n, h] => Some(last(ops, n, h, None)),
            [n, h, s] => Some(last(ops, n, h, Some(s))),
            _ => Some(Err(CmdError::wrong_args(
                "string last needleString haystackString ?lastIndex?",
            ))),
        },
        "trim" => Some(trim_dispatch(
            ops,
            rest,
            "string trim string ?chars?",
            true,
            true,
        )),
        "trimleft" => Some(trim_dispatch(
            ops,
            rest,
            "string trimleft string ?chars?",
            true,
            false,
        )),
        "trimright" => Some(trim_dispatch(
            ops,
            rest,
            "string trimright string ?chars?",
            false,
            true,
        )),
        "wordstart" => arity(2, "string wordstart string index")
            .or_else(|| Some(word_bound(ops, &rest[0], &rest[1], true))),
        "wordend" => arity(2, "string wordend string index")
            .or_else(|| Some(word_bound(ops, &rest[0], &rest[1], false))),
        // Not handled here — caller falls back to its own path (`is`, …).
        _ => None,
    }
}

/// `string wordstart|wordend str charIndex` — the bounds of the word containing
/// `charIndex` (`StringStartCmd`/`StringEndCmd`). A "word" is a maximal run of
/// word-characters, or a single non-word character. `start` selects `wordstart`
/// (the index of the word's first char) vs `wordend` (one past its last char).
pub fn word_bound<O: ValueOps>(
    ops: &mut O,
    s: &O::Value,
    idx: &O::Value,
    start: bool,
) -> Result<O::Value, CmdError> {
    let chars: Vec<char> = ops.as_str(s).chars().collect();
    let index = index::resolve(&ops.as_str(idx), chars.len())?;
    let result = if start {
        word_start(&chars, index)
    } else {
        word_end(&chars, index)
    };
    Ok(ops.new_int(i64::try_from(result).unwrap_or(i64::MAX)))
}

/// The start of the word containing (clamped) `index` — walk left over word
/// chars. A non-word char at `index` is its own word start.
fn word_start(chars: &[char], index: i64) -> usize {
    let n = chars.len();
    if n == 0 {
        return 0;
    }
    let last = i64::try_from(n - 1).unwrap_or(i64::MAX);
    let i = index.min(last);
    if i <= 0 {
        return 0;
    }
    let i = usize::try_from(i).unwrap_or(0);
    if !is_word_char(chars[i]) {
        return i;
    }
    let mut j = i;
    while j > 0 && is_word_char(chars[j - 1]) {
        j -= 1;
    }
    j
}

/// The end (one past the last char) of the word containing (clamped) `index` —
/// walk right over word chars. A non-word char advances exactly one.
fn word_end(chars: &[char], index: i64) -> usize {
    let n = chars.len();
    if index >= i64::try_from(n).unwrap_or(i64::MAX) {
        return n;
    }
    let i = usize::try_from(index.max(0)).unwrap_or(0);
    let mut cur = i;
    while cur < n && is_word_char(chars[cur]) {
        cur += 1;
    }
    if cur == i { cur + 1 } else { cur }
}

/// `Tcl_UniCharIsWordChar`: letters and decimal digits (approximated by
/// `char::is_alphanumeric`) plus connector punctuation.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || is_connector_punct(c)
}

/// The Unicode connector-punctuation set Tcl treats as word characters
/// (underscore and friends).
fn is_connector_punct(c: char) -> bool {
    matches!(
        c,
        '\u{005F}'
            | '\u{203F}'
            | '\u{2040}'
            | '\u{2054}'
            | '\u{FE33}'
            | '\u{FE34}'
            | '\u{FE4D}'
            | '\u{FE4E}'
            | '\u{FE4F}'
            | '\u{FF3F}'
    )
}

/// Dispatch a `string` subcommand from the raw argument vector (`args[0]` is the
/// subcommand). Convenience over [`dispatch_canon`] for a runtime that does not
/// pre-resolve abbreviations; exact subcommand names only.
pub fn dispatch<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Option<Result<O::Value, CmdError>> {
    let sub = ops.as_str(args.first()?).to_string();
    dispatch_canon(ops, &sub, &args[1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    /// A throwaway string-only `ValueOps` for the `first`/`last` index tests.
    #[derive(Default)]
    struct StrOps;

    impl ValueOps for StrOps {
        type Value = String;
        fn new_str(&mut self, s: &str) -> String {
            s.to_owned()
        }
        fn new_int(&mut self, n: i64) -> String {
            n.to_string()
        }
        fn new_double(&mut self, f: f64) -> String {
            tcl_syntax::number::format_double(f)
        }
        fn new_bool(&mut self, b: bool) -> String {
            (if b { "1" } else { "0" }).to_owned()
        }
        fn new_list(&mut self, items: Vec<String>) -> String {
            items.join(" ")
        }
        fn as_str(&mut self, v: &String) -> std::rc::Rc<str> {
            std::rc::Rc::from(v.as_str())
        }
        fn as_int(&mut self, v: &String) -> Result<i64, tcl_syntax::value::ValueError> {
            v.parse()
                .map_err(|_| tcl_syntax::value::ValueError::NotInteger(v.clone()))
        }
        fn as_double(&mut self, _v: &String) -> Result<f64, tcl_syntax::value::ValueError> {
            Ok(0.0)
        }
        fn as_bool(&mut self, _v: &String) -> Result<bool, tcl_syntax::value::ValueError> {
            Ok(false)
        }
        fn list_elements(
            &mut self,
            v: &String,
        ) -> Result<Vec<String>, tcl_syntax::value::ValueError> {
            Ok(v.split_whitespace().map(str::to_owned).collect())
        }
    }

    fn last_of(needle: &str, haystack: &str, last_index: Option<&str>) -> i64 {
        let mut ops = StrOps;
        let n = needle.to_owned();
        let h = haystack.to_owned();
        let li = last_index.map(str::to_owned);
        let r = last(&mut ops, &n, &h, li.as_ref()).expect("string last");
        r.parse().unwrap()
    }

    #[test]
    fn string_last_empty_haystack_with_index_does_not_panic() {
        // `string last a "" 0` must return -1, not panic on an
        // out-of-bounds slice of the empty haystack.
        assert_eq!(last_of("a", "", Some("0")), -1);
        assert_eq!(last_of("a", "", None), -1);
        // Needle longer than haystack with an explicit index.
        assert_eq!(last_of("abc", "ab", Some("1")), -1);
    }

    #[test]
    fn string_last_basic() {
        assert_eq!(last_of("a", "banana", None), 5);
        assert_eq!(last_of("an", "banana", None), 3);
        assert_eq!(last_of("an", "banana", Some("2")), 1);
        assert_eq!(last_of("z", "banana", None), -1);
    }

    #[test]
    fn string_compare_equal_share_option_and_unicode_semantics() {
        let mut ops = StrOps;
        assert_eq!(
            compare(
                &mut ops,
                &[
                    "-noc".to_owned(),
                    "-len".to_owned(),
                    "2".to_owned(),
                    "ABxx".to_owned(),
                    "abYY".to_owned(),
                ],
                CompareMode::Compare,
            )
            .unwrap(),
            "0"
        );
        // Tcl folds before applying -length: U+0130 lowercases to `i` plus a
        // combining dot, whose first character compares equal to `i`.
        assert_eq!(
            compare(
                &mut ops,
                &[
                    "-nocase".to_owned(),
                    "-length".to_owned(),
                    "1".to_owned(),
                    "\u{0130}".to_owned(),
                    "i".to_owned(),
                ],
                CompareMode::Equal,
            )
            .unwrap(),
            "1"
        );
        assert_eq!(
            compare(
                &mut ops,
                &["-bogus".to_owned(), "a".to_owned(), "b".to_owned()],
                CompareMode::Equal,
            )
            .unwrap_err()
            .message(),
            "bad option \"-bogus\": must be -nocase or -length"
        );
    }

    #[test]
    fn word_start_finds_word_boundaries() {
        // Tcl `string wordstart` semantics. "abc def" = indices a0 b1 c2 ' '3 d4 e5 f6.
        let c = chars("abc def");
        assert_eq!(word_start(&c, 1), 0); // inside "abc"
        assert_eq!(word_start(&c, 5), 4); // inside "def"
        assert_eq!(word_start(&c, 3), 3); // the space is its own word
        assert_eq!(word_start(&c, 0), 0);
        assert_eq!(word_start(&c, 100), 4); // clamps to last char of "def"
        assert_eq!(word_start(&[], 3), 0); // empty
    }

    #[test]
    fn totitle_first_char_uses_titlecase_not_uppercase() {
        let mut ops = StrOps;
        let title = |ops: &mut StrOps, s: &str| {
            case_convert(ops, &[s.to_owned()], CaseMode::Title, "u").unwrap()
        };
        // The Latin dz digraph (U+01F3) titlecases to Dž (U+01F2), not the
        // uppercase DZ (U+01F1) — string-17.7; the rest is lowercased.
        assert_eq!(title(&mut ops, "\u{01F3}BCabc"), "\u{01F2}bcabc");
        assert_eq!(title(&mut ops, "\u{01C9}x"), "\u{01C8}x"); // lj → Lj
        // A non-digraph first char still titlecases to its uppercase.
        assert_eq!(title(&mut ops, "hELLO"), "Hello");
        // `toupper` of the digraph is the uppercase form (U+01F1), not titlecase.
        assert_eq!(
            case_convert(&mut ops, &["\u{01F3}".to_owned()], CaseMode::Upper, "u").unwrap(),
            "\u{01F1}"
        );
    }

    #[test]
    fn case_mapping_is_simple_not_full() {
        // C's `Tcl_UtfTo{Upper,Lower,Title}` map one code point to one code
        // point through `Tcl_UniCharTo{Upper,Lower,Title}` (`tclUtf.c:1777-1858`),
        // which hold only Unicode's *simple* mappings — so a Rust *full* mapping
        // that expands has no C counterpart and the character is preserved.
        assert_eq!(simple_upper('\u{00DF}'), '\u{00DF}'); // ß, not SS
        assert_eq!(simple_upper('\u{FB01}'), '\u{FB01}'); // ﬁ, not FI
        // …but an expanding *full* mapping does not imply the character has no
        // simple mapping, and U+0130 is the one place that matters: its full
        // lowercase is `i` + U+0307, while `Tcl_UniCharToLower` maps it to plain
        // `i` (`UnicodeData.txt` field 13 = 0069). Inferring identity from the
        // expansion alone made `string tolower` and `STR_LOWER` disagree with C.
        // It is the only such code point — the only one in Unicode whose full
        // lowercase expands at all.
        assert_eq!(simple_lower('\u{0130}'), 'i');
        // Ordinary 1:1 mappings still apply, including the shrinking ones.
        assert_eq!(simple_upper('a'), 'A');
        assert_eq!(simple_lower('É'), 'é');
        assert_eq!(simple_upper('\u{0131}'), 'I'); // ı (2 bytes) → I (1 byte)
        // C's length guard keeps a mapping that would *grow* the UTF-8 encoding
        // ("only copy the upper case char to dst if its size is <= the original
        // char", `tclUtf.c:1338-1345`): ɐ (U+0250, 2 bytes) does not become
        // Ɐ (U+2C6F, 3 bytes).
        assert_eq!(simple_upper('\u{0250}'), '\u{0250}');
        // Titlecase differs from uppercase only for the Latin digraphs.
        assert_eq!(simple_title('\u{01F3}'), '\u{01F2}'); // dz → Dž
        assert_eq!(simple_title('h'), 'H');
        // `Tcl_UtfToTitle`'s rest-of-string rule lowercases, except the Georgian
        // Mtavruli block it deliberately skips (`tclUtf.c:1452-1456`).
        assert_eq!(simple_title_rest('A'), 'a');
        assert_eq!(simple_title_rest('\u{1C90}'), '\u{1C90}');
        assert_eq!(simple_title_rest('\u{1CC0}'), '\u{1CC0}'); // just past the block
    }

    #[test]
    fn word_end_finds_word_boundaries() {
        // Tcl `string wordend` — end is one past the last word char.
        let c = chars("abc def");
        assert_eq!(word_end(&c, 1), 3); // end of "abc" (exclusive)
        assert_eq!(word_end(&c, 4), 7); // end of "def"
        assert_eq!(word_end(&c, 3), 4); // a non-word char advances exactly one
        assert_eq!(word_end(&c, 100), 7); // past end → length
    }

    #[test]
    fn string_last_empty_haystack_is_minus_one_not_panic() {
        // Regression: `string last a {} 0` clamped `last` to 0 and then sliced
        // an empty haystack, panicking. An empty haystack holds no match → -1.
        assert_eq!(last_of("a", "", Some("0")), -1);
        assert_eq!(last_of("a", "", None), -1);
        assert_eq!(last_of("abc", "", Some("end")), -1);
    }

    #[test]
    fn string_last_finds_last_occurrence() {
        assert_eq!(last_of("a", "banana", None), 5);
        assert_eq!(last_of("an", "banana", None), 3);
        // Bounded search: last "an" ending at or before index 2.
        assert_eq!(last_of("an", "banana", Some("2")), 1);
        // No match → -1.
        assert_eq!(last_of("z", "banana", None), -1);
    }
}
