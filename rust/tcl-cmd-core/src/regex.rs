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

//! `regexp` / `regsub` command **plumbing**, shared over a [`RegexEngine`]
//! provider and [`ValueOps`](tcl_syntax::value::ValueOps).
//!
//! Tcl's ARE matching has two separable halves: the **engine** (compile a
//! pattern, find matches in a codepoint string) and the **command plumbing**
//! (option parsing, the match/advance loop, `-indices`/`-inline`/`-start`/`-all`
//! handling, submatch-variable assignment, the `regsub` substitution-spec
//! expansion, and the match-count return semantics). The two runtimes share the
//! same engine *contract* but **not the same engine**: `runtime/rust` links the
//! real Tcl 9 Henry-Spencer ARE engine (so it is byte-for-byte tclsh), while the
//! bytecode VM drives the Rust `regex` crate (approximate — full ARE syntax like
//! `\m`/`\M`/`[[:<:]]` is out of scope there). This module is everything *except*
//! the engine: written once, run by both, so the VM inherits the runtime's full
//! option set and the correct char-offset semantics it lacked.
//!
//! All offsets here are **character** (codepoint) offsets, matching Tcl's index
//! model; [`decode_utf8`] maps the subject to codepoints plus a char→byte table
//! so the plumbing can slice the original bytes. The engine presents matches in
//! char offsets too (the FFI engine is natively codepoint-based; the VM's
//! crate-based engine translates byte↔char behind the seam).
//!
//! Semantics verified against tclsh 9.0; `runtime/rust`'s linked ARE engine is
//! the behavioural oracle for the loop.

use tcl_syntax::value::ValueOps;

use crate::option_table::OptionTable;

// the engine seam

/// The "did not participate" sentinel for a subexpression's offset (mirrors the
/// engine's `(size_t)-1`).
pub const NO_MATCH: usize = usize::MAX;

/// One reported (sub-)match: half-open `[so, eo)` in **character** offsets.
/// `so == NO_MATCH` means the subexpression did not participate.
#[derive(Clone, Copy)]
pub struct RegMatch {
    pub so: usize,
    pub eo: usize,
}

/// The compile-time options that affect matching, in an engine-neutral form
/// (each provider maps these to its own flags). `-line` sets both `linestop`
/// and `lineanchor`.
#[derive(Default, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // option flags, not a state machine
pub struct RegexFlags {
    /// `-nocase` — case-insensitive.
    pub nocase: bool,
    /// `-expanded` — whitespace/comments in the pattern are ignored.
    pub expanded: bool,
    /// `-linestop` — `.` and `[^…]` stop at a newline.
    pub linestop: bool,
    /// `-lineanchor` — `^`/`$` match at line boundaries.
    pub lineanchor: bool,
}

/// A compiled-regex provider. Stateless at the type level (compiling is a pure
/// function of pattern + flags), so the plumbing is generic over it without ever
/// holding a provider borrow across the match loop.
pub trait RegexEngine {
    /// The provider's compiled-pattern handle.
    type Regex;

    /// Compile `pattern` (UTF-8 bytes) with `flags`. On failure, return the
    /// engine's error *detail* bytes (the plumbing adds the standard prefix).
    ///
    /// # Errors
    /// The engine's compile-error detail for a malformed pattern.
    fn compile(pattern: &[u8], flags: RegexFlags) -> Result<Self::Regex, Vec<u8>>;

    /// Number of capturing subexpressions (so the whole match plus this many).
    fn nsub(re: &Self::Regex) -> usize;

    /// Find the leftmost match in `cps` (the whole subject as codepoints) at or
    /// after character `offset`. `notbol` requests that `^` not match at
    /// `offset` (the FFI engine's `REG_NOTBOL`; a context-aware crate engine can
    /// ignore it). Returns the match vector (index 0 = whole match, then each
    /// subexpression) in **absolute** character offsets, or `None` on no match.
    fn exec(
        re: &mut Self::Regex,
        cps: &[i32],
        offset: usize,
        notbol: bool,
    ) -> Option<Vec<RegMatch>>;
}

