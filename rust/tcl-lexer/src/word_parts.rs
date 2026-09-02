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

//! **The** owner of "split a Tcl word into its substitution components".
//!
//! One decomposer for the `TCL_TOKEN_TEXT` / `TCL_TOKEN_BS` /
//! `TCL_TOKEN_VARIABLE` / `TCL_TOKEN_COMMAND` breakdown C Tcl's `ParseTokens`
//! (`tclParse.c`) produces for a bare or `"quoted"` word, for a `subst`
//! template, and for an array index. Before this module the same walk existed
//! four times — `runtime/rust/src/parse.rs`'s `scan_parts`, that crate's
//! `subst.rs` mirror, `tcl-vm`'s `subst.rs`, and the compiler's
//! `segmenter.rs`/`ir.rs` `WordExpr` builder — and drifted: only one of them
//! raised C's `missing close-bracket`, only one respected brace nesting when
//! finding a `]`, and only one decoded the backslashes of a literal run.
//!
//! ## Why it lives in `tcl-lexer`
//!
//! Its neighbours are already here: [`crate::braced_var_name_end`] (the
//! release-variant `${…}` close rule), [`crate::scan_array_index`] (the
//! release-variant `$name(index)` scan), [`crate::command_substitution_end`]
//! (the brace/quote/comment-aware `]` search) and
//! [`crate::close_quote_offset`]. This crate sits below `tcl-syntax`, both
//! runtimes and the compiler in the dependency DAG, so a leaf module here is
//! reachable from every consumer without inverting an edge.
//!
//! ## The model
//!
//! A borrow-based enum tree, not C's `numComponents` index arithmetic:
//!
//! - [`WordBody::Literal`] — nothing to substitute; **the bytes are the
//!   value**, borrowed from the source. C's `TCL_TOKEN_SIMPLE_WORD` fast path.
//! - [`WordBody::Parts`] — [`WordPart::Text`] (a literal run with its
//!   backslash escapes already folded in — there is no separate `BS` part, so
//!   one decoder produces `subst`'s answer, not two), [`WordPart::Variable`],
//!   [`WordPart::Command`], [`WordPart::ParseError`].
//!
//! Everything borrows `&'s [u8]` from the caller's source; only a `Text` run
//! that actually had an escape to decode owns its bytes. That keeps the
//! literal fast path zero-copy, which is what lets the runtime's `parse_cache`
//! hold parsed commands against a stable script slab without the stale-slab
//! hazard of memory-management.md MM-B.6 — the borrow makes reuse-after-free a
//! compile error rather than a runtime one.
//!
//! ## Errors are parts, not a `Result`
//!
//! The scan is infallible: a malformed construct becomes a
//! [`WordPart::ParseError`] carrying C's exact message, and the parts before
//! it are still returned. That is not leniency, it is C's *order*. Verified
//! against `tclsh9.0` (9.0.4) and `tclsh8.6` (8.6.16):
//!
//! ```text
//! % proc side {} { puts ran; return S }
//! % subst {[side][b}
//! ran
//! missing close-bracket
//! ```
//!
//! `subst` substitutes incrementally, so the earlier `[side]` runs and keeps
//! its side effects before the bad bracket is reported. A consumer that wants
//! C's *script*-parsing order instead (`Tcl_ParseCommand` parses every word of
//! a command before evaluating any of them, so nothing runs) scans all its
//! words first and raises the first [`WordPart::ParseError`] it finds before
//! resolving anything.
//!
//! ## Adoption by `tcl-compiler::segmenter`
//!
//! Not done here, deliberately — see
//! `docs/design/lanes/wasm-native-lowering.md` § `r10-word-parts`. The API is
//! shaped so it can be: [`decompose`] takes the word's *content* span plus a
//! [`LexerConfig`] and returns parts whose byte extents are recoverable from
//! the borrows, which is what `WordExpr`/`WordPart` need to keep their public
//! shape (`CommandTokens::from_segmented` maps part-for-part). The segmenter
//! keeps owning *command* and *word* boundaries; only the within-word
//! breakdown moves here.

use std::borrow::Cow;

use tcl_dialect::EscapeSyntax;

use crate::lexer::LexerConfig;
use crate::ranges::{
    ArrayIndexEnd, BracedVarEnd, INVALID_CHARACTER_IN_ARRAY_INDEX, MISSING_CLOSE_BRACE_FOR_VAR,
    braced_var_name_end, close_quote_offset, command_substitution_end_bytes, scan_array_index,
};
use crate::substitution::backslash_subst_in;