// shared error & result shapes

/// A `regexp`/`regsub` failure: the full, ready-to-report message bytes.
pub struct RegexError(pub Vec<u8>);

/// The outcome of [`regexp`] for the adapter to apply (var writes stay in the
/// adapter — they are Family-B state).
pub enum RegexpResult<V> {
    /// `-inline`: set the command result to this list value (no var writes).
    Inline(V),
    /// Non-inline: set the command result to the integer `count`. If `assign` is
    /// `Some`, also write each `(var-name, value)` first (a match occurred);
    /// `None` means no match — the match variables are left **untouched** (tclsh
    /// does not modify them on a failed match).
    Count {
        assign: Option<Vec<(Vec<u8>, V)>>,
        count: i64,
    },
}

/// The outcome of [`regsub`]: the substituted text, the substitution count, and
/// the optional result-variable name (the adapter sets the var or returns the
/// text, and owns the const-variable check).
pub struct RegsubResult {
    pub text: Vec<u8>,
    pub count: i64,
    pub var: Option<Vec<u8>>,
}

// pure helpers

/// Decode UTF-8 `bytes` into codepoints plus a parallel byte-offset table. The
/// returned `offsets` has length `codepoints.len() + 1`: `offsets[i]` is the
/// byte index where codepoint `i` starts, the final entry being `bytes.len()` —
/// so a character range `[so, eo)` slices the original bytes as
/// `bytes[offsets[so]..offsets[eo]]`. Invalid sequences decode to U+FFFD.
#[must_use]
pub fn decode_utf8(bytes: &[u8]) -> (Vec<i32>, Vec<usize>) {
    let mut cps = Vec::with_capacity(bytes.len());
    let mut offs = Vec::with_capacity(bytes.len() + 1);
    let mut i = 0;
    while i < bytes.len() {
        offs.push(i);
        let b0 = bytes[i];
        let (cp, n) = if b0 < 0x80 {
            (u32::from(b0), 1)
        } else if b0 & 0xE0 == 0xC0 {
            (u32::from(b0 & 0x1F), 2)
        } else if b0 & 0xF0 == 0xE0 {
            (u32::from(b0 & 0x0F), 3)
        } else if b0 & 0xF8 == 0xF0 {
            (u32::from(b0 & 0x07), 4)
        } else {
            (0xFFFD, 1)
        };
        let mut cp = cp;
        let mut taken = 1;
        if n > 1 && i + n <= bytes.len() {
            let mut ok = true;
            for j in 1..n {
                let b = bytes[i + j];
                if b & 0xC0 != 0x80 {
                    ok = false;
                    break;
                }
                cp = (cp << 6) | u32::from(b & 0x3F);
                taken += 1;
            }
            if !ok || taken != n {
                cp = 0xFFFD;
                taken = 1;
            }
        } else if n > 1 {
            cp = 0xFFFD;
            taken = 1;
        }
        // A codepoint is at most 0x10FFFF, well within i32 range.
        #[allow(clippy::cast_possible_wrap)]
        cps.push(cp as i32);
        i += taken;
    }
    offs.push(bytes.len());
    (cps, offs)
}

/// Tcl's per-iteration `REG_NOTBOL` rule: set unless `offset` is the very start
/// or follows a newline (so `^` behaves correctly in `-line` mode and at resumed
/// offsets).
fn notbol_at(cps: &[i32], offset: usize) -> bool {
    if offset == 0 {
        false
    } else if offset > cps.len() {
        true
    } else {
        cps[offset - 1] != i32::from(b'\n')
    }
}

fn parse_isize(b: &[u8]) -> Option<isize> {
    core::str::from_utf8(b).ok()?.trim().parse::<isize>().ok()
}

/// Resolve a `-start` index spec (integer / `end` / `end±N`) against the
/// character length, clamped to `0` (Tcl resets negatives to the start).
///
/// `N` in `end±N` is a user-supplied integer, so `len - 1 ± N` is done with
/// `saturating_*`: `end+9999999999999999999` would otherwise overflow `isize`
/// and wrap to a bogus (possibly in-range) offset. Saturating pins it to the
/// `isize` extremes — a too-large `+N` becomes "past the end" (the match loop's
/// `offset >= char_len` check then yields no match) and a too-large `-N` becomes
/// the start, both the intended clamp behaviour.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn resolve_start(spec: &[u8], char_len: usize) -> usize {
    let len = char_len as isize;
    let idx = if spec == b"end" {
        len - 1
    } else if let Some(rest) = spec.strip_prefix(b"end") {
        match rest.first() {
            Some(b'-') => parse_isize(&rest[1..]).map_or(0, |n| (len - 1).saturating_sub(n)),
            Some(b'+') => parse_isize(&rest[1..]).map_or(0, |n| (len - 1).saturating_add(n)),
            _ => 0,
        }
    } else {
        parse_isize(spec).unwrap_or(0)
    };
    if idx < 0 { 0 } else { idx as usize }
}

/// The compile flags + `-all`/`-start` shared by both commands' option sets.
#[derive(Default)]
struct Common {
    all: bool,
    flags: RegexFlags,
    start: Option<Vec<u8>>,
}

fn wrong_args(usage: &[u8]) -> RegexError {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    RegexError(m)
}

/// Wrap a provider compile-error detail in Tcl's standard prefix.
fn compile_error(detail: &[u8]) -> RegexError {
    let mut m = b"cannot compile regular expression pattern: ".to_vec();
    m.extend_from_slice(detail);
    RegexError(m)
}

// regexp

const REGEXP_USAGE: &[u8] = b"regexp ?-option ...? exp string ?matchVar? ?subMatchVar ...?";

// C's `options[]` in `Tcl_RegexpObjCmd` (`tclCmdMZ.c`), matched with
// `TCL_EXACT`: abbreviations are rejected, so `regexp -no …` is a bad option
// here. tclsh's bytecode compiler (`TclCompileRegexpCmd`) separately accepts
// any two-plus-character prefix of `-nocase` in its no-match-variable fast
// path `regexp ?-nocase? ?--? exp string`, so that one form abbreviates in
// tclsh scripts; every other form (match variables, any other option, an
// `eval`'d word list) reaches the runtime command and is exact-only. We
// implement the runtime semantics everywhere (probed tclsh 8.6.14; 9.0.4
// source).
const RE_ALL: usize = 0;
const RE_ABOUT: usize = 1;
const RE_INDICES: usize = 2;
const RE_INLINE: usize = 3;
const RE_EXPANDED: usize = 4;
const RE_LINE: usize = 5;
const RE_LINESTOP: usize = 6;
const RE_LINEANCHOR: usize = 7;
const RE_NOCASE: usize = 8;
const RE_START: usize = 9;
const REGEXP_NAMES: [&str; 11] = [
    "-all",
    "-about",
    "-indices",
    "-inline",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-nocase",
    "-start",
    "--",
];
const REGEXP_OPTIONS: OptionTable<'static> = OptionTable::exact_only("option", &REGEXP_NAMES);