/// C Tcl's error for a `"`-delimited word that never finds its closing quote
/// (`Tcl_ParseQuotedString` → `TCL_PARSE_MISSING_QUOTE`, `tclParse.c`).
/// Byte-exact on 8.6.16 and 9.0.4: `eval {list a "b}` reports `missing "`.
pub const MISSING_QUOTE: &str = "missing \"";

/// C Tcl's error for a `[` command substitution that never finds its closing
/// bracket (`ParseTokens` → `TCL_PARSE_MISSING_BRACKET`, `tclParse.c`).
/// `eval {list a [b}` and `subst {[b}` both report `missing close-bracket` on
/// 8.6.16 and 9.0.4.
pub const MISSING_CLOSE_BRACKET: &str = "missing close-bracket";

/// C Tcl's error for a `{`-delimited word that never finds its closing brace
/// (`Tcl_ParseBraces` → `TCL_PARSE_MISSING_BRACE`, `tclParse.c`).
pub const MISSING_CLOSE_BRACE: &str = "missing close-brace";

/// C Tcl's error for a `$name(` array reference whose index never closes
/// (`Tcl_ParseVarName` → `TCL_PARSE_MISSING_PAREN`, `tclParse.c`).
/// `subst {$x(}` reports `missing )` on 8.6.16 and 9.0.4.
pub const MISSING_PAREN: &str = "missing )";

/// Which substitution kinds the scan performs — C's `TCL_SUBST_*` bits, and
/// `subst`'s `-nobackslashes` / `-nocommands` / `-novariables`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Four independent switches, which is what C's `TCL_SUBST_*` bit set and
// `subst`'s three `-no*` options are; a state machine would be a fiction over
// them.
#[allow(clippy::struct_excessive_bools)]
pub struct SubstFlags {
    /// `$var` / `${var}` / `$arr(i)` (off under `-novariables`).
    pub vars: bool,
    /// `[cmd]` (off under `-nocommands`).
    pub cmds: bool,
    /// `\x` escapes (off under `-nobackslashes`).
    pub backslashes: bool,
    /// Whether a **bare** `$name` (no braces) is a variable reference.
    ///
    /// True everywhere C Tcl parses source. It is false for exactly one
    /// consumer: `tcl-vm`'s compiled-word `PUSH` operands, where the compiler
    /// has already inlined every bare `$name` it resolved and normalised the
    /// rest to `${name}`, so a surviving bare `$` is literal data. Modelling
    /// that as a flag on the shared scan keeps the VM on this owner instead of
    /// justifying a private copy.
    pub bare_var_refs: bool,
}

impl Default for SubstFlags {
    /// Everything on — a bare/quoted source word, and plain `subst`.
    fn default() -> Self {
        Self {
            vars: true,
            cmds: true,
            backslashes: true,
            bare_var_refs: true,
        }
    }
}

impl SubstFlags {
    /// The compiled-word flavour: `${…}` and `[…]` substitute, a bare `$` is
    /// literal. See [`SubstFlags::bare_var_refs`].
    #[must_use]
    pub const fn compiled_word() -> Self {
        Self {
            vars: true,
            cmds: true,
            backslashes: true,
            bare_var_refs: false,
        }
    }
}