/// Drive `regexp` over the engine `E` and value-ops `O`. `args` is the
/// command's arguments **without** the command name.
///
/// # Errors
/// Option/arg/compile errors as ready-to-report [`RegexError`] messages.
#[allow(clippy::too_many_lines)] // option scan + match loop + result build, read top-to-bottom
pub fn regexp<O: ValueOps, E: RegexEngine>(
    ops: &mut O,
    args: &[&[u8]],
) -> Result<RegexpResult<O::Value>, RegexError> {
    let mut c = Common::default();
    let mut indices = false;
    let mut inline = false;
    let mut about = false;

    let mut i = 0;
    while i < args.len() {
        let name = args[i];
        if name.first() != Some(&b'-') {
            break;
        }
        let idx = REGEXP_OPTIONS.index_of(name).map_err(RegexError)?;
        i += 1;
        match idx {
            RE_ALL => c.all = true,
            RE_ABOUT => about = true,
            RE_INDICES => indices = true,
            RE_INLINE => inline = true,
            RE_EXPANDED => c.flags.expanded = true,
            RE_LINE => {
                c.flags.linestop = true;
                c.flags.lineanchor = true;
            }
            RE_LINESTOP => c.flags.linestop = true,
            RE_LINEANCHOR => c.flags.lineanchor = true,
            RE_NOCASE => c.flags.nocase = true,
            RE_START => match args.get(i) {
                Some(v) => {
                    c.start = Some(v.to_vec());
                    i += 1;
                }
                // A trailing `-start` ends the options (C's `goto
                // endOfForLoop`); the arity check below then reports it.
                None => break,
            },
            // `--`: explicit end of options.
            _ => break,
        }
    }

    if about {
        return Err(RegexError(b"regexp -about is not yet supported".to_vec()));
    }
    let rest = &args[i..];
    if rest.len() < 2 {
        return Err(wrong_args(REGEXP_USAGE));
    }
    if inline && rest.len() > 2 {
        return Err(RegexError(
            b"regexp match variables not allowed when using -inline".to_vec(),
        ));
    }

    let pattern = rest[0];
    let str_bytes = rest[1];
    let (cps, byteoff) = decode_utf8(str_bytes);
    let char_len = cps.len();
    let match_vars = &rest[2..];

    let mut re = E::compile(pattern, c.flags).map_err(|d| compile_error(&d))?;
    let nsubs = E::nsub(&re);

    let mut offset = c
        .start
        .as_ref()
        .map_or(0, |spec| resolve_start(spec, char_len));

    // Tcl's `all` doubles as flag + counter: starts 1 if `-all`, else 0.
    let mut all_count: i64 = i64::from(c.all);
    let mut inline_items: Vec<O::Value> = Vec::new();
    let mut last_matches: Option<Vec<RegMatch>> = None;

    loop {
        let notbol = notbol_at(&cps, offset);
        let Some(matches) = E::exec(&mut re, &cps, offset, notbol) else {
            if all_count <= 1 {
                // No match at all (first time through).
                return Ok(if inline {
                    RegexpResult::Inline(ops.new_list(Vec::new()))
                } else {
                    RegexpResult::Count {
                        assign: None,
                        count: 0,
                    }
                });
            }
            break;
        };
        let m0 = matches[0];
        if inline {
            for k in 0..=nsubs {
                inline_items.push(build_match_item(
                    ops, &matches, k, nsubs, indices, str_bytes, &byteoff,
                ));
            }
        } else {
            last_matches = Some(matches);
        }

        if !c.all {
            break;
        }
        offset = m0.eo;
        if m0.eo == m0.so {
            offset += 1; // zero-length match: always advance to avoid looping
        }
        all_count += 1;
        if offset >= char_len {
            break;
        }
    }

    if inline {
        return Ok(RegexpResult::Inline(ops.new_list(inline_items)));
    }
    // Non-inline: build the (var, value) pairs from the final match. `-all`'s
    // match variables hold the *last* match, so building once after the loop is
    // both correct and leak-free (no intermediate values are constructed).
    let lm = last_matches.expect("a match occurred");
    let assign = match_vars
        .iter()
        .enumerate()
        .map(|(k, &name)| {
            let v = build_match_item(ops, &lm, k, nsubs, indices, str_bytes, &byteoff);
            (name.to_vec(), v)
        })
        .collect();
    Ok(RegexpResult::Count {
        assign: Some(assign),
        count: if all_count > 0 { all_count - 1 } else { 1 },
    })
}