/// One substitution component of a non-literal word (or `subst` input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordPart<'s> {
    /// A literal run with its backslash escapes already decoded under the
    /// release's grammar when backslash substitution is active. Borrows the
    /// source when there was nothing to decode (the fast path).
    Text(Cow<'s, [u8]>),
    /// `$name` / `${name}` / `$arr(index)`.
    Variable(VarRef<'s>),
    /// `[...]` command substitution — the inner script, brackets stripped.
    Command(&'s [u8]),
    /// A construct C's parser rejects, carrying its exact message. See the
    /// module docs for why this is a part and not a `Result`.
    ParseError(&'static str),
}

/// A variable reference whose array index (if any) is itself decomposed —
/// the index is substituted at evaluation time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VarRef<'s> {
    /// The bare / `${…}` name, always literal.
    pub name: &'s [u8],
    /// `Some` for `$arr(index)`: the index's own components.
    pub index: Option<Vec<WordPart<'s>>>,
}

/// A variable reference with its array index left as a **raw source span** —
/// what a consumer that substitutes the index itself needs (`tcl-vm`'s
/// `subst`, whose index substitution has its own control-flow rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawVarRef<'s> {
    /// The bare / `${…}` name.
    pub name: &'s [u8],
    /// `Some` for `$arr(index)`: the unsubstituted index text.
    pub index: Option<&'s [u8]>,
    /// Byte offset just past the whole reference.
    pub next: usize,
}

/// A word's content: a literal (nothing to substitute) or a component list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordBody<'s> {
    /// The bytes are the value — C's `TCL_TOKEN_SIMPLE_WORD`.
    Literal(&'s [u8]),
    /// Components to substitute and concatenate.
    Parts(Vec<WordPart<'s>>),
}

/// Cap on `$name(index)` nesting depth [`decompose`] recurses into while
/// parsing an index's own components.
///
/// The index parse is self-recursive with no natural bound — `$a($b($c(…)))`
/// costs one native frame group per `(` — and is reachable from ordinary
/// `subst` / word substitution with no special syntax, so unbounded input
/// aborts the process with an uncatchable stack overflow (issue #996).
/// Empirically that class of recursion overflowed a 256 KiB stack between
/// depth 100-150 and a 1 MiB stack by depth 2000; `crate::lexer`'s
/// `MAX_ARRAY_INDEX_DEPTH` measured the same construct at the token layer.
/// 64 is far past any real nesting and comfortably under every measured crash
/// threshold, with room for a smaller WASM host stack. Past the cap the index
/// is kept as one literal text run instead of recursing — graceful
/// degradation, matching `scan_array_index_body`'s.
const MAX_INDEX_DEPTH: u32 = 64;

/// Decompose `src` into its substitution components under `flags`.
///
/// `src` is a word's **content** (delimiters already stripped) or a `subst`
/// template. Returns [`WordBody::Literal`] — a borrow of `src` — when no
/// enabled substitution actually occurs.
#[must_use]
pub fn decompose(src: &[u8], flags: SubstFlags, config: LexerConfig) -> WordBody<'_> {
    decompose_at_depth(src, flags, config, 0)
}

/// Where the `"`-delimited word opening at `open_quote` closes, or
/// [`MISSING_QUOTE`].
///
/// The one place the `missing "` spelling is decided. Delegates the search to
/// [`close_quote_offset`], which steps over `\X` pairs and complete `[…]`
/// substitutions so a `"` inside either does not close the word.
pub fn quoted_word_close(source: &str, open_quote: usize) -> Result<usize, &'static str> {
    close_quote_offset(source, open_quote).ok_or(MISSING_QUOTE)
}

/// Scan the `$`-reference at `src[at]`, leaving any array index as raw source.
///
/// `Ok(None)` means the `$` is **not** a reference and is literal text — C's
/// rule when the next byte is neither `{` nor a name byte. An unterminated
/// `${…}` / `$name(` is different: those are C's errors, returned as `Err`.
///
/// Both release axes are resolved through their own owners:
/// [`braced_var_name_end`] for the `${…}` close rule (8.x stops at the first
/// literal `}`; 9.x counts nesting and skips `\X`) and [`scan_array_index`]
/// for which raw bytes an index accepts.
pub fn scan_var_ref(
    src: &[u8],
    at: usize,
    config: LexerConfig,
) -> Result<Option<RawVarRef<'_>>, &'static str> {
    debug_assert_eq!(src.get(at), Some(&b'$'), "caller must point at a `$`");
    if src.get(at + 1) == Some(&b'{') {
        let name_start = at + 2;
        return match braced_var_name_end(src, name_start, config.braced_var) {
            BracedVarEnd::Closed(end) => Ok(Some(RawVarRef {
                name: &src[name_start..end],
                index: None,
                next: end + 1,
            })),
            BracedVarEnd::Unterminated => Err(MISSING_CLOSE_BRACE_FOR_VAR),
        };
    }
    let start = at + 1;
    let name_end = tcl_core_types::naming::scan_var_name_end(src, start);
    if name_end == start {
        return Ok(None);
    }
    if src.get(name_end) == Some(&b'(') {
        let scan = scan_array_index(src, name_end, config.array_index, config.braced_var);
        if scan.invalid.is_some() {
            return Err(INVALID_CHARACTER_IN_ARRAY_INDEX);
        }
        return match scan.end {
            ArrayIndexEnd::Closed(end) => Ok(Some(RawVarRef {
                name: &src[start..name_end],
                index: Some(&src[name_end + 1..end - 1]),
                next: end,
            })),
            ArrayIndexEnd::Unterminated => Err(MISSING_PAREN),
        };
    }
    Ok(Some(RawVarRef {
        name: &src[start..name_end],
        index: None,
        next: name_end,
    }))
}

/// Where the `[` command substitution at `at` closes — one byte **past** the
/// `]` — or the error C reports for it.
///
/// The search is [`command_substitution_end`](crate::command_substitution_end),
/// which is brace-, quote- and comment-aware: a `]` written inside `{…}`, a
/// `"…"` word, or a `#` comment of the substituted script does not close it.
/// Every private copy this module replaced got at least one of those wrong.
///
/// When nothing closes it, the error is **not** automatically
/// [`MISSING_CLOSE_BRACKET`]. A substituted `[…]` is a *script*: C recurses
/// into `Tcl_ParseCommand` at the bracket rather than hunting for the matching
/// `]` first, so an error it meets inside the bracket surfaces instead. With
/// `t` = `[set y ${a{b]`, `subst $t` reports `missing close-brace for variable
/// name` on both oracles, not `missing close-bracket`.
pub fn command_subst_close(
    src: &[u8],
    at: usize,
    flags: SubstFlags,
    config: LexerConfig,
) -> Result<usize, &'static str> {
    match command_substitution_end_bytes(src, at) {
        Some(end) => Ok(end),
        None => Err(error_inside_unterminated_bracket(src, at, flags, config)),
    }
}

/// Is `c` a valid first byte of a bare `$name`? A `$` not followed by one of
/// these (or `{`) is a literal `$` in Tcl (`Tcl_ParseVarName`).
fn is_var_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b':'
}

fn decompose_at_depth(
    src: &[u8],
    flags: SubstFlags,
    config: LexerConfig,
    depth: u32,
) -> WordBody<'_> {
    let len = src.len();
    let triggered = src.iter().any(|&c| {
        (flags.vars && c == b'$') || (flags.cmds && c == b'[') || (flags.backslashes && c == b'\\')
    });
    if !triggered {
        return WordBody::Literal(src);
    }
    // Computed once per call, not per byte: whether this call already sits at
    // the index-nesting cap, so any `$name(index)` here keeps its index as
    // literal text rather than recursing further.
    let past_cap = depth >= MAX_INDEX_DEPTH;

    let mut parts: Vec<WordPart> = Vec::new();
    let mut lit_start = 0usize;
    let mut i = 0usize;

    while i < len {
        let c = src[i];
        if flags.vars && c == b'$' && starts_var_ref(src, i, flags) {
            flush_text(&mut parts, src, lit_start, i, flags, config.escapes);
            match scan_var_ref(src, i, config) {
                Ok(Some(raw)) => {
                    let index = raw.index.map(|idx| {
                        if past_cap {
                            vec![WordPart::Text(Cow::Borrowed(idx))]
                        } else {
                            match decompose_at_depth(idx, flags, config, depth + 1) {
                                WordBody::Literal(b) => vec![WordPart::Text(Cow::Borrowed(b))],
                                WordBody::Parts(p) => p,
                            }
                        }
                    });
                    parts.push(WordPart::Variable(VarRef {
                        name: raw.name,
                        index,
                    }));
                    i = raw.next;
                }
                // `Ok(None)` cannot happen: `starts_var_ref` already tested
                // the same condition `scan_var_ref` returns `None` for. Kept
                // total anyway — a literal `$` is the answer either way.
                Ok(None) => {
                    i += 1;
                    continue;
                }
                // C stops parsing here. So do we: the components already
                // scanned are kept (they run first — see the module docs) and
                // the error terminates the walk.
                Err(msg) => {
                    parts.push(WordPart::ParseError(msg));
                    return WordBody::Parts(parts);
                }
            }
            lit_start = i;
        } else if flags.cmds && c == b'[' {
            flush_text(&mut parts, src, lit_start, i, flags, config.escapes);
            match command_subst_close(src, i, flags, config) {
                Ok(end) => {
                    parts.push(WordPart::Command(&src[i + 1..end - 1]));
                    i = end;
                }
                Err(msg) => {
                    parts.push(WordPart::ParseError(msg));
                    return WordBody::Parts(parts);
                }
            }
            lit_start = i;
        } else if flags.backslashes && c == b'\\' && i + 1 < len {
            // Step over the escaped byte so an escaped `$` / `[` is not a
            // substitution boundary. Only the byte immediately after the
            // backslash matters here — a longer escape's trailing digits are
            // ordinary run text — because the whole run is decoded once, at
            // `flush_text`.
            i += 2;
        } else {
            i += 1;
        }
    }
    flush_text(&mut parts, src, lit_start, len, flags, config.escapes);
    // C's `TCL_TOKEN_SIMPLE_WORD` collapse. A span whose only component is one
    // *borrowed* text run had nothing to substitute after all — a `$` with no
    // name behind it, a `[` that the flags left inert — so it goes back as a
    // zero-copy `Literal`. An *owned* run (escapes were decoded) cannot: it no
    // longer borrows `src`.
    if let [WordPart::Text(Cow::Borrowed(b))] = parts.as_slice() {
        return WordBody::Literal(b);
    }
    WordBody::Parts(parts)
}