/// Slice the original bytes for the character range `[so, eo)` via the char→byte
/// table, **guarding every index**. `byteoff` has one entry per character plus a
/// final `bytes.len()`, so `so`/`eo` must be `<= char_len`; the FFI ARE engine is
/// the behavioural oracle and should always honour that, but it is foreign code,
/// so an out-of-range or inverted `[so, eo)` must not index-panic here. Anything
/// off the table or backwards yields an empty slice rather than aborting.
fn slice_match<'a>(str_bytes: &'a [u8], byteoff: &[usize], so: usize, eo: usize) -> &'a [u8] {
    match (byteoff.get(so), byteoff.get(eo)) {
        (Some(&a), Some(&b)) if a <= b && b <= str_bytes.len() => &str_bytes[a..b],
        _ => &[],
    }
}

/// Build the value for match item `k`: an `{start end}` index pair (`-indices`)
/// or the matched substring (default). A non-participating group yields
/// `{-1 -1}` / the empty string, per `Tcl_RegexpObjCmd`.
// char offsets into a real subject are far below `i64::MAX`; the `i64` pair is
// Tcl's `-indices` result format.
#[allow(clippy::cast_possible_wrap)]
fn build_match_item<O: ValueOps>(
    ops: &mut O,
    matches: &[RegMatch],
    k: usize,
    nsubs: usize,
    indices: bool,
    str_bytes: &[u8],
    byteoff: &[usize],
) -> O::Value {
    let m = if k <= nsubs {
        matches.get(k).copied()
    } else {
        None
    };
    if indices {
        let (start, end): (i64, i64) = match m {
            Some(rm) if rm.so != NO_MATCH => (rm.so as i64, rm.eo as i64 - 1),
            _ => (-1, -1),
        };
        let a = ops.new_int(start);
        let b = ops.new_int(end);
        ops.new_list(vec![a, b])
    } else {
        match m {
            Some(rm) if rm.so != NO_MATCH && rm.eo > 0 => {
                ops.new_bytes(slice_match(str_bytes, byteoff, rm.so, rm.eo))
            }
            _ => ops.new_bytes(b""),
        }
    }
}

// regsub

const REGSUB_USAGE: &[u8] = b"regsub ?-option ...? exp string subSpec ?varName?";

// C's `options[]` in `Tcl_RegsubObjCmd` (`tclCmdMZ.c`), matched with
// `TCL_EXACT` like `regexp`'s — and here even tclsh's bytecode compiler
// (`TclCompileRegsubCmd`) only fast-paths a literal `-all`, so `regsub` is
// exact-only in every context.
const RS_ALL: usize = 0;
const RS_COMMAND: usize = 1;
const RS_EXPANDED: usize = 2;
const RS_LINE: usize = 3;
const RS_LINESTOP: usize = 4;
const RS_LINEANCHOR: usize = 5;
const RS_NOCASE: usize = 6;
const RS_START: usize = 7;
const REGSUB_NAMES: [&str; 9] = [
    "-all",
    "-command",
    "-expanded",
    "-line",
    "-linestop",
    "-lineanchor",
    "-nocase",
    "-start",
    "--",
];
const REGSUB_OPTIONS: OptionTable<'static> = OptionTable::exact_only("option", &REGSUB_NAMES);