/// [`command_subst_close`]'s error choice for an unterminated `[`.
///
/// The walk is iterative on purpose: recursing back into [`decompose`] would
/// cost one native frame per unterminated bracket, and `[[[[[…` is ordinary
/// `subst` input.
fn error_inside_unterminated_bracket(
    src: &[u8],
    at: usize,
    flags: SubstFlags,
    config: LexerConfig,
) -> &'static str {
    let mut i = at + 1;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'$' if flags.vars && starts_var_ref(src, i, flags) => {
                match scan_var_ref(src, i, config) {
                    Ok(Some(raw)) => i = raw.next,
                    Ok(None) => i += 1,
                    Err(msg) => return msg,
                }
            }
            _ => i += 1,
        }
    }
    MISSING_CLOSE_BRACKET
}

/// Whether the `$` at `at` opens a variable reference under `flags`.
fn starts_var_ref(src: &[u8], at: usize, flags: SubstFlags) -> bool {
    match src.get(at + 1) {
        Some(b'{') => true,
        Some(&c) => flags.bare_var_refs && is_var_name_byte(c),
        None => false,
    }
}

/// Push `src[start..end]` as a [`WordPart::Text`], decoding its escapes under
/// the release's grammar when backslash substitution is on (borrowing else).
fn flush_text<'s>(
    parts: &mut Vec<WordPart<'s>>,
    src: &'s [u8],
    start: usize,
    end: usize,
    flags: SubstFlags,
    escapes: EscapeSyntax,
) {
    if end > start {
        let run = &src[start..end];
        let text = if flags.backslashes {
            decode_bytes_in(run, escapes)
        } else {
            Cow::Borrowed(run)
        };
        parts.push(WordPart::Text(text));
    }
}

/// Byte adapter over the one escape decoder ([`backslash_subst_in`]).
///
/// `tcl_syntax::backslash::decode_bytes_in` is the identical adapter one
/// altitude up; both are `&str` round-trips over this crate's decoder, which
/// stays the single owner of the escape grammar. Non-UTF-8 input cannot occur
/// for a well-formed Tcl string rep and borrows through unchanged.
fn decode_bytes_in(raw: &[u8], escapes: EscapeSyntax) -> Cow<'_, [u8]> {
    let Ok(s) = core::str::from_utf8(raw) else {
        return Cow::Borrowed(raw);
    };
    match backslash_subst_in(s, escapes) {
        Cow::Borrowed(b) => Cow::Borrowed(b.as_bytes()),
        Cow::Owned(o) => Cow::Owned(o.into_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use tcl_dialect::{ArrayIndexSyntax, BracedVarStyle};

    /// The oracle sheet behind the expectations in this module. Every message
    /// and every release-split reading below was taken from running these
    /// lines under `tclsh9.0` (9.0.4) and `tclsh8.6` (8.6.16); paste the sheet
    /// into either interpreter to re-derive them without this harness.
    ///
    /// ```text
    /// % set a hi ; set arr(k) v
    /// % subst {${a{b}c}}      9.0: can't read "a{b}c"   8.6: can't read "a{b"
    /// % subst {${a\}b}}       9.0: can't read "a\}b"    8.6: can't read "a\"
    /// % subst {${a}           both: missing close-brace for variable name
    /// % subst {$arr({k})}     9.0: invalid character in array index
    /// %                       8.6: can't read "arr({k})": no such element…
    /// % subst {x[b}           both: missing close-bracket
    /// % subst {$arr(}         both: missing )
    /// % eval {list a "b}      both: missing "
    /// % subst {[list {a]b}]}  both: a\]b          (the `]` in braces is inert)
    /// % subst {[list "a]b"]}  both: a\]b          (…and in a quoted word)
    /// % subst "\[list a\n# ]\nb]"
    /// %                       both: invalid command name "b"
    /// %                                           (…and in a comment)
    /// % subst {price$ x}      both: price$ x      (a `$` with no name is data)
    /// % subst {\$a}           both: $a
    /// ```
    const ORACLE_SHEET: () = ();

    fn config(braced_var: BracedVarStyle, array_index: ArrayIndexSyntax) -> LexerConfig {
        LexerConfig {
            braced_var,
            array_index,
            ..LexerConfig::default()
        }
    }

    fn nine() -> LexerConfig {
        config(BracedVarStyle::Tcl9Nesting, ArrayIndexSyntax::Tcl9)
    }

    fn eight() -> LexerConfig {
        config(BracedVarStyle::FirstClose, ArrayIndexSyntax::Tcl8)
    }

    fn parts(src: &[u8], cfg: LexerConfig) -> Vec<WordPart<'_>> {
        match decompose(src, SubstFlags::default(), cfg) {
            WordBody::Parts(p) => p,
            WordBody::Literal(b) => panic!("expected parts, got literal {b:?}"),
        }
    }

    fn text(bytes: &[u8]) -> WordPart<'_> {
        WordPart::Text(Cow::Borrowed(bytes))
    }

    fn scalar(name: &[u8]) -> WordPart<'_> {
        WordPart::Variable(VarRef { name, index: None })
    }

    #[test]
    fn a_word_with_no_trigger_is_a_borrowed_literal() {
        let src = b"plainword";
        let WordBody::Literal(got) = decompose(src, SubstFlags::default(), nine()) else {
            panic!("expected the literal fast path");
        };
        // Zero-copy: the same allocation, not a copy of it. This is the
        // property `parse_cache` (memory-management.md MM-B.6) rests on.
        assert!(std::ptr::eq(got.as_ptr(), src.as_ptr()));
    }

    #[test]
    fn text_variable_and_command_split_out() {
        assert_eq!(
            parts(b"x${name}y", nine()),
            vec![text(b"x"), scalar(b"name"), text(b"y")]
        );
        assert_eq!(
            parts(b"a[clock seconds]b", nine()),
            vec![text(b"a"), WordPart::Command(b"clock seconds"), text(b"b")]
        );
        assert_eq!(
            parts(b"$arr($i)", nine()),
            vec![WordPart::Variable(VarRef {
                name: b"arr",
                index: Some(vec![scalar(b"i")]),
            })]
        );
    }

    /// A `$` that is not followed by `{` or a name byte is literal text
    /// (`Tcl_ParseVarName`): `subst {price$ x}` is `price$ x` on both oracles.
    #[test]
    fn a_dollar_with_no_name_is_data() {
        assert_eq!(
            decompose(b"price$ x", SubstFlags::default(), nine()),
            WordBody::Literal(b"price$ x")
        );
        // …and an escaped `$` is data too — the escape is folded into the run,
        // not left as a separate part (`subst {\$a}` is `$a`).
        assert_eq!(parts(b"\\$a", nine()), vec![text(b"$a")]);
        assert_eq!(parts(b"\\[a]", nine()), vec![text(b"[a]")]);
    }

    /// Issue #1457's axis: `Tcl_ParseVarName`'s `${…}` close rule moved
    /// between the 8.x family (first literal `}`) and 9.x (brace nesting, with
    /// `\X` inert), and the difference is user-visible in the *name* read.
    #[test]
    fn braced_var_close_rule_follows_the_release() {
        assert_eq!(parts(b"${a{b}c}", nine()), vec![scalar(b"a{b}c")]);
        assert_eq!(
            parts(b"${a{b}c}", eight()),
            vec![scalar(b"a{b"), text(b"c}")]
        );
        assert_eq!(parts(b"${a\\}b}", nine()), vec![scalar(b"a\\}b")]);
        // 8.x closes at the first `}`; the rest of the template is text, and
        // its escapes decode with it (`\}` → `}`).
        assert_eq!(
            parts(b"${a\\}b}", eight()),
            vec![scalar(b"a\\"), text(b"b}")]
        );
    }

    /// An unterminated `${…}` is C's error, not a name that runs to end of
    /// input. The 9.x nesting rule also *widens* what counts as unterminated.
    #[test]
    fn unterminated_braced_var_is_a_parse_error() {
        let err = WordPart::ParseError(MISSING_CLOSE_BRACE_FOR_VAR);
        for cfg in [nine(), eight()] {
            assert_eq!(parts(b"${abc", cfg), vec![err.clone()]);
        }
        assert_eq!(parts(b"${a{b}", nine()), vec![err.clone()]);
        assert_eq!(parts(b"${a{b}", eight()), vec![scalar(b"a{b")]);
    }

    /// Issue #1732's axis: Tcl 9 rejects raw `{`, `"`, `(`, `}` written in an
    /// array index; Tcl 8 passed them through. The mask applies to *source*
    /// bytes only — an escape or a substitution result is legal on both.
    #[test]
    fn array_index_source_mask_follows_the_release() {
        assert_eq!(
            parts(b"$arr({k})", nine()),
            vec![WordPart::ParseError(INVALID_CHARACTER_IN_ARRAY_INDEX)]
        );
        assert_eq!(
            parts(b"$arr({k})", eight()),
            vec![WordPart::Variable(VarRef {
                name: b"arr",
                index: Some(vec![text(b"{k}")]),
            })]
        );
        for src in [&b"$a(\\{k\\})"[..], b"$a(${k})", b"$a([format \\{])"] {
            assert!(
                !parts(src, nine())
                    .iter()
                    .any(|p| matches!(p, WordPart::ParseError(_))),
                "Tcl 9 accepts escaped/substituted index source: {src:?}"
            );
        }
    }

    /// The three unterminated forms C names, with its exact messages. The
    /// parts scanned *before* the failure are kept: `subst {[side][b}` runs
    /// `side` and then reports `missing close-bracket` on both oracles.
    #[test]
    fn unterminated_constructs_carry_c_tcls_exact_message() {
        assert_eq!(
            parts(b"x[b", nine()),
            vec![text(b"x"), WordPart::ParseError(MISSING_CLOSE_BRACKET)]
        );
        assert_eq!(
            parts(b"[side][b", nine()),
            vec![
                WordPart::Command(b"side"),
                WordPart::ParseError(MISSING_CLOSE_BRACKET)
            ]
        );
        assert_eq!(
            parts(b"$arr(", nine()),
            vec![WordPart::ParseError(MISSING_PAREN)]
        );
        // A substituted `[…]` is a script, so C recurses into it rather than
        // hunting for the `]`: an error inside an unterminated bracket is what
        // surfaces. `subst [format {[set y $%sa%sb]} "{" "{"]` reports
        // `missing close-brace for variable name` on both oracles.
        assert_eq!(
            parts(b"[set y ${a{b]", nine()),
            vec![WordPart::ParseError(MISSING_CLOSE_BRACE_FOR_VAR)]
        );
        assert_eq!(MISSING_QUOTE, "missing \"");
        assert_eq!(MISSING_CLOSE_BRACE, "missing close-brace");
    }

    /// The `]` search is brace-, quote- and comment-aware, because the
    /// substituted text is a *script*. Each of the three private copies this
    /// module replaced got at least one of these wrong.
    #[test]
    fn the_bracket_search_respects_braces_quotes_and_comments() {
        assert_eq!(
            parts(b"[list {a]b}]", nine()),
            vec![WordPart::Command(b"list {a]b}")]
        );
        assert_eq!(
            parts(b"[list \"a]b\"]", nine()),
            vec![WordPart::Command(b"list \"a]b\"")]
        );
        assert_eq!(
            parts(b"[list a\n# ]\nb]", nine()),
            vec![WordPart::Command(b"list a\n# ]\nb")]
        );
        // Nesting and `\]` still work.
        assert_eq!(
            parts(b"[a [b] c]", nine()),
            vec![WordPart::Command(b"a [b] c")]
        );
        assert_eq!(parts(b"[a\\]b]", nine()), vec![WordPart::Command(b"a\\]b")]);
    }

    /// A literal run decodes under the emulated release's escape grammar
    /// (issue #1479): TIP 388 capped `\x` at two hex digits from 8.6 and added
    /// `\U`, so `\x4142` is `B` under 8.5 and `A42` from 8.6.
    #[test]
    fn literal_runs_decode_under_the_releases_escape_grammar() {
        let with = |escapes| LexerConfig { escapes, ..nine() };
        for (escapes, hex, wide) in [
            (EscapeSyntax::Tcl84, &b"B"[..], &b"U0001F600"[..]),
            (EscapeSyntax::Tcl86, b"A42", "\u{FFFD}".as_bytes()),
            (EscapeSyntax::Tcl90, b"A42", "\u{1F600}".as_bytes()),
        ] {
            assert_eq!(parts(b"\\x4142", with(escapes)), vec![text(hex)]);
            assert_eq!(parts(b"\\U0001F600", with(escapes)), vec![text(wide)]);
        }
    }

    /// Each substitution kind switches independently — `subst`'s `-no*`
    /// options. With backslashes off a run borrows through undecoded.
    #[test]
    fn each_substitution_kind_switches_independently() {
        let no_vars = SubstFlags {
            vars: false,
            ..SubstFlags::default()
        };
        assert_eq!(
            decompose(b"$x\\t", no_vars, nine()),
            WordBody::Parts(vec![text(b"$x\t")])
        );
        let no_bs = SubstFlags {
            backslashes: false,
            ..SubstFlags::default()
        };
        assert_eq!(
            decompose(b"a\\tb$x", no_bs, nine()),
            WordBody::Parts(vec![text(b"a\\tb"), scalar(b"x")])
        );
    }

    /// The compiled-word flavour (`tcl-vm`'s `PUSH` operands): the compiler
    /// has already inlined or normalised every real variable reference, so a
    /// surviving bare `$` is data while `${…}` and `[…]` still substitute.
    #[test]
    fn compiled_word_flags_keep_a_bare_dollar_literal() {
        assert_eq!(
            decompose(b"x$y", SubstFlags::compiled_word(), nine()),
            WordBody::Literal(b"x$y")
        );
        assert_eq!(
            decompose(b"x${y}", SubstFlags::compiled_word(), nine()),
            WordBody::Parts(vec![text(b"x"), scalar(b"y")])
        );
        // …and the escapes of that literal run are still decoded, which is
        // the whole of issue #1646: `string length "x\$y"` is 3, not 4.
        assert_eq!(
            decompose(b"x\\$y", SubstFlags::compiled_word(), nine()),
            WordBody::Parts(vec![text(b"x$y")])
        );
    }

    #[test]
    fn quoted_word_close_reports_missing_quote() {
        assert_eq!(quoted_word_close("\"abc\" rest", 0), Ok(4));
        assert_eq!(quoted_word_close("\"abc", 0), Err(MISSING_QUOTE));
        // A `"` inside a complete `[…]` of the word does not close it.
        assert_eq!(quoted_word_close("\"a[foo \"b\"]c\"", 0), Ok(12));
    }

    #[test]
    fn scan_var_ref_leaves_the_index_raw() {
        let cfg = nine();
        let got = scan_var_ref(b"$arr($i)x", 0, cfg).unwrap().unwrap();
        assert_eq!(got.name, b"arr");
        assert_eq!(got.index, Some(&b"$i"[..]));
        assert_eq!(got.next, 8);
        // Colon runs belong to the name (`namespace` separators).
        let got = scan_var_ref(b"$a:::b rest", 0, cfg).unwrap().unwrap();
        assert_eq!(got.name, b"a:::b");
        assert_eq!(got.next, 6);
        // Not a reference at all.
        assert_eq!(scan_var_ref(b"$ x", 0, cfg), Ok(None));
        // A trailing `$` has no name behind it, so it is data, not an error.
        assert_eq!(scan_var_ref(b"a$", 1, cfg), Ok(None));
    }

    /// Regression coverage for issue #996: the index parse recurses once per
    /// `$name(index)` level, reachable from ordinary `subst` with no special
    /// syntax. The same construct overflowed a 256 KiB native stack between
    /// depth 100-150. Past `MAX_INDEX_DEPTH` the index is kept as literal
    /// text; the assertion is that the scan returns at all.
    #[test]
    fn deeply_nested_array_index_survives() {
        const DEPTH: usize = 5000;
        let mut src = String::from("$a0");
        for i in 0..DEPTH {
            src.push('(');
            write!(src, "$a{}", i + 1).expect("writing to a String cannot fail");
        }
        src.push('1');
        for _ in 0..DEPTH {
            src.push(')');
        }
        let _ = decompose(src.as_bytes(), SubstFlags::default(), nine());
    }

    /// …while realistic nesting is still scanned in full: the `)` search steps
    /// over a nested `$name(…)`'s own parens, so each level closes on its own
    /// (`set c(1) inner; set b(inner) mid; set a(mid) outer; set x $a($b($c(1)))`
    /// yields `outer` under `tclsh9.0`).
    #[test]
    fn moderate_nesting_is_scanned_in_full() {
        assert_eq!(
            parts(b"$a($b($c(1)))", nine()),
            vec![WordPart::Variable(VarRef {
                name: b"a",
                index: Some(vec![WordPart::Variable(VarRef {
                    name: b"b",
                    index: Some(vec![WordPart::Variable(VarRef {
                        name: b"c",
                        index: Some(vec![text(b"1")]),
                    })]),
                })]),
            })]
        );
    }

    #[test]
    fn oracle_sheet_is_recorded() {
        let () = ORACLE_SHEET;
    }
}