/// Drive `regsub` over the engine `E`. `args` is the command's arguments
/// **without** the command name. The result string + count + optional var name
/// are returned for the adapter to apply.
///
/// # Errors
/// Option/arg/compile errors as ready-to-report [`RegexError`] messages.
pub fn regsub<E: RegexEngine>(args: &[&[u8]]) -> Result<RegsubResult, RegexError> {
    let mut c = Common::default();

    let mut i = 0;
    while i < args.len() {
        let name = args[i];
        if name.first() != Some(&b'-') {
            break;
        }
        let idx = REGSUB_OPTIONS.index_of(name).map_err(RegexError)?;
        i += 1;
        match idx {
            RS_ALL => c.all = true,
            RS_COMMAND => {
                return Err(RegexError(b"regsub -command is not yet supported".to_vec()));
            }
            RS_EXPANDED => c.flags.expanded = true,
            RS_LINE => {
                c.flags.linestop = true;
                c.flags.lineanchor = true;
            }
            RS_LINESTOP => c.flags.linestop = true,
            RS_LINEANCHOR => c.flags.lineanchor = true,
            RS_NOCASE => c.flags.nocase = true,
            RS_START => match args.get(i) {
                Some(v) => {
                    c.start = Some(v.to_vec());
                    i += 1;
                }
                // A trailing `-start` ends the options (C's `goto
                // endOfForLoop`); the arity check below then reports it.
                None => break,
            },
            // `--`: explicit end of options.
            _ => break,
        }
    }

    let rest = &args[i..];
    if rest.len() < 3 || rest.len() > 4 {
        return Err(wrong_args(REGSUB_USAGE));
    }
    let pattern = rest[0];
    let str_bytes = rest[1];
    let subspec = rest[2];
    let var = rest.get(3).map(|&v| v.to_vec());

    let (cps, byteoff) = decode_utf8(str_bytes);
    let char_len = cps.len();

    let mut re = E::compile(pattern, c.flags).map_err(|d| compile_error(&d))?;
    let nsubs = E::nsub(&re);

    let mut offset = c
        .start
        .as_ref()
        .map_or(0, |spec| resolve_start(spec, char_len));

    let mut result: Vec<u8> = Vec::new();
    let mut count: i64 = 0;

    while offset <= char_len {
        let notbol = offset > 0 && cps[offset - 1] != i32::from(b'\n');
        let Some(matches) = E::exec(&mut re, &cps, offset, notbol) else {
            break;
        };
        if count == 0 && offset > 0 {
            // Copy the skipped prefix when a `-start` offset was given.
            result.extend_from_slice(&str_bytes[..byteoff[offset]]);
        }
        count += 1;

        let m0 = matches[0];
        // Text before this match.
        result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[m0.so]]);
        // The substitution spec, with `&`/`\N` expanded.
        apply_subspec(&mut result, subspec, &matches, nsubs, str_bytes, &byteoff);

        // Advance, always consuming at least one char on an empty match.
        if m0.eo == offset {
            if offset < char_len {
                result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[offset + 1]]);
            }
            offset += 1;
        } else {
            offset = m0.eo;
            if m0.so == m0.eo {
                if offset < char_len {
                    result.extend_from_slice(&str_bytes[byteoff[offset]..byteoff[offset + 1]]);
                }
                offset += 1;
            }
        }
        if !c.all {
            break;
        }
    }

    let text = if count == 0 {
        str_bytes.to_vec()
    } else {
        if offset < char_len {
            result.extend_from_slice(&str_bytes[byteoff[offset]..]);
        }
        result
    };

    Ok(RegsubResult { text, count, var })
}

/// Expand a `regsub` substitution spec into `out`: `&` / `\0` → whole match,
/// `\N` → capture group N, `\\` → `\`, `\&` → `&`; any other run is copied
/// verbatim. Mirrors the `wsubspec` scan in `Tcl_RegsubObjCmd`.
fn apply_subspec(
    out: &mut Vec<u8>,
    sub: &[u8],
    matches: &[RegMatch],
    nsubs: usize,
    str_bytes: &[u8],
    byteoff: &[usize],
) {
    let mut k = 0;
    let mut run = 0; // start of the current verbatim run
    while k < sub.len() {
        let ch = sub[k];
        let idx: usize;
        if ch == b'&' {
            idx = 0;
        } else if ch == b'\\' {
            match sub.get(k + 1) {
                Some(&d) if d.is_ascii_digit() => idx = (d - b'0') as usize,
                Some(&d) if d == b'\\' || d == b'&' => {
                    // Literal `\` or `&`: flush the run, emit the bare char.
                    out.extend_from_slice(&sub[run..k]);
                    out.push(d);
                    k += 2;
                    run = k;
                    continue;
                }
                _ => {
                    // Backslash before any other char: keep both verbatim.
                    k += 1;
                    continue;
                }
            }
        } else {
            k += 1;
            continue;
        }
        // Reached for `&` or `\N`: flush the verbatim run, then the group.
        out.extend_from_slice(&sub[run..k]);
        if idx <= nsubs
            && let Some(rm) = matches.get(idx)
            && rm.so != NO_MATCH
        {
            // Guard the engine-supplied offsets (see `slice_match`): a foreign ARE
            // engine returning an out-of-range `[so, eo)` must not index-panic.
            out.extend_from_slice(slice_match(str_bytes, byteoff, rm.so, rm.eo));
        }
        k += if ch == b'\\' { 2 } else { 1 };
        run = k;
    }
    out.extend_from_slice(&sub[run..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_start_handles_integer_and_end_forms() {
        // regexp `-start` index (cmd-core regex.rs had no unit coverage):
        // integer / end / end±N against char length, clamped to 0.
        assert_eq!(resolve_start(b"5", 10), 5);
        assert_eq!(resolve_start(b"0", 10), 0);
        assert_eq!(resolve_start(b"end", 10), 9);
        assert_eq!(resolve_start(b"end-2", 10), 7);
        assert_eq!(resolve_start(b"end+1", 10), 10);
        assert_eq!(resolve_start(b"-3", 10), 0); // Tcl resets negatives to start
        assert_eq!(resolve_start(b"bad", 10), 0); // unparseable → 0
        assert_eq!(resolve_start(b"end", 0), 0); // end of empty clamps to 0
    }

    #[test]
    fn resolve_start_end_offset_saturates_without_overflow() {
        // A giant `end±N` must not overflow the isize add/sub. An `N` near
        // `isize::MAX` still parses, so `(len-1) ± N` is where the wrap would
        // happen — `saturating_*` pins it instead: `end+N` to "past the end" (the
        // match loop then finds nothing), `end-N` back to the start.
        let big = b"end+9223372036854775800"; // close to isize::MAX, parses fine
        assert_eq!(resolve_start(big, 10), isize::MAX as usize);
        assert_eq!(resolve_start(b"end-9223372036854775800", 10), 0);
        // A value too big to even parse as isize falls back to 0 (also panic-free).
        assert_eq!(resolve_start(b"end+99999999999999999999999", 10), 0);
    }

    #[test]
    fn slice_match_guards_out_of_range_engine_offsets() {
        // The byte-offset slice must never index-panic on a (foreign)
        // engine offset past the char→byte table or with `eo < so`.
        let bytes = b"hello";
        let byteoff = [0usize, 1, 2, 3, 4, 5]; // 5 chars + final len
        assert_eq!(slice_match(bytes, &byteoff, 1, 4), b"ell");
        // `eo` past the table → empty, not a panic.
        assert_eq!(slice_match(bytes, &byteoff, 0, 99), b"");
        // `so` past the table → empty.
        assert_eq!(slice_match(bytes, &byteoff, 99, 100), b"");
        // Inverted range → empty.
        assert_eq!(slice_match(bytes, &byteoff, 4, 1), b"");
        // The `NO_MATCH` sentinel as an index → empty (never indexes).
        assert_eq!(slice_match(bytes, &byteoff, NO_MATCH, NO_MATCH), b"");
    }

    #[test]
    fn parse_isize_trims_and_rejects_garbage() {
        assert_eq!(parse_isize(b"42"), Some(42));
        assert_eq!(parse_isize(b"-5"), Some(-5));
        assert_eq!(parse_isize(b"  3  "), Some(3));
        assert_eq!(parse_isize(b"x"), None);
        assert_eq!(parse_isize(b""), None);
    }
}
