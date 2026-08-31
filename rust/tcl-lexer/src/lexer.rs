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

//! Streaming Tcl lexer.
//!
//! Handles every top-level Tcl construct:
//!
//! - **EOF handling** — emits a trailing ghost `EOL` (once) when the
//!   source does not already end with an EOL token.
//! - **SEP** — runs of horizontal whitespace (`' '`, `\t`, `\r`, VT, FF).
//!   `\r` is horizontal whitespace in Tcl, not an EOL.
//! - **EOL** — runs of EOL characters (`\n`, `;`) interleaved with
//!   horizontal whitespace.
//! - **COMMENT** — `#` at command start, scanned to the next `\n`
//!   (exclusive). Backslash-newline continuation inside comments is
//!   not handled yet; an input whose comment contains a `\` is
//!   reported as a `SyntaxError` so the differential harness can
//!   filter it.
//! - **ESC** — runs of characters that are neither whitespace nor EOL
//!   nor one of the special characters. Terminated by
//!   `$` or `[` so that variable and command substitutions
//!   dispatch on the next iteration.
//! - **VAR** — variable substitution in all four Tcl forms:
//!   `$name`, `$ns::var` (namespace-separated), `${name}` (braced),
//!   and `$arr(idx)` (array index with nested parens and embedded
//!   `${...}` support). A bare `$` with no name following is emitted
//!   as an `STR` token whose span covers just the `$`. Unterminated
//!   `${` and `$arr(` tokenize best-effort (warning collection reports
//!   them as diagnostics).
//! - **CMD** — command substitution `[…]`. The scanner tracks four
//!   pieces of state while inside the command body: outer bracket
//!   nesting (`level`), brace nesting (`blevel`), whether we are
//!   inside a `"…"` quoted sub-region (`in_quotes`), and Tcl comment
//!   state (`in_comment` / `at_command_start`). A `[` increments
//!   `level` only when not inside braces, quotes, or a comment; a `]`
//!   closes the command only when the outer bracket is the innermost
//!   nesting and it is not commented out. Backslash escapes consume
//!   two characters (CRLF counted as one line advance); `${…}`
//!   sub-scans exist to stop a `)` or `}` inside a braced variable
//!   name from fooling the counter. Unterminated `[` tokenizes
//!   best-effort.
//! - **STR** — braced strings `{…}`. Emitted when a `{` appears
//!   at a word boundary (the previous token was `EOL` / `SEP` / `STR`
//!   / `EXPAND`). The body is scanned with balanced `{` / `}` counting;
//!   backslash sequences consume two characters as a pair (backslash is
//!   inert inside braces — the backslash and the following character
//!   are retained literally in the token text). A `{` that is NOT at a
//!   word boundary is a regular word character in the enclosing `ESC`
//!   token. Unterminated `{` tokenizes best-effort.
//! - **Quoted ESC** — `"…"` quoted strings emit `ESC` tokens
//!   carrying the `in_quote = true` flag for the duration of the
//!   quoted run. The lexer keeps an `in_quote: bool` field that is
//!   toggled on the opening and closing `"`; while it is set, the
//!   dispatch ignores separators, EOL characters, `#`, and `{` (all
//!   of which become literal content) and handles only the four
//!   active constructs `$`, `[`, `\`, and `"`. The opening `"` is
//!   captured in the first emitted `ESC`'s span (with
//!   `token_text` stripping it), not as a separate token; if the
//!   body begins with `$` or `[` the first `ESC` is an empty-body
//!   token whose span is extended to cover the terminator. Sub-tokens
//!   (`VAR`, `CMD`) emitted inside a quoted run carry
//!   `in_quote = true`; the **last** `ESC` before the closing `"` and
//!   the closing `"` itself (emitted as a possibly empty `ESC`) reset
//!   `in_quote` to `false` before the token is returned. A `"` mid-word
//!   (not at a word boundary) is a regular character inside the
//!   enclosing `ESC`.
//!
//! ### Architecture
//!
//! Tokens are pure data: a `TokenType`, a byte [`Span`], and an
//! `in_quote` flag — nothing more. Text and `(line, character,
//! offset)` positions are resolved on demand via the lexer's
//! [`SourceMap`]. The lexer tracks only a byte `pos: u32` and a small
//! amount of behavioural state (`at_command_start`, `last_kind`,
//! `done`); there is no incremental column bookkeeping.
//!
//! Every positional entity (Tokens now, IR and CFG nodes elsewhere)
//! carries only a span, and a single `SourceMap` per document is the
//! canonical place that resolves spans to text and positions.
//!
//! ### Offsets and columns
//!
//! [`SourcePosition::offset`] is a byte offset. The `character` field
//! is **byte offset within the line** (`col = offset - line_start`).
//! ASCII parity is exact; non-ASCII drifts from the LSP UTF-16
//! contract — multi-byte column parity is not yet handled.
//!
//! ### Not yet implemented
//!
//! - backslash escapes and line continuation; warning collection
//!   (which will turn the best-effort recovery of unterminated
//!   `${` / `$arr(` into proper diagnostics); ghost character
//!   insertion for error recovery. "Ghost" is our term of art
//!   (chosen over "synthetic" / "virtual" to avoid collisions with
//!   Rust vocabulary — `virtual` is a reserved keyword) for tokens
//!   and characters that exist in the token stream without
//!   corresponding bytes in the source buffer.
//! - sub-lexing support for nested constructs; UTF-16 column
//!   parity; `LineIndex::from_rope_slice` adapter
//!
//! [`Span`]: crate::Span
//! [`SourceMap`]: crate::SourceMap
//! [`SourcePosition`]: crate::SourcePosition
//! [`SourcePosition::offset`]: crate::SourcePosition#structfield.offset

use tcl_core_types::RecursionLimit;
use tcl_dialect::BracedVarStyle;
use thiserror::Error;

use crate::source_map::SourceMap;
use crate::span::Span;
use crate::tokens::{Token, TokenType};

/// Cap on `$name(…)` array-index nesting depth that
/// [`Lexer::scan_array_index_body`]/[`Lexer::skip_var_in_index`] will
/// recurse into. The two are mutually recursive with no natural bound —
/// `$a($b($c(...)))` recurses one native-stack frame group per `(` — so
/// pathologically deep input (e.g. generated/minified Tcl) could otherwise
/// abort the process with an uncatchable stack overflow. 64 is far past any
/// array-index nesting real Tcl code uses; see
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
const MAX_ARRAY_INDEX_DEPTH: RecursionLimit = RecursionLimit(64);

/// Configuration for the Tcl lexer.
///
/// Holds dialect-specific flags and sub-lexing offsets as explicit
/// fields passed at construction time; the `Default` values are the
/// non-strict, Tcl-8.5+ defaults with no sub-lexing offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LexerConfig {
    /// When true, `{*}` at a word boundary followed by a
    /// non-separator emits an [`EXPAND`](TokenType::Expand) token.
    /// True for Tcl 8.5+ (the default); false for Tcl 8.4 and
    /// iRules dialects.
    pub expand_syntax: bool,
    /// When true, the F5 implicit word break (the R-rules of
    /// `docs/design/bigip-irule-parser-measurements.md` §1, §3): a word
    /// that **started** with `{` or `"` ends at its matching close
    /// delimiter, and any following character that is not whitespace or
    /// a command terminator begins a new word — a zero-width ghost SEP
    /// token is injected so the segmenter sees two words, and no
    /// diagnostic is emitted (R6). Fires repeatedly and in every word
    /// position including the command name (R4); never after a bare
    /// word, `$var`, `${name}`, or `[cmd]` (R5). An `f5-tcl` trunk
    /// axis — measured identical in TMM iRules, tmsh cli scripts, and
    /// iApp implementations (§4a) — carried under its historical name.
    pub irules_brace_separator: bool,
    /// The F5 brace-line continuation axis (the N-rules of
    /// `docs/design/bigip-irule-parser-measurements.md` §2): under
    /// [`BraceLineContinuation::Continues`], a newline whose next line's
    /// first non-whitespace character is `{` does not terminate the
    /// command — it lexes as a SEP instead of an EOL, unconditionally
    /// (N2) and at any nesting depth (N3, via body re-lexing under the
    /// same config). Blank, whitespace-only, and comment lines still
    /// terminate (N4); backslash-newline handling is unchanged. An
    /// `f5-tcl` trunk axis.
    pub brace_line_continuation: tcl_dialect::BraceLineContinuation,
    /// When true, certain unterminated constructs (missing
    /// close-brace, missing close-bracket, extra chars after
    /// close-quote/brace) are reported as `LexError` instead of
    /// best-effort warnings. Used by the VM's compilation path.
    pub strict_quoting: bool,
    /// How a `${…}` variable name is delimited — see [`BracedVarStyle`].
    pub braced_var: BracedVarStyle,
    /// Byte offset to add to every `SourcePosition.offset`
    /// produced by the lexer. Used when sub-lexing a body
    /// extracted from a parent token.
    pub base_offset: u32,
    /// Line number to add to every `SourcePosition.line`.
    pub base_line: u32,
    /// Column to add to the first line's character values.
    pub base_col: u32,
    /// What a UTF-8 byte-order mark at byte 0 of this buffer *is* — script
    /// prologue or ordinary content.  See [`LeadingBom`].
    pub leading_bom: LeadingBom,
    /// Which backslash-escape grammar text lexed under this config decodes
    /// with — see [`tcl_dialect::EscapeSyntax`].
    ///
    /// The lexer itself never decodes an escape: `\X` is an inert two-byte
    /// pair for token-boundary purposes, and no release's escape grammar moves
    /// a boundary (every form's payload is hex or octal digits, which separate
    /// nothing). It rides here because the consumers that *do* decode — a
    /// runtime's word builder, its `subst` — already thread a `LexerConfig`
    /// from the dialect profile, so this is the same seam rather than a second
    /// one.
    pub escapes: tcl_dialect::EscapeSyntax,
}

/// How the lexer reads a UTF-8 byte-order mark (U+FEFF) sitting at byte 0 of
/// the buffer it was handed.
///
/// [`LeadingBom::Content`] is the default and the only correct answer for any
/// buffer that is not a whole file: the lexer is shared with the VM's `eval`
/// path, where a mark at the head of a string is ordinary data. Only a
/// *file*-analysis entry point may choose [`LeadingBom::Skip`], and only when
/// the dialect's script reader does the same
/// (`tcl_dialect::LexerGrammar::script_skips_leading_bom` — Tcl 9's `source`
/// skips it, Tcl 8.x's does not). See issue #1218.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LeadingBom {
    /// Lex the mark as the first word's opening characters, like any other
    /// character.
    #[default]
    Content,
    /// Skip it, by starting the scan past it — so every token keeps its true
    /// byte offset and the client-visible line/column maths on line 0 is
    /// unchanged. The mark stays in the document; it is simply not part of
    /// any token.
    Skip,
}

impl Default for LexerConfig {
    fn default() -> Self {
        Self {
            expand_syntax: true,
            irules_brace_separator: false,
            brace_line_continuation: tcl_dialect::BraceLineContinuation::Terminates,
            strict_quoting: false,
            braced_var: BracedVarStyle::Tcl9Nesting,
            base_offset: 0,
            base_line: 0,
            base_col: 0,
            leading_bom: LeadingBom::Content,
            escapes: tcl_dialect::EscapeSyntax::Tcl90,
        }
    }
}

/// The UTF-8 encoding of U+FEFF, the byte-order mark.
pub const UTF8_BOM: &str = "\u{FEFF}";

impl LexerConfig {
    /// Build a config from a dialect profile's [`LexerGrammar`] — the
    /// dialect-derived fields come from the grammar; the call-site knobs
    /// (strict quoting, sub-lexing offsets) keep their defaults.
    #[must_use]
    pub fn from_grammar(grammar: tcl_dialect::LexerGrammar) -> Self {
        Self {
            expand_syntax: grammar.expand_syntax,
            irules_brace_separator: grammar.irules_brace_separator,
            brace_line_continuation: grammar.brace_line_continuation,
            braced_var: grammar.braced_var,
            escapes: grammar.escapes,
            ..Self::default()
        }
    }

    /// Build a config preset for the given dialect name, from the dialect
    /// profile's grammar (`tcl_dialect::DialectProfile::grammar` — the
    /// single source for `{*}` expansion, the iRules `}{` separator, and
    /// the `${…}` delimiting rule):
    ///
    /// * `expand_syntax` — true for Tcl 8.5+ runtimes and dialects that
    ///   embed one (Expect, the EDA flavours, `bpf`, `spectcl`). False for
    ///   Tcl 8.4 and for **every** F5 dialect: measurement
    ///   (`docs/design/bigip-irule-parser-measurements.md` §4a) showed
    ///   `f5-tmsh` and `f5-iapps` are 8.4.6 forks like `f5-irules`, not the
    ///   8.5 embeds the pre-#1631 catalogue assumed, and on all three
    ///   `{*}$l` lexes as a literal `*` plus the unexpanded word.
    /// * `irules_brace_separator` — true for every dialect on the `f5-tcl`
    ///   trunk: `f5-irules`, `f5-tmsh` and `f5-iapps` all select
    ///   `GRAMMAR_F5_TCL`, and §4a measured the implicit word break
    ///   byte-identically in all three. The field keeps its historical
    ///   name.
    /// * `braced_var` — the `${…}` delimiting rule:
    ///   [`BracedVarStyle::FirstClose`] for every 8.x-runtime dialect
    ///   (8.4–8.6, the F5 dialects — tmsh included — EDA, and Expect),
    ///   [`BracedVarStyle::Tcl9Nesting`] for 9.x runtimes (`bpf` embeds
    ///   Tcl 9.0) and unversioned Tcl.
    ///
    /// Unknown dialect names resolve to the permissive fallback profile
    /// (modern-Tcl semantics) so a typo in a workspace's `languageId`
    /// doesn't change parsing behaviour.
    #[must_use]
    pub fn for_dialect(dialect: &str) -> Self {
        Self::from_grammar(
            tcl_dialect::DialectProfile::find(dialect)
                .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl)
                .grammar,
        )
    }

    /// [`Self::from_grammar`] for the entry point that lexes a **whole file**:
    /// identical, plus [`Self::leading_bom`] set from the grammar's
    /// `script_skips_leading_bom`.
    ///
    /// Use this exactly where a Tcl runtime would `source` the buffer — the
    /// top of a document analysis. Every nested re-lex (a body, an `eval`
    /// argument, a VM string) must keep using [`Self::from_grammar`]: a BOM
    /// there is data, not a file prologue.
    #[must_use]
    pub fn for_file_grammar(grammar: tcl_dialect::LexerGrammar) -> Self {
        Self {
            leading_bom: if grammar.script_skips_leading_bom {
                LeadingBom::Skip
            } else {
                LeadingBom::Content
            },
            ..Self::from_grammar(grammar)
        }
    }

    /// [`Self::for_dialect`] for the entry point that lexes a **whole file** —
    /// the by-name twin of [`Self::for_file_grammar`].
    ///
    /// Every LSP provider that re-segments the raw document text has such an
    /// entry point, and each must use this one rather than
    /// [`Self::for_dialect`]: otherwise a leading mark lexes into the first
    /// word, so the first command's semantic token spans the mark, its body
    /// fold is lost (the marked name resolves to no registry command), and its
    /// document links / inlay hints / references miss (issue #1243).
    #[must_use]
    pub fn for_file_dialect(dialect: &str) -> Self {
        Self::for_file_grammar(
            tcl_dialect::DialectProfile::find(dialect)
                .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl)
                .grammar,
        )
    }

    /// This config demoted to a **nested** re-lex: identical, except a leading
    /// byte-order mark is ordinary content again.
    ///
    /// A whole-file config may skip a mark at byte 0 because that is the
    /// script's prologue; a mark at the head of a nested body slice, an `eval`
    /// argument, or a VM string is data. A provider that threads one config
    /// through its own body recursion calls this below the top level, so the
    /// file rule cannot leak into a nested slice (issue #1243).
    #[must_use]
    pub fn nested(self) -> Self {
        Self {
            leading_bom: LeadingBom::Content,
            ..self
        }
    }

    /// [`Self::nested`] applied everywhere but the top level of a provider's
    /// own body recursion — the shape those providers all want, stated once.
    #[must_use]
    pub fn at_depth(self, depth: u32) -> Self {
        if depth == 0 { self } else { self.nested() }
    }
}

/// Errors produced by the Tcl lexer.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LexError {
    /// A syntax error detected in strict-quoting mode.
    #[error("{message}")]
    SyntaxError {
        /// Human-readable message.
        message: String,
    },
}

/// A non-fatal warning collected during lexing. The analyser
/// harvests these to produce LSP diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexWarning {
    /// Byte offset in the source where the issue was detected.
    pub offset: u32,
    /// Human-readable message.
    pub message: String,
}

/// Streaming Tcl lexer.
///
/// Produces [`Token`]s via the [`Iterator`] impl. Each token carries
/// only a [`Span`]; text and positions are resolved through the
/// lexer's [`SourceMap`], available via [`Lexer::source_map`] and
/// [`Lexer::into_source_map`].
#[derive(Debug)]
pub struct Lexer<'src> {
    source_map: SourceMap<'src>,
    /// Byte offset of the next character to consume.
    pos: u32,
    /// Whether the next token starts a new command. Set on construction
    /// and after every EOL; preserved across SEP tokens.
    at_command_start: bool,
    /// Whether we are currently inside a `"…"` quoted string.
    in_quote: bool,
    /// Ghost SEP token injected by `parse_brace` / `parse_quoted` when
    /// `irules_brace_separator` is set and the close delimiter of a
    /// brace- or quote-started word is immediately followed by a
    /// non-separator (F5 R2, measurements §1).
    pending_sep: Option<Token>,
    /// Non-fatal warnings collected during lexing (unterminated
    /// braces, extra chars after close-quote, etc.).
    warnings: Vec<LexWarning>,
    /// Kind of the most recently emitted token. Used to decide whether
    /// EOF needs a trailing ghost EOL and to compute
    /// [`Lexer::is_newword`].
    last_kind: TokenType,
    /// Once true, [`Iterator::next`] returns `None`.
    done: bool,
    config: LexerConfig,
    /// Zero-width "ghost" closing delimiters injected for error
    /// recovery, keyed by source byte offset (value is the delimiter
    /// byte, e.g. `b']'`).  When the scanner reaches a ghost offset it
    /// *sees* the ghost byte before the real one; consuming a ghost is
    /// zero-width (it removes the entry without advancing `pos`), so an
    /// unterminated `[foo bar` re-lexes as a terminated command without
    /// shifting any downstream offsets.  Empty on the normal lexing path.
    ghosts: std::collections::BTreeMap<u32, u8>,
}

impl<'src> Lexer<'src> {
    /// Build a lexer over `source`, scanning the source once to build
    /// the internal [`SourceMap`]. Use [`Lexer::with_source_map`]
    /// instead when a `SourceMap` already exists (e.g. cached on a
    /// document buffer) to avoid re-scanning.
    #[must_use]
    pub fn new(source: &'src str) -> Self {
        Self::with_source_map(SourceMap::new(source), LexerConfig::default())
    }

    /// Build a lexer over a whole source buffer with an explicit dialect
    /// configuration.
    #[must_use]
    pub fn with_config(source: &'src str, config: LexerConfig) -> Self {
        Self::with_source_map(SourceMap::new(source), config)
    }

    /// Build a lexer with a pre-built `SourceMap` and custom config.
    ///
    /// The caller is responsible for ensuring the `SourceMap` was
    /// built from the same source string.
    #[must_use]
    pub fn with_source_map(source_map: SourceMap<'src>, config: LexerConfig) -> Self {
        // A leading byte-order mark is skipped by *starting past it*, so every
        // token keeps its true byte offset (issue #1218). Guarded on the flag
        // as well as the bytes: the same lexer serves the VM's `eval`, where a
        // BOM at the head of a string is ordinary data.
        let pos = if config.leading_bom == LeadingBom::Skip
            && source_map.source().starts_with(UTF8_BOM)
        {
            u32::try_from(UTF8_BOM.len()).unwrap_or(0)
        } else {
            0
        };
        Self {
            source_map,
            pos,
            at_command_start: true,
            in_quote: false,
            pending_sep: None,
            warnings: Vec::new(),
            // Start in "last kind was EOL" so an empty source produces
            // zero tokens rather than a lone ghost trailing EOL.
            last_kind: TokenType::Eol,
            done: false,
            config,
            ghosts: std::collections::BTreeMap::new(),
        }
    }

    /// Attach zero-width ghost closing delimiters (offset → delimiter
    /// byte) for error recovery; see the `ghosts` field.  Builder form,
    /// chains after [`Lexer::new`] / [`Lexer::with_source_map`].
    #[must_use]
    pub fn with_ghosts(mut self, ghosts: std::collections::BTreeMap<u32, u8>) -> Self {
        self.ghosts = ghosts;
        self
    }

    /// Treat `source` as already being the *interior* of an open
    /// double-quoted string, from byte 0: only `$`, `[`, and `\` escapes
    /// are special, and everything else — including `{` / `}`,
    /// whitespace, and `#` — is ordinary literal content, exactly like
    /// [`Self::parse_quoted`]'s own in-quote dispatch. No top-level
    /// word-splitting or brace-quoting is applied at all.
    ///
    /// For scanning already-extracted word/value text (a `set` value, a
    /// proc-arg default, …) whose own enclosing quotes have already been
    /// stripped by whatever produced the text — re-tokenising it with the
    /// ordinary top-level rules would wrongly treat an embedded `{…}` run
    /// as a fresh brace-quoted (non-substituting) word, when it was
    /// actually just literal content the original quoted context already
    /// carried through unchanged. Builder form, chains after
    /// [`Lexer::new`] / [`Lexer::with_source_map`].
    ///
    /// An unterminated quote is expected here (there is no real closing
    /// delimiter to find) and never surfaces as an error: same
    /// best-effort behaviour as any other unterminated quoted string
    /// (see [`Self::parse_quoted`]).
    #[must_use]
    pub fn as_quoted_body(mut self) -> Self {
        self.in_quote = true;
        self
    }

    /// The ghost delimiter byte active at `offset`, if any.
    fn ghost_at(&self, offset: u32) -> Option<u8> {
        if self.ghosts.is_empty() {
            return None;
        }
        self.ghosts.get(&offset).copied()
    }

    /// Borrow the lexer's source map without consuming the lexer.
    #[must_use]
    pub fn source_map(&self) -> &SourceMap<'src> {
        &self.source_map
    }

    /// Consume the lexer and return its source map — handy when the
    /// caller wants the same `SourceMap` to resolve the tokens later
    /// without rebuilding it.
    #[must_use]
    pub fn into_source_map(self) -> SourceMap<'src> {
        self.source_map
    }

    /// Borrow the warnings collected during lexing.
    #[must_use]
    pub fn warnings(&self) -> &[LexWarning] {
        &self.warnings
    }

    /// Consume the lexer and return its warnings.
    #[must_use]
    pub fn into_warnings(self) -> Vec<LexWarning> {
        self.warnings
    }

    /// Collect every token, including `SEP` and `EOL`, into a `Vec`.
    ///
    /// # Errors
    ///
    /// Returns [`LexError::SyntaxError`] when strict-quoting mode
    /// is active and the input triggers one of the raise-sites
    /// (`parse_var`, `parse_command`, `parse_brace`, `parse_quoted`).
    pub fn tokenise_all(self) -> Result<Vec<Token>, LexError> {
        self.collect()
    }

    /// Collect every token alongside the non-fatal warnings
    /// accumulated during lexing.
    ///
    /// Like `tokenise_all` but also surfaces the `(offset, message)`
    /// warnings, so callers can merge them into their own diagnostics.
    /// Without these, editors lose recoverable-syntax diagnostics
    /// (extra chars after close-brace / close-quote, unterminated
    /// strings).
    ///
    /// # Errors
    ///
    /// Returns the first [`LexError`] encountered while scanning;
    /// in that case no warnings are surfaced (the error itself
    /// subsumes any diagnostics).
    pub fn tokenise_all_with_warnings(mut self) -> Result<(Vec<Token>, Vec<LexWarning>), LexError> {
        let mut tokens = Vec::new();
        for result in self.by_ref() {
            tokens.push(result?);
        }
        Ok((tokens, self.warnings))
    }

    #[inline]
    fn source(&self) -> &'src str {
        self.source_map.source()
    }

    #[inline]
    fn current_byte(&self) -> Option<u8> {
        // A ghost closing delimiter at `pos` is seen before the real
        // byte.
        if let Some(g) = self.ghost_at(self.pos) {
            return Some(g);
        }
        self.source().as_bytes().get(self.pos as usize).copied()
    }

    /// Return the character starting at `self.pos`, or `None` at EOF.
    #[inline]
    fn current_char(&self) -> Option<char> {
        if let Some(g) = self.ghost_at(self.pos) {
            return Some(char::from(g));
        }
        self.source()
            .get(self.pos as usize..)
            .and_then(|s| s.chars().next())
    }

    /// Emit a warning (non-strict) or return an error (strict).
    /// Called from `parse_brace`, `parse_command`, and
    /// `parse_quoted` for unterminated constructs and
    /// extra-chars-after-close violations.
    fn warn_or_error(&mut self, message: &str) -> Result<(), LexError> {
        if self.config.strict_quoting {
            Err(LexError::SyntaxError {
                message: message.to_owned(),
            })
        } else {
            self.warnings.push(LexWarning {
                offset: self.pos,
                message: message.to_owned(),
            });
            Ok(())
        }
    }

    /// Build a token whose span covers `start_offset..self.pos`
    /// with `content_offset = 0` (no prefix delimiter to strip).
    /// Used for `Sep`, `Eol`, `Comment`, and plain `Esc`.
    fn make_token(&self, kind: TokenType, start_offset: u32) -> Token {
        Token::new(kind, Span::new(start_offset, self.pos))
    }

    fn parse_sep(&mut self) -> Token {
        let start_offset = self.pos;
        while let Some(byte) = self.current_byte() {
            if !is_horizontal_whitespace_byte(byte) {
                break;
            }
            self.pos += 1; // All SEP characters are ASCII.
        }
        self.make_token(TokenType::Sep, start_offset)
    }

    /// Whether the `\n` at `self.pos` continues the current command under
    /// the F5 N-rules (measurements §2): the axis is on and the next
    /// line's first non-whitespace character is `{`. Purely lexical —
    /// independent of the command's identity, arity, or completeness
    /// (N2). A blank, whitespace-only, or comment line answers `false`
    /// (N4): the byte the horizontal-whitespace scan stops on is then a
    /// `\n`, `#`, or end-of-input rather than `{`.
    fn newline_continues_command(&self) -> bool {
        if !self.config.brace_line_continuation.continues() {
            return false;
        }
        debug_assert_eq!(self.current_byte(), Some(b'\n'));
        let bytes = self.source().as_bytes();
        let mut index = self.pos as usize + 1;
        while let Some(&byte) = bytes.get(index) {
            if is_horizontal_whitespace_byte(byte) {
                index += 1;
            } else {
                return byte == b'{';
            }
        }
        false
    }

    /// Consume the continuation newline plus the next line's leading
    /// horizontal whitespace as a single SEP token (N1): the command
    /// does not terminate, and the `{` that follows opens an ordinary
    /// braced word at a fresh word boundary.
    fn parse_brace_line_continuation_sep(&mut self) -> Token {
        let start_offset = self.pos;
        self.pos += 1; // the newline
        while let Some(byte) = self.current_byte() {
            if !is_horizontal_whitespace_byte(byte) {
                break;
            }
            self.pos += 1;
        }
        self.make_token(TokenType::Sep, start_offset)
    }

    fn parse_eol(&mut self) -> Token {
        let start_offset = self.pos;
        // Consume a run mixing EOL characters and horizontal
        // whitespace in a single token.
        while let Some(byte) = self.current_byte() {
            if !is_horizontal_whitespace_byte(byte) && !is_eol_byte(byte) {
                break;
            }
            self.pos += 1;
        }
        self.make_token(TokenType::Eol, start_offset)
    }

    fn parse_comment(&mut self) -> Token {
        let start_offset = self.pos;
        self.pos += 1; // consume the leading '#'
        while let Some(ch) = self.current_char() {
            match ch {
                '\n' => break,
                '\\' => {
                    // Consume backslash + next char as a pair.
                    // `\<newline>` continues the comment to the
                    // next line (matching C Tcl behaviour).
                    self.pos += 1;
                    match self.current_char() {
                        Some(esc @ ('\n' | '\r')) => {
                            self.pos += 1;
                            if esc == '\r' && self.current_byte() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        Some(esc) => {
                            self.pos += u32::try_from(esc.len_utf8()).expect("char len fits u32");
                        }
                        None => {}
                    }
                }
                _ => {
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }
        self.make_token(TokenType::Comment, start_offset)
    }

    fn parse_esc(&mut self) -> Token {
        let start_offset = self.pos;
        while let Some(ch) = self.current_char() {
            if is_horizontal_whitespace(ch) || is_eol_char(ch) {
                break;
            }
            if ch == '$' || ch == '[' {
                break;
            }
            if ch == '\\' {
                if crate::substitution::is_line_continuation(self.source(), self.pos as usize) {
                    // `\<newline>` line continuation (bare-word
                    // context). At word start → emit the
                    // continuation as a SEP. Mid-word → stop;
                    // the iterator re-enters at the backslash.
                    if self.pos == start_offset {
                        return self.parse_backslash_newline_sep();
                    }
                    break;
                }
                match self
                    .source()
                    .as_bytes()
                    .get((self.pos + 1) as usize)
                    .copied()
                {
                    Some(_) => {
                        // `\<other>`: consume the pair as literal
                        // content (both the backslash and the
                        // escaped character stay in the token text).
                        self.pos += 1;
                        if let Some(esc) = self.current_char() {
                            self.pos += u32::try_from(esc.len_utf8()).expect("char len fits u32");
                        }
                    }
                    None => {
                        // Trailing backslash at EOF: literal.
                        self.pos += 1;
                    }
                }
                continue;
            }
            self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
        }
        self.make_token(TokenType::Esc, start_offset)
    }

    /// Consume a `\<newline>` line-continuation sequence at word
    /// start as a SEP token. A backslash-newline at the very start of
    /// a token emits `TokenType::Sep` so the next token can recognise
    /// `{` or `"` as brace/quote delimiters (a fresh word boundary).
    fn parse_backslash_newline_sep(&mut self) -> Token {
        let start = self.pos;
        self.pos += 1; // skip backslash
        if self.current_byte().is_some() {
            self.pos += 1; // skip the newline char
        }
        self.make_token(TokenType::Sep, start)
    }

    /// Parse a variable substitution starting at the current `$`.
    ///
    /// Handles all four Tcl forms:
    ///
    /// - `$name`, `$ns::var` — identifier scan accepting Unicode
    ///   alphanumerics, underscores, and `::` namespace separators.
    /// - `${name}` — braced scan; the closing `}` is consumed but
    ///   NOT included in the token span (`_end = pos - 1` before the
    ///   `}` advance).
    /// - `$arr(idx)` — array indexing with balanced `(`/`)` and
    ///   embedded `${…}` support. The `)` IS included in the span.
    /// - bare `$` — emitted as an `STR` token whose span covers
    ///   just the `$`.
    ///
    /// **Span convention.** The span always starts at the `$`
    /// position so the resolved start/end `SourcePosition`s include
    /// the dollar sign. The "human-readable" content (variable name
    /// without the leading `$` or `${`) is accessed via
    /// [`SourceMap::token_text`] rather than `SourceMap::text(span)`.
    ///
    /// Never fails. Unterminated `${` and `$arr(` tokenize
    /// best-effort, emitting non-fatal warnings once warning
    /// collection is in place.
    fn parse_var(&mut self) -> Result<Token, LexError> {
        let dollar_pos = self.pos;
        self.pos += 1; // skip '$'

        // `${name}` braced form.
        //
        // Per Tcl 9.0.3's parser (`tclParse.c::Tcl_ParseVarName`), this
        // is *not* a literal scan-to-first-`}`: the parser tracks inner
        // `{…}` with brace counting and consumes `\X` (a backslash plus
        // the following char) as part of the name — so `${a\}b}` reads
        // var `a\}b` and `${a{b}c}` reads var `a{b}c`. The Tcl(n) man
        // page's "no further substitution or modification" claim refers
        // only to `$` / `[` substitution; backslashes and inner braces
        // ARE recognised as syntax.'s
        // `_parse_var` braced branch.
        if self.current_byte() == Some(b'{') {
            self.pos += 1; // skip '{'
            let content_start = self.pos;
            self.skip_braced_var_name_body();
            let content_empty = self.pos == content_start;
            let has_close_brace = self.current_byte() == Some(b'}');
            let span_end = if content_empty && has_close_brace {
                self.pos + 1
            } else {
                self.pos
            };
            if has_close_brace {
                self.pos += 1;
            } else {
                self.warn_or_error("missing close-brace for variable name")?;
            }
            return Ok(Token::with_content_offset(
                TokenType::Var,
                Span::new(dollar_pos, span_end),
                2,
            ));
        }

        // `$name` or `$ns::var` identifier form. Bareword characters are the
        // ASCII `[0-9A-Za-z_]` ONLY (`TclIsBareword`, identical in 8.6.14 and
        // 9.0.1; the man page: "Letters and digits are only the standard
        // ASCII ones") — `$café` names the variable `caf` and `é` is ordinary
        // word text, and a `$` before a non-ASCII letter is a literal `$`.
        let name_start = self.pos;
        let name_end =
            tcl_core_types::naming::scan_var_name_end(self.source().as_bytes(), self.pos as usize);
        self.pos = u32::try_from(name_end).expect("source offset fits u32");
        // `$arr(idx)` array-index form
        if self.current_byte() == Some(b'(') {
            self.scan_array_index_body(0)?;
            return Ok(Token::with_content_offset(
                TokenType::Var,
                Span::new(dollar_pos, self.pos),
                1,
            ));
        }

        // Bare `$`
        if self.pos == name_start {
            return Ok(Token::new(
                TokenType::Str,
                Span::new(dollar_pos, dollar_pos + 1),
            ));
        }

        Ok(Token::with_content_offset(
            TokenType::Var,
            Span::new(dollar_pos, self.pos),
            1,
        ))
    }

    /// Consume a `(…)` array-index body starting at the `(`.
    ///
    /// C Tcl (`Tcl_ParseVarName` → `Tcl_ParseTokens` with a `)` terminator)
    /// does **not** count paren nesting: the index ends at the first `)` at
    /// token level. A literal `(` is plain text; a `)` is only kept in the
    /// index when it is escaped (`\)`) or sits inside a command substitution
    /// (`[…]`) or a nested variable reference (`${…}` / `$name(…)`) — whose
    /// tokens are scanned so their inner `)` are not mistaken for the
    /// terminator. Paren-counting made `$a((b)` never close (swallowing the
    /// rest of the source) and made `$a(x\)y)` end at the escaped `)`.
    /// Advances `self.pos` past the closing `)` (or to EOF
    /// for unterminated input).
    ///
    /// `depth` is the nesting level of this call (0 at the top, via
    /// [`Self::parse_var`]); past [`MAX_ARRAY_INDEX_DEPTH`] a nested `$…(`
    /// is no longer recursed into — its `(` is scanned as an ordinary
    /// character instead — so pathologically deep `$a($b($c(...)))` input
    /// degrades gracefully rather than overflowing the native stack.
    fn scan_array_index_body(&mut self, depth: u32) -> Result<(), LexError> {
        debug_assert_eq!(self.current_byte(), Some(b'('));
        self.pos += 1; // skip '('
        let past_cap = MAX_ARRAY_INDEX_DEPTH.exceeded(depth);
        loop {
            let Some(ch) = self.current_char() else {
                self.warn_or_error("missing )")?;
                return Ok(());
            };
            match ch {
                ')' => {
                    self.pos += 1;
                    return Ok(());
                }
                '\\' => {
                    // Escape: consume the backslash and the byte it protects, so
                    // `\)` stays in the index.
                    self.pos += 1;
                    if let Some(next) = self.current_char() {
                        self.pos += u32::try_from(next.len_utf8()).expect("char len fits u32");
                    }
                }
                '[' => self.skip_command_in_index(),
                '$' if !past_cap => self.skip_var_in_index(depth + 1)?,
                _ => {
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }
    }

    /// Skip a `[…]` command substitution inside an array index so a `)` inside
    /// it is not the index terminator. Tracks brace nesting (`blevel`) and
    /// double-quote state (`in_quotes`) — mirroring [`Self::parse_command`] — so
    /// a `]` inside a braced or quoted word (e.g. `$a([puts {]}])`) does not
    /// close the substitution early. Stops after the matching `]` (or at EOF).
    fn skip_command_in_index(&mut self) {
        debug_assert_eq!(self.current_byte(), Some(b'['));
        self.pos += 1;
        let mut depth: u32 = 1;
        let mut blevel: u32 = 0;
        let mut in_quotes = false;
        while depth > 0 {
            let Some(ch) = self.current_char() else {
                return;
            };
            match ch {
                '\\' => {
                    self.pos += 1;
                    if let Some(next) = self.current_char() {
                        self.pos += u32::try_from(next.len_utf8()).expect("char len fits u32");
                    }
                }
                '"' if blevel == 0 => {
                    in_quotes = !in_quotes;
                    self.pos += 1;
                }
                '{' if !in_quotes => {
                    blevel += 1;
                    self.pos += 1;
                }
                '}' if !in_quotes => {
                    blevel = blevel.saturating_sub(1);
                    self.pos += 1;
                }
                '[' if blevel == 0 && !in_quotes => {
                    depth += 1;
                    self.pos += 1;
                }
                ']' if blevel == 0 && !in_quotes => {
                    depth -= 1;
                    self.pos += 1;
                }
                _ => {
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }
    }

    /// Skip a `$` variable reference inside an array index (`${…}` or `$name`
    /// with an optional nested `(…)` index) so its inner `)` are not mistaken
    /// for the outer index terminator.
    ///
    /// `depth` is passed straight through to a nested
    /// [`Self::scan_array_index_body`] call — see its doc comment.
    fn skip_var_in_index(&mut self, depth: u32) -> Result<(), LexError> {
        debug_assert_eq!(self.current_byte(), Some(b'$'));
        self.pos += 1; // skip '$'
        if self.current_byte() == Some(b'{') {
            self.pos += 1;
            self.skip_braced_var_name_body();
            if self.current_byte() == Some(b'}') {
                self.pos += 1;
            }
            return Ok(());
        }
        // Bare name: an ASCII alphanumeric / `_` run (`TclIsBareword`), with
        // `::` namespace separators.
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.pos += 1;
            } else if ch == ':' && self.peek_byte(1) == Some(b':') {
                self.pos += 2;
            } else {
                break;
            }
        }
        // A nested `$name(index)` — recurse so its `)` closes the inner index.
        if self.current_byte() == Some(b'(') {
            self.scan_array_index_body(depth)?;
        }
        Ok(())
    }

    /// Return the byte at `self.pos + offset`, if any.
    #[inline]
    fn peek_byte(&self, offset: u32) -> Option<u8> {
        self.source()
            .as_bytes()
            .get((self.pos + offset) as usize)
            .copied()
    }

    /// Parse a quoted-string `ESC` token.
    ///
    /// Called from the iterator in two modes:
    ///
    /// - `opening = true`: the iterator has seen a `"` at a word
    ///   boundary (top-level, `!in_quote`, `is_newword()`). Skip
    ///   the opening `"`, set `in_quote = true`, then scan content
    ///   until a terminator.
    /// - `opening = false`: the iterator is already `in_quote`.
    ///   Scan content from the current position until a terminator.
    ///   If the terminator is the closing `"`, consume it and
    ///   reset `in_quote = false`.
    ///
    /// The content scan stops at `$`, `[`, `"`, or `\`, or EOF.
    /// `$` and `[` are left in the stream for the iterator to
    /// dispatch to `parse_var` / `parse_command` on the next
    /// iteration. `\` is left in the stream so the iterator can
    /// surface it as an unsupported character (proper handling is
    /// not yet implemented). Everything else — separators, EOL characters,
    /// `#`, `{`, `}` — is consumed as literal content.
    ///
    /// ### Span convention
    ///
    /// The span starts at `start_offset` (the opening `"` when
    /// `opening`, or the first content byte otherwise) and normally
    /// ends at the position where the scanner stopped. In the
    /// **empty-content** case — where the scanner stopped
    /// immediately without consuming any content — the span is
    /// extended by one byte to cover the stop character itself,
    /// so the end position lands on it, for the degenerate cases:
    ///
    /// - `""` — empty body, stop char is the closing `"`. Span
    ///   covers `""`; `token_text` returns `""`.
    /// - `"$foo"` opening ESC — stop char is `$`. Span covers
    ///   `"$`; `token_text` returns `""` and the next dispatch
    ///   handles the `$`.
    /// - `"[cmd]"` opening ESC — same, stop char is `[`.
    /// - Closing empty ESC after a sub-token (`$foo"` after a
    ///   VAR) — stop char is the closing `"`. Span covers `"`;
    ///   `token_text` returns `""`.
    ///
    /// Never fails. An unterminated quoted string tokenizes
    /// best-effort — the scanner consumes everything up to EOF
    /// and returns an `ESC` with `in_quote = true` still set, so
    /// the trailing synthetic EOL inherits the `true` flag too.
    /// The "missing close-quote" warning is not yet emitted.
    fn parse_quoted(&mut self, opening: bool) -> Result<Token, LexError> {
        let start_offset = self.pos;
        if opening {
            self.pos += 1; // skip opening `"`
            self.in_quote = true;
        }
        let content_start = self.pos;
        let mut closed = false;

        while let Some(ch) = self.current_char() {
            match ch {
                '"' => {
                    closed = true;
                    break;
                }
                '$' | '[' => break,
                '\\' => {
                    // Inside a quoted string, `\<char>` is consumed
                    // as a literal pair (both the backslash and
                    // the following character stay in the token
                    // text). `\<newline>` inside a quote is NOT a
                    // word break — it's just another pair of
                    // literal bytes.
                    self.pos += 1;
                    match self.current_char() {
                        Some(esc @ ('\n' | '\r')) => {
                            self.pos += 1;
                            if esc == '\r' && self.current_byte() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        Some(esc) => {
                            self.pos += u32::try_from(esc.len_utf8()).expect("char len fits u32");
                        }
                        None => {} // trailing backslash at EOF
                    }
                }
                _ => {
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }

        let content_empty = self.pos == content_start;
        // Empty-content clamp: extend the span by one byte so the
        // end position lands on the terminator (closing `"`, or the
        // `$` / `[` that stopped the scan). Don't extend at EOF
        // with no terminator — that would push `span.end` past
        // `source.len()` and make `SourceMap::text` panic on
        // slicing.
        let has_stop = closed || self.current_char().is_some();
        let span_end = if content_empty && has_stop {
            self.pos + 1
        } else {
            self.pos
        };

        if closed {
            self.pos += 1; // advance past closing `"`
            self.in_quote = false;
            // Check for extra characters after close-quote.
            if let Some(after) = self.current_byte() {
                let is_bs_nl = after == b'\\'
                    && matches!(
                        self.source().as_bytes().get(self.pos as usize + 1),
                        Some(b'\n' | b'\r')
                    );
                if self.config.irules_brace_separator {
                    // F5 R2 (measurements §1): a word that started with
                    // `"` splits exactly as a braced one — `"a"b` →
                    // `a`/`b`, `"a""b"` → `a`/`b`, `"a"}` → `a`/`}` —
                    // with no diagnostic (R6).
                    if !is_separator_byte(after) && !is_bs_nl {
                        let sep_span = Span::empty(self.pos);
                        self.pending_sep = Some(Token::new(TokenType::Sep, sep_span));
                    }
                } else {
                    let ok = is_separator_byte(after) || after == b']' || is_bs_nl;
                    if !ok {
                        self.warn_or_error("extra characters after close-quote")?;
                    }
                }
            }
        } else if self.current_char().is_none() {
            // EOF without closing quote.
            self.warn_or_error("missing \"")?;
        }

        let content_offset: u8 = if !opening && content_empty && closed {
            // A bare closing-quote sub-token (the trailing `"` of `"$foo"`,
            // emitted as a 1-byte ESC): the `"` is a leading delimiter of a
            // zero-content token, so `content_offset == 1` makes
            // `SourceMap::token_text` yield `""`. This distinguishes it from a
            // *literal* trailing `"` in a bare word, which `parse_esc` emits
            // with `content_offset == 0` and `in_quote == false` (issue 160).
            // The opening / mid-string cases keep `opening.into()` so the
            // semantic-tokens fragment logic (which trims an extended `$`/`[`
            // introducer keyed on `content_offset`) is unaffected.
            1
        } else {
            opening.into()
        };
        Ok(Token::with_content_offset(
            TokenType::Esc,
            Span::new(start_offset, span_end),
            content_offset,
        ))
    }

    /// Whether the next token is at a word boundary (so `{` at
    /// `self.pos` starts a braced string rather than being part of a
    /// bare word) — true when the previously emitted token was `Sep`,
    /// `Eol`,
    /// `Str`, or `Expand`. The initial state before any token is
    /// emitted is `Eol`, so the first character of the source is
    /// always at a word boundary.
    #[inline]
    fn is_newword(&self) -> bool {
        matches!(
            self.last_kind,
            TokenType::Sep | TokenType::Eol | TokenType::Str | TokenType::Expand
        )
    }

    /// Check for the `{*}` expansion prefix (Tcl 8.5+) at the
    /// current `{`, and dispatch to either `parse_expand` or
    /// `parse_brace`. The condition is:
    ///
    /// ```text
    /// if expand_syntax
    ///     and text[pos+1] == '*'
    ///     and text[pos+2] == '}'
    ///     and text[pos+3] is a non-separator
    /// ```
    fn parse_brace_or_expand(&mut self) -> Result<Token, LexError> {
        if self.config.expand_syntax
            && self.peek_byte(1) == Some(b'*')
            && self.peek_byte(2) == Some(b'}')
            && let Some(after) = self.peek_byte(3)
            && !is_separator_byte(after)
        {
            return Ok(self.parse_expand());
        }
        self.parse_brace()
    }

    /// Emit an `EXPAND` token for the `{*}` prefix. The token is a
    /// zero-width marker anchored at the `{` position (`span =
    /// [pos, pos)`), so `range_positions` gives start == end there.
    /// The lexer then advances `self.pos` past the full `{*}` so
    /// the next dispatch starts at the word to expand.
    fn parse_expand(&mut self) -> Token {
        let start = self.pos;
        self.pos += 3; // skip `{*}`
        // Emit an empty span anchored at the `{` position.
        Token::new(TokenType::Expand, Span::empty(start))
    }

    /// Parse a braced string starting at the current `{`. The caller
    /// is responsible for checking `is_newword()` before dispatching
    /// here — a `{` in the middle of a bare word is a regular
    /// character in `parse_esc`, not a braced-string opener.
    ///
    /// The scanner counts balanced `{` / `}` pairs (`level` starts
    /// at 1 after skipping the opening `{`). Backslash sequences
    /// consume two characters as a pair (CRLF counted as one), but
    /// the backslash and the following character are preserved
    /// literally in the token text — braces are "verbatim" in Tcl,
    /// so `\}` inside a brace body does NOT count as a close brace
    /// even though the scanner skips over it.
    ///
    /// The span starts at the `{` and normally ends at the last
    /// character of the body (NOT the closing `}`). The
    /// empty-body degenerate `{}` extends the span by one so
    /// `range_positions` reports the `}` as the end position.
    /// `SourceMap::token_text` strips the leading `{` (and the
    /// trailing `}` for the degenerate `{}` case) so callers
    /// see just the inside of the braces.
    ///
    /// Never fails. Unterminated `{` tokenizes best-effort. Under the F5
    /// word-break axis (`irules_brace_separator`), a non-separator after
    /// the close brace injects a ghost SEP instead of the "extra
    /// characters after close-brace" warning — see the R2 comment below.
    fn parse_brace(&mut self) -> Result<Token, LexError> {
        let brace_pos = self.pos;
        self.pos += 1; // skip opening '{'
        let content_start = self.pos;

        let mut level: u32 = 1;
        let mut span_end: u32 = self.pos;

        while let Some(ch) = self.current_char() {
            match ch {
                '\\' => {
                    // Consume the backslash and the next character
                    // as a pair (CRLF counted as one character).
                    self.pos += 1;
                    match self.current_char() {
                        Some(esc @ ('\n' | '\r')) => {
                            self.pos += 1;
                            if esc == '\r' && self.current_byte() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        Some(esc) => {
                            self.pos += u32::try_from(esc.len_utf8()).expect("char len fits u32");
                        }
                        None => {
                            // Trailing backslash at EOF — leave as is.
                        }
                    }
                }
                '{' => {
                    level += 1;
                    self.pos += 1;
                }
                '}' => {
                    level -= 1;
                    if level == 0 {
                        // Leave `self.pos` at the closing `}`; the
                        // code after the loop decides whether to
                        // include it in the span and consume it.
                        span_end = self.pos;
                        break;
                    }
                    self.pos += 1;
                }
                _ => {
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }

        // At this point `self.pos` is at the closing `}` (level
        // reached zero) or at EOF. `span_end` was set inside the
        // `}` branch; if the loop fell through (EOF), set it now.
        if level > 0 {
            span_end = self.pos;
        }

        let content_empty = span_end == content_start;
        let has_close_brace = self.current_byte() == Some(b'}');
        let final_span_end = if content_empty && has_close_brace {
            span_end + 1 // include the `}` so the end position lands on it
        } else {
            span_end
        };
        if has_close_brace {
            self.pos += 1;
            // Check for extra characters after close-brace.
            if let Some(after) = self.current_byte()
                && !is_separator_byte(after)
            {
                // Backslash-newline after close-brace is fine
                let is_bs_nl = after == b'\\'
                    && matches!(
                        self.source().as_bytes().get(self.pos as usize + 1),
                        Some(b'\n' | b'\r')
                    );
                if !is_bs_nl {
                    if self.config.irules_brace_separator {
                        // F5 R2 (measurements §1): the word started with
                        // `{`, so ANY non-separator after its close brace
                        // begins a new word — a zero-width ghost SEP,
                        // with no diagnostic (R6). The next word parses
                        // from scratch under ordinary word-start rules
                        // (R3): `{a}{b}` → `a`/`b`, `{a}b` → `a`/`b`,
                        // `{*}$l` → literal `*` word plus `$l` (the
                        // separator wins; expansion does not exist).
                        let sep_span = Span::empty(self.pos);
                        self.pending_sep = Some(Token::new(TokenType::Sep, sep_span));
                    } else {
                        self.warn_or_error("extra characters after close-brace")?;
                    }
                }
            }
        } else {
            self.warn_or_error("missing close-brace")?;
        }

        Ok(Token::with_content_offset(
            TokenType::Str,
            Span::new(brace_pos, final_span_end),
            1,
        ))
    }

    /// Parse a command substitution starting at the current `[`.
    ///
    /// Scans the command body until a matching `]`, tracking outer
    /// bracket nesting (`level`), brace nesting (`blevel`), and
    /// whether we are inside a `"…"` sub-region (`in_quotes`). Each
    /// piece of state gates which characters are meaningful:
    ///
    /// - `"` — toggles `in_quotes` when `blevel == 0`.
    /// - `[` — increments `level` when not braced and not quoted.
    /// - `]` — decrements `level` when not braced and not quoted;
    ///   closes the command when `level` reaches zero.
    /// - `\\` — consumes the next character unconditionally (CRLF
    ///   counted as one pair). This makes `\]` and `\"` inside a
    ///   command body inert.
    /// - `$` followed by `{` (outside braces and quotes) — sub-scans
    ///   a `${…}` construct so a `}` or `)` inside a braced
    ///   variable name does not fool the counter.
    /// - `{` / `}` — adjust `blevel` when not quoted.
    ///
    /// The span always starts at the `[` and normally ends at the
    /// last character of the body (NOT the closing `]`). The
    /// empty-command degenerate case `[]` extends the span by one so
    /// `range_positions` reports the `]` as the end position.
    /// `SourceMap::token_text` strips the leading `[` (and the
    /// trailing `]` from the degenerate case) so callers see just the
    /// command body.
    ///
    /// Never fails. An unterminated `[` tokenizes best-effort; the
    /// `missing close-bracket` warning is not yet emitted.
    /// Advance `self.pos` past a `${…}` braced variable name beginning at the
    /// current `$` (whose next byte is `{`), using the same brace-nesting +
    /// backslash-pair rules as [`Self::parse_var`]'s braced branch: `\X` is a
    /// literal pair (so `\}` does not close) and inner `{`/`}` nest. Shared by
    /// the command-substitution scanner so an inner `}`/`]`/`)` in a braced
    /// name does not fool its delimiter counter (issue 163).
    fn skip_braced_var_name(&mut self) {
        self.pos += 2; // skip '${'
        self.skip_braced_var_name_body();
        if self.current_byte() == Some(b'}') {
            self.pos += 1;
        }
    }

    /// Advance from the first byte of a `${…}` variable **name** to the `}`
    /// that closes it (or to end-of-input when the form is unterminated),
    /// leaving `pos` *on* the closer.
    ///
    /// The release-aware close rule itself lives in
    /// [`crate::ranges::braced_var_name_end`], the one owner both `subst`
    /// engines resolve `${…}` through as well (issue #1457).
    ///
    /// An unterminated form is C's `missing close-brace for variable name`
    /// error, but this lexer is the *tokenizer* — it must keep producing tokens
    /// for half-typed source, so it takes the documented lenient recovery and
    /// runs the name to end-of-input. The evaluating engines raise instead.
    fn skip_braced_var_name_body(&mut self) {
        let end = match crate::ranges::braced_var_name_end(
            self.source().as_bytes(),
            self.pos as usize,
            self.config.braced_var,
        ) {
            crate::ranges::BracedVarEnd::Closed(end) => end,
            crate::ranges::BracedVarEnd::Unterminated => self.source().len(),
        };
        self.pos = u32::try_from(end).expect("source offset fits u32");
    }

    /// Advance one character/escape while inside a command-position comment
    /// nested in `[...]`. Returns `true` only when an unescaped newline ends
    /// the comment. Escape width comes from the canonical decoder, including
    /// CRLF continuations and their swallowed indentation.
    fn advance_command_comment(&mut self, ch: char) -> bool {
        match ch {
            '\n' => {
                self.pos += 1;
                true
            }
            '\\' => {
                let end = crate::substitution::backslash_escape_end_in(
                    self.source(),
                    self.pos as usize,
                    self.config.escapes,
                );
                self.pos = u32::try_from(end).expect("source offset fits u32");
                false
            }
            _ => {
                self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                false
            }
        }
    }

    /// Consume a recovery ghost `]` at the current position. Ghosts close a
    /// command unconditionally: their derivation already vetoed inert brace,
    /// escape, and `${...}` positions. Returns `None` when there is no ghost,
    /// or `Some(true)` when this one closes the outermost substitution.
    fn advance_command_ghost(&mut self, level: &mut u32) -> Option<bool> {
        (self.ghost_at(self.pos) == Some(b']')).then(|| {
            *level -= 1;
            if *level == 0 {
                true
            } else {
                // It is a zero-width insertion: remove it and re-examine the
                // real byte at this offset on the next iteration.
                self.ghosts.remove(&self.pos);
                false
            }
        })
    }

    /// Finish a `[...]` token after its body scan stopped at a real/ghost
    /// closer or EOF. Empty command bodies include their real closer so the
    /// inner-end span still lands on it; recovery ghosts remain zero-width.
    fn finish_command_token(
        &mut self,
        bracket_pos: u32,
        content_start: u32,
    ) -> Result<Token, LexError> {
        let content_empty = self.pos == content_start;
        let close_is_ghost = self.ghost_at(self.pos) == Some(b']');
        let has_close_bracket = self.current_byte() == Some(b']');
        let span_end = if content_empty && has_close_bracket && !close_is_ghost {
            self.pos + 1
        } else {
            self.pos
        };
        if close_is_ghost {
            self.ghosts.remove(&self.pos);
        } else if has_close_bracket {
            self.pos += 1;
        } else {
            self.warn_or_error("missing close-bracket")?;
        }
        Ok(Token::with_content_offset(
            TokenType::Cmd,
            Span::new(bracket_pos, span_end),
            1,
        ))
    }

    fn parse_command(&mut self) -> Result<Token, LexError> {
        let bracket_pos = self.pos;
        self.pos += 1; // skip '['
        let content_start = self.pos;

        let mut level: u32 = 1;
        let mut blevel: u32 = 0;
        let mut in_quotes = false;
        // A command substitution contains a Tcl script, so its command
        // position is independent of the outer lexer.  Keep this pair with
        // the delimiter counters: a `#` is a comment only outside quotes /
        // braces, after the `[` or a top-level `;` / newline (issue #1483).
        let mut in_comment = false;
        let mut at_command_start = true;

        while let Some(ch) = self.current_char() {
            if let Some(closed) = self.advance_command_ghost(&mut level) {
                if closed {
                    break;
                }
                continue;
            }

            // `#` comments are parsed by the nested Tcl script, not by the
            // outer lexer.  In particular, a `]` in a command-position
            // comment is content and must not close this substitution.  A
            // backslash-newline continues the comment, matching
            // `parse_comment` and C Tcl's TclParse.c rule.
            if in_comment {
                if self.advance_command_comment(ch) {
                    in_comment = false;
                    at_command_start = true;
                }
                continue;
            }

            if ch == '#' && at_command_start && blevel == 0 && !in_quotes {
                in_comment = true;
                self.pos += 1;
                continue;
            }

            match ch {
                '"' if blevel == 0 => {
                    in_quotes = !in_quotes;
                    at_command_start = false;
                    self.pos += 1;
                }
                '[' if blevel == 0 && !in_quotes => {
                    level += 1;
                    // A nested command substitution starts a fresh script.
                    at_command_start = true;
                    self.pos += 1;
                }
                ']' if blevel == 0 && !in_quotes => {
                    level -= 1;
                    if level == 0 {
                        // Leave `self.pos` at the closing `]`; the
                        // code after the loop decides whether to
                        // include it in the span and consume it.
                        break;
                    }
                    at_command_start = false;
                    self.pos += 1;
                }
                '\\' => {
                    let continuation =
                        crate::substitution::is_line_continuation(self.source(), self.pos as usize);
                    // Consume the backslash and the next character as a pair.
                    // A raw CR is ordinary escaped data, so a following LF is
                    // left for the next iteration to terminate the command.
                    self.pos += 1;
                    match self.current_char() {
                        Some(esc) => {
                            self.pos += u32::try_from(esc.len_utf8()).expect("char len fits u32");
                        }
                        None => {
                            // Trailing backslash at EOF — leave as is.
                        }
                    }
                    // A continuation substitutes to whitespace and keeps
                    // command position; every other escape is word content.
                    if !continuation {
                        at_command_start = false;
                    }
                }
                '$' if !in_quotes && blevel == 0 => {
                    // `${…}` inside a command body: sub-scan to the matching `}`
                    // so any `)` or `]` inside a braced variable name does not
                    // fool the outer counter.
                    if self.peek_byte(1) == Some(b'{') {
                        self.skip_braced_var_name();
                    } else {
                        self.pos += 1;
                    }
                    at_command_start = false;
                }
                '{' if !in_quotes => {
                    blevel += 1;
                    at_command_start = false;
                    self.pos += 1;
                }
                '}' if !in_quotes => {
                    blevel = blevel.saturating_sub(1);
                    at_command_start = false;
                    self.pos += 1;
                }
                '\n' | ';' if blevel == 0 && !in_quotes => {
                    at_command_start = true;
                    self.pos += 1;
                }
                ' ' | '\t' | '\r' | '\u{0b}' | '\u{0c}' if blevel == 0 && !in_quotes => {
                    // Leading whitespace preserves command position.
                    self.pos += 1;
                }
                _ => {
                    at_command_start = false;
                    self.pos += u32::try_from(ch.len_utf8()).expect("char len fits u32");
                }
            }
        }

        self.finish_command_token(bracket_pos, content_start)
    }
}

impl Iterator for Lexer<'_> {
    type Item = Result<Token, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        // Drain any pending ghost SEP (iRules `}{` boundary).
        if let Some(sep) = self.pending_sep.take() {
            self.last_kind = sep.kind;
            return Some(Ok(sep));
        }

        // EOF: emit a trailing ghost EOL (once) then stop.
        if self.pos as usize >= self.source().len() {
            if self.last_kind == TokenType::Eol {
                self.done = true;
                return None;
            }
            self.last_kind = TokenType::Eol;
            return Some(Ok(Token::new(TokenType::Eol, Span::empty(self.pos))));
        }

        let ch = self
            .current_char()
            .expect("source[pos..] is non-empty when pos < len");

        // Dispatch splits on `in_quote`. Inside a quoted string
        // the scanner ignores separators, EOL characters, `#`, and
        // `{` / `}` — they become literal content — and only `$`,
        // `[`, `"`, and `\` are meaningful. Outside a quoted
        // string the dispatch is the full top-level one.
        let result = if self.in_quote {
            // Inside a quoted string: `$`, `[` dispatch to their
            // own parsers; `\` is still a deferred error; every
            // other character (including the closing `"`) is
            // handled by `parse_quoted(false)`, which scans
            // content and consumes the closing `"` when it
            // encounters one.
            match ch {
                '$' => self.parse_var(),
                '[' => self.parse_command(),
                _ => self.parse_quoted(false),
            }
        } else {
            match ch {
                _ if is_horizontal_whitespace(ch) => Ok(self.parse_sep()),
                // F5 N1 (measurements §2): a newline whose next line
                // starts (after horizontal whitespace) with `{` is a word
                // separator, not a command terminator. A `;` always
                // terminates, and a blank/comment line falls through to
                // the ordinary EOL run below (N4).
                '\n' if self.newline_continues_command() => {
                    Ok(self.parse_brace_line_continuation_sep())
                }
                _ if is_eol_char(ch) => Ok(self.parse_eol()),
                '#' if self.at_command_start => Ok(self.parse_comment()),
                '$' => self.parse_var(),
                '[' => self.parse_command(),
                '{' if self.is_newword() => self.parse_brace_or_expand(),
                '"' if self.is_newword() => self.parse_quoted(true),
                _ => Ok(self.parse_esc()),
            }
        };

        match result {
            Ok(mut tok) => {
                match tok.kind {
                    TokenType::Eol => self.at_command_start = true,
                    TokenType::Sep | TokenType::Comment => {
                        // Preserve current value.
                    }
                    _ => self.at_command_start = false,
                }
                self.last_kind = tok.kind;
                // Tag the token with the current `in_quote` state.
                // `parse_quoted` resets `self.in_quote = false`
                // *before* returning when it consumes the closing
                // `"`, so the last ESC of a quoted run picks up
                // `false` here.
                tok.in_quote = self.in_quote;
                Some(Ok(tok))
            }
            Err(err) => {
                // Fuse the iterator on fatal error.
                self.done = true;
                Some(Err(err))
            }
        }
    }
}

#[inline]
fn is_horizontal_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\r' | '\u{0B}' | '\u{0C}')
}

#[inline]
fn is_horizontal_whitespace_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | 0x0B | 0x0C)
}

#[inline]
fn is_eol_char(ch: char) -> bool {
    ch == '\n' || ch == ';'
}

#[inline]
fn is_eol_byte(byte: u8) -> bool {
    byte == b'\n' || byte == b';'
}

/// Union of `is_horizontal_whitespace_byte` and `is_eol_byte`.
/// Used by the `{*}` expansion-prefix guard to test the byte after `}`.
#[inline]
fn is_separator_byte(byte: u8) -> bool {
    is_horizontal_whitespace_byte(byte) || is_eol_byte(byte)
}

// There is no "deferred special" character set: every valid ASCII
// character in a Tcl source is handled by the lexer.

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    use crate::line_index::LineIndex;
    use crate::tokens::{ByteCol, SourcePosition};

    struct Lexed<'src> {
        source_map: SourceMap<'src>,
        tokens: Vec<Token>,
    }

    impl<'src> Lexed<'src> {
        fn run(source: &'src str) -> Self {
            let lexer = Lexer::new(source);
            let source_map = lexer.source_map().clone();
            let tokens = lexer.tokenise_all().expect("L3 lexer accepts fixture");
            Self { source_map, tokens }
        }

        fn kinds(&self) -> Vec<TokenType> {
            self.tokens.iter().map(|t| t.kind).collect()
        }

        fn texts(&self) -> Vec<&'src str> {
            self.tokens
                .iter()
                .map(|t| self.source_map.text(t.span))
                .collect()
        }

        fn positions(&self, idx: usize) -> (SourcePosition, SourcePosition) {
            self.source_map.range_positions(self.tokens[idx].span)
        }
    }

    #[test]
    fn empty_source_produces_no_tokens() {
        let lexed = Lexed::run("");
        assert!(lexed.tokens.is_empty());
    }

    #[test]
    fn single_word_emits_esc_and_trailing_eol() {
        let lexed = Lexed::run("foo");
        assert_eq!(lexed.tokens.len(), 2);
        assert_eq!(lexed.tokens[0].kind, TokenType::Esc);
        assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "foo");
        assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "");
    }

    #[test]
    fn two_words_separated_by_space() {
        let lexed = Lexed::run("foo bar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
        assert_eq!(lexed.texts(), vec!["foo", " ", "bar", ""]);
    }

    #[test]
    fn multiple_spaces_collapse_into_one_sep_token() {
        let lexed = Lexed::run("foo   bar");
        assert_eq!(lexed.texts(), vec!["foo", "   ", "bar", ""]);
    }

    #[test]
    fn tab_separator() {
        let lexed = Lexed::run("foo\tbar");
        assert_eq!(lexed.texts(), vec!["foo", "\t", "bar", ""]);
    }

    #[test]
    fn cr_is_separator_not_eol() {
        // `\r` is a separator char, not an end-of-line char.
        let lexed = Lexed::run("foo\rbar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn lf_is_eol() {
        let lexed = Lexed::run("foo\nbar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Eol,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn semicolon_is_eol() {
        let lexed = Lexed::run("foo;bar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Eol,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn mixed_eol_and_whitespace_becomes_single_eol_token() {
        let lexed = Lexed::run("foo\n \t;\nbar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Eol,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n \t;\n");
    }

    #[test]
    fn leading_whitespace_before_word() {
        let lexed = Lexed::run("  foo");
        assert_eq!(
            lexed.kinds(),
            vec![TokenType::Sep, TokenType::Esc, TokenType::Eol]
        );
    }

    #[test]
    fn trailing_whitespace_still_emits_ghost_eol() {
        let lexed = Lexed::run("foo  ");
        assert_eq!(
            lexed.kinds(),
            vec![TokenType::Esc, TokenType::Sep, TokenType::Eol]
        );
        assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "");
    }

    #[test]
    fn trailing_newline_does_not_add_second_eol() {
        let lexed = Lexed::run("foo\n");
        assert_eq!(lexed.kinds(), vec![TokenType::Esc, TokenType::Eol]);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
    }

    #[test]
    fn comment_at_command_start() {
        let lexed = Lexed::run("# hello world");
        assert_eq!(lexed.tokens[0].kind, TokenType::Comment);
        assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "# hello world");
    }

    #[test]
    fn comment_terminated_by_newline() {
        let lexed = Lexed::run("# hello\nfoo");
        assert_eq!(lexed.tokens[0].kind, TokenType::Comment);
        assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "# hello");
        assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
        assert_eq!(lexed.tokens[2].kind, TokenType::Esc);
        assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "foo");
    }

    #[test]
    fn comment_after_whitespace_at_command_start() {
        // Leading whitespace preserves `at_command_start`, so `#` is
        // still a comment.
        let lexed = Lexed::run("   # comment");
        assert_eq!(lexed.tokens[0].kind, TokenType::Sep);
        assert_eq!(lexed.tokens[1].kind, TokenType::Comment);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "# comment");
    }

    #[test]
    fn hash_not_at_command_start_is_part_of_word() {
        let lexed = Lexed::run("foo #bar");
        assert_eq!(
            lexed.kinds(),
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Esc,
                TokenType::Eol,
            ]
        );
        assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "#bar");
    }

    #[test]
    fn two_commands_separated_by_eol_both_allow_comments() {
        let lexed = Lexed::run("foo\n# comment");
        assert!(lexed.tokens.iter().any(|t| t.kind == TokenType::Comment));
    }

    #[test]
    fn position_tracking_simple_word() {
        let lexed = Lexed::run("foo");
        let (start, end) = lexed.positions(0);
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(2), 2));
    }

    #[test]
    fn position_tracking_across_newline() {
        let lexed = Lexed::run("ab\ncd");
        // ESC "ab" at (0,0)-(0,1)
        let (start, end) = lexed.positions(0);
        assert_eq!(lexed.source_map.text(lexed.tokens[0].span), "ab");
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(1), 1));
        // EOL "\n" at (0,2)-(0,2)
        let (start, end) = lexed.positions(1);
        assert_eq!(lexed.tokens[1].kind, TokenType::Eol);
        assert_eq!(lexed.source_map.text(lexed.tokens[1].span), "\n");
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(2), 2));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(2), 2));
        // ESC "cd" at (1,0)-(1,1)
        let (start, end) = lexed.positions(2);
        assert_eq!(lexed.source_map.text(lexed.tokens[2].span), "cd");
        assert_eq!(start, SourcePosition::new(1, ByteCol::new(0), 3));
        assert_eq!(end, SourcePosition::new(1, ByteCol::new(1), 4));
    }

    #[test]
    fn spans_are_accurate() {
        let lexed = Lexed::run("foo bar");
        assert_eq!(lexed.tokens[0].span, Span::new(0, 3)); // "foo"
        assert_eq!(lexed.tokens[1].span, Span::new(3, 4)); // " "
        assert_eq!(lexed.tokens[2].span, Span::new(4, 7)); // "bar"
        assert!(lexed.tokens[3].span.is_empty()); // ghost EOL
    }

    #[test]
    fn dollar_is_no_longer_an_unsupported_character() {
        // Regression guard: `$` is no longer in the deferred set.
        // The lexer should accept `$bar` as a VAR token, not error.
        let tokens = Lexer::new("foo $bar").tokenise_all().unwrap();
        let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Var,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn brace_is_no_longer_an_unsupported_character() {
        // Regression guard: `{` and `}` are no longer in the deferred
        // set. `foo {bar}` should now lex as ESC + SEP + STR + EOL.
        let tokens = Lexer::new("foo {bar}").tokenise_all().unwrap();
        let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                TokenType::Esc,
                TokenType::Sep,
                TokenType::Str,
                TokenType::Eol,
            ]
        );
    }

    #[test]
    fn bracket_is_no_longer_an_unsupported_character() {
        // Regression guard: `[` is no longer in the deferred set.
        // `[cmd]` should now lex as a CMD token, not error.
        let tokens = Lexer::new("[cmd]").tokenise_all().unwrap();
        let kinds: Vec<TokenType> = tokens.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, vec![TokenType::Cmd, TokenType::Eol]);
    }

    #[test]
    fn backslash_in_bare_word_is_literal_pair() {
        // `foo\nbar` — `\n` (backslash + 'n') is a literal pair
        // inside a bare word, matching C Tcl's tokenisation.
        let lexed = Lexed::run(r"foo\nbar");
        assert_eq!(lexed.texts(), vec![r"foo\nbar", ""]);
    }

    #[test]
    fn backslash_in_comment_continues_line() {
        // `# comment\<newline>continues` — the backslash-newline
        // continues the comment to the next line.
        let lexed = Lexed::run("# hello\\\nworld");
        assert_eq!(lexed.kinds(), vec![TokenType::Comment, TokenType::Eol]);
        assert_eq!(
            lexed.source_map.token_text(lexed.tokens[0]),
            "# hello\\\nworld"
        );
    }

    #[test]
    fn backslash_in_bare_word_does_not_error() {
        // Regression guard: `\` is no longer in the deferred set.
        let lexed = Lexed::run(r"foo\nbar");
        assert_eq!(lexed.kinds(), vec![TokenType::Esc, TokenType::Eol]);
    }

    #[test]
    fn shared_source_map_constructor() {
        let source = "foo\nbar";
        let map = SourceMap::new(source);
        let via_shared = Lexer::with_source_map(map, LexerConfig::default())
            .tokenise_all()
            .unwrap();
        let via_new = Lexer::new(source).tokenise_all().unwrap();
        assert_eq!(via_shared, via_new);
    }

    #[test]
    fn shared_line_index_via_source_map() {
        let source = "alpha beta";
        let idx = LineIndex::new(source);
        let map = SourceMap::with_line_index(source, idx);
        let tokens = Lexer::with_source_map(map, LexerConfig::default())
            .tokenise_all()
            .unwrap();
        assert!(!tokens.is_empty());
    }

    // Variable substitution

    fn var_token_text(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
        let lexer = Lexer::new(source);
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("L4 lexer accepts fixture");
        // `token_text` strips the leading `$` / `${` for VAR tokens
        // so the assertions check just the variable name.
        let rows = tokens
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned()))
            .collect();
        (rows, map)
    }

    #[test]
    fn var_simple_identifier() {
        let (rows, _) = var_token_text("$foo");
        assert_eq!(
            rows,
            vec![
                (TokenType::Var, "foo".into()),
                (TokenType::Eol, String::new()),
            ]
        );
    }

    #[test]
    fn var_with_underscore() {
        let (rows, _) = var_token_text("$_private");
        assert_eq!(rows[0], (TokenType::Var, "_private".into()));
    }

    #[test]
    fn var_alphanumeric_accepts_digits_anywhere() {
        assert_eq!(
            var_token_text("$foo1").0[0],
            (TokenType::Var, "foo1".into())
        );
        // Digits are allowed at the start of variable names —
        // Tcl uses `$1`, `$2` etc. for regexp backrefs.
        assert_eq!(var_token_text("$1").0[0], (TokenType::Var, "1".into()));
    }

    #[test]
    fn var_uppercase() {
        assert_eq!(var_token_text("$FOO").0[0], (TokenType::Var, "FOO".into()));
    }

    #[test]
    fn var_namespace_separator() {
        let (rows, _) = var_token_text("$ns::var");
        assert_eq!(rows[0], (TokenType::Var, "ns::var".into()));
    }

    #[test]
    fn var_multi_level_namespace() {
        let (rows, _) = var_token_text("$a::b::c");
        assert_eq!(rows[0], (TokenType::Var, "a::b::c".into()));
    }

    #[test]
    fn var_leading_namespace() {
        // `$::global` — starts with `::` (double colon).
        let (rows, _) = var_token_text("$::global");
        assert_eq!(rows[0], (TokenType::Var, "::global".into()));
    }

    #[test]
    fn var_consumes_entire_colon_run() {
        // `$a:::b` — C Tcl's `Tcl_ParseVarName` consumes the whole colon run
        // once a `::` starts it, so the variable is `a:::b`, not `a::` + `:b`
        // (issue 162).
        let (rows, _) = var_token_text("$a:::b");
        assert_eq!(rows[0], (TokenType::Var, "a:::b".into()));
        // Four colons likewise stay in the name.
        assert_eq!(
            var_token_text("$a::::b").0[0],
            (TokenType::Var, "a::::b".into())
        );
    }

    #[test]
    fn var_single_colon_terminates_name() {
        // A single `:` is not part of the identifier; it ends the
        // VAR token and the rest becomes an ESC token.
        let (rows, _) = var_token_text("$foo:bar");
        assert_eq!(rows[0], (TokenType::Var, "foo".into()));
        assert_eq!(rows[1], (TokenType::Esc, ":bar".into()));
    }

    #[test]
    fn var_braced_form() {
        let (rows, _) = var_token_text("${name}");
        // The braces are stripped — the token text is the body only.
        assert_eq!(rows[0], (TokenType::Var, "name".into()));
    }

    #[test]
    fn var_braced_empty_body() {
        let (rows, _) = var_token_text("${}");
        assert_eq!(rows[0], (TokenType::Var, String::new()));
    }

    #[test]
    fn var_braced_allows_arbitrary_characters() {
        // Inside `${…}` all characters except `}` are legal, including
        // spaces, `$`, `[`, etc.
        let (rows, _) = var_token_text("${weird name with spaces}");
        assert_eq!(rows[0], (TokenType::Var, "weird name with spaces".into()));
    }

    #[test]
    fn var_braced_unterminated_tokenises_best_effort() {
        // Missing `}` tokenises the remaining input as the variable
        // name; the non-fatal warning is not yet emitted.
        let (rows, _) = var_token_text("${unterminated");
        assert_eq!(rows[0], (TokenType::Var, "unterminated".into()));
    }

    // `${name}` brace-name parsing under the DEFAULT config follows C Tcl
    // 9.0.3's `Tcl_ParseVarName` (the project's reference standard — see
    // `docs/design/rust/engineering-guide.md` principle #0): inner `{…}` nests with brace
    // counting and `\X` is consumed as a literal pair, so the closer is
    // the first `}` at brace-depth zero. The expectations below were
    // confirmed against `tclsh9.0` (9.0.3) via the variable-name a failed
    // read reports — e.g. `${a\}b}` reads var `a\}b`, `${a{b}c}` reads var
    // `a{b}c`. Tcl 8.4/8.5/8.6 instead stop at the *first* `}` (their
    // `Tcl_ParseVarName` is `while (numBytes && (*src != '}'))`, no brace
    // counting, no backslash) — [`BracedVarStyle::FirstClose`], which every
    // 8.x-runtime dialect profile selects via `LexerConfig::for_dialect`
    // (dialect-profile-model.md).

    #[test]
    fn var_braced_escaped_close_brace_is_part_of_name() {
        // `${a\}b}` — the `\}` is a literal backslash + brace inside
        // the name, so the name is `a\}b` and the *real* closer is the
        // final `}`. Verified against tclsh 9.0.3.
        let (rows, _) = var_token_text(r"${a\}b}");
        assert_eq!(rows[0], (TokenType::Var, r"a\}b".into()));
        assert_eq!(rows.len(), 2); // VAR + synthetic EOL, no trailing ESC
    }

    #[test]
    fn var_braced_nested_braces_balance() {
        // `${a{b}c}` — the inner `{…}` is balanced and consumed as part
        // of the name; the name is `a{b}c`. Verified against tclsh 9.0.3.
        let (rows, _) = var_token_text("${a{b}c}");
        assert_eq!(rows[0], (TokenType::Var, "a{b}c".into()));
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn var_braced_deeply_nested_braces() {
        let (rows, _) = var_token_text("${a{b{c}d}e}");
        assert_eq!(rows[0], (TokenType::Var, "a{b{c}d}e".into()));
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn var_braced_name_ending_in_brace() {
        // `${a{b}}` — the inner `{b}` balances, so the name is `a{b}` and it
        // *ends* with the inner `}`; the outer `}` is the closer.
        // `token_text` must keep that trailing `}` (regression: it used to be
        // stripped unconditionally, yielding `a{b`). Verified against tclsh
        // 9.0.3 (`${a{b}}` reads var `a{b}`).
        let (rows, _) = var_token_text("${a{b}}");
        assert_eq!(rows[0], (TokenType::Var, "a{b}".into()));
        assert_eq!(rows.len(), 2); // VAR + synthetic EOL, no stray `}` word
    }

    #[test]
    fn var_braced_newline_in_name() {
        // A bare newline inside `${…}` is ordinary name content (it is not
        // a delimiter and not a backslash). Name = `a\nb`.
        let (rows, _) = var_token_text("${a\nb}");
        assert_eq!(rows[0], (TokenType::Var, "a\nb".into()));
    }

    #[test]
    fn var_braced_trailing_backslash_at_eof() {
        // A `\` with nothing after it is consumed best-effort without
        // panicking; the (unterminated) name is the lone backslash.
        let (rows, _) = var_token_text("${a\\");
        assert_eq!(rows[0], (TokenType::Var, "a\\".into()));
    }

    #[test]
    fn var_array_index() {
        let (rows, _) = var_token_text("$arr(idx)");
        // Span covers the whole `arr(idx)` — including the parens —
        // but not the leading `$`.
        assert_eq!(rows[0], (TokenType::Var, "arr(idx)".into()));
    }

    #[test]
    fn var_array_index_nested_parens() {
        // C Tcl does NOT nest parens in an array index — it
        // terminates at the first `)`. `$arr(one(two)three)` is the variable
        // `arr(one(two)` followed by the literal text `three)`.
        let (rows, _) = var_token_text("$arr(one(two)three)");
        assert_eq!(rows[0].0, TokenType::Var);
        assert_eq!(rows[0].1, "arr(one(two)");
        assert!(
            rows.iter().any(|(_, t)| t.contains("three")),
            "trailing `three)` must be a separate token: {rows:?}",
        );
    }

    #[test]
    fn var_array_index_first_paren_terminates() {
        // `$a((b)` ends the variable at the first `)`; the paren-counting bug
        // never reached depth 0 and swallowed the rest of the source.
        let (rows, _) = var_token_text("$a((b) rest");
        assert_eq!(rows[0].0, TokenType::Var);
        assert_eq!(rows[0].1, "a((b)");
    }

    #[test]
    fn var_array_index_escaped_paren_stays() {
        // `$a(x\)y)` — the escaped `)` is part of the index; the index ends at
        // the final, unescaped `)`.
        let (rows, _) = var_token_text("$a(x\\)y)");
        assert_eq!(rows[0].0, TokenType::Var);
        assert_eq!(rows[0].1, "a(x\\)y)");
    }

    #[test]
    fn var_array_index_command_sub_parens_do_not_terminate() {
        // A `)` inside a `[…]` command substitution is not the terminator.
        let (rows, _) = var_token_text("$a([foo(x)])");
        assert_eq!(rows[0].0, TokenType::Var);
        assert_eq!(rows[0].1, "a([foo(x)])");
    }

    #[test]
    fn var_array_index_command_sub_bracketed_brace_does_not_close_early() {
        // A `]` inside a braced word within the `[…]` substitution must not
        // close the substitution early (skip_command_in_index tracks brace
        // depth). `$a([puts {]}])` — the command is `puts {]}`, so the index is
        // the whole `[puts {]}]` and the VAR token is `a([puts {]}])`.
        let (rows, _) = var_token_text("$a([puts {]}])");
        assert_eq!(rows[0].0, TokenType::Var);
        assert_eq!(rows[0].1, "a([puts {]}])");
        // Same for a `]` inside a double-quoted word.
        let (rows, _) = var_token_text("$a([puts \"]\"])");
        assert_eq!(rows[0].0, TokenType::Var);
        assert_eq!(rows[0].1, "a([puts \"]\"])");
    }

    #[test]
    fn var_array_index_with_inner_braced_var() {
        // `${key}` inside the index scans to the matching `}` as a
        // unit — the `(` / `)` inside such a braced name would not
        // count against the array-index depth, so a
        // variable-named-with-parens doesn't fool the index
        // scanner.
        let (rows, _) = var_token_text("$arr(${key})");
        assert_eq!(rows[0], (TokenType::Var, "arr(${key})".into()));
    }

    #[test]
    fn var_array_index_unterminated_tokenises_best_effort() {
        let (rows, _) = var_token_text("$arr(idx");
        assert_eq!(rows[0], (TokenType::Var, "arr(idx".into()));
    }

    #[test]
    fn bare_dollar_is_an_str_token() {
        // A bare `$` is emitted as an STR token whose text is the
        // `$` character — not a VAR.
        let (rows, _) = var_token_text("$");
        assert_eq!(rows[0], (TokenType::Str, "$".into()));
    }

    #[test]
    fn bare_dollar_followed_by_space() {
        let (rows, _) = var_token_text("$ foo");
        assert_eq!(rows[0], (TokenType::Str, "$".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "foo".into()));
    }

    #[test]
    fn bare_dollar_followed_by_lf() {
        let (rows, _) = var_token_text("$\n");
        assert_eq!(rows[0], (TokenType::Str, "$".into()));
        assert_eq!(rows[1], (TokenType::Eol, "\n".into()));
    }

    #[test]
    fn var_followed_by_word() {
        // `$foo bar` — VAR then SEP then ESC.
        let (rows, _) = var_token_text("$foo bar");
        assert_eq!(rows[0], (TokenType::Var, "foo".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "bar".into()));
    }

    #[test]
    fn multiple_vars() {
        let (rows, _) = var_token_text("$a $b $c");
        assert_eq!(rows[0], (TokenType::Var, "a".into()));
        assert_eq!(rows[2], (TokenType::Var, "b".into()));
        assert_eq!(rows[4], (TokenType::Var, "c".into()));
    }

    #[test]
    fn var_resets_at_command_start() {
        // After a VAR token, `#` is no longer a comment opener.
        let (rows, _) = var_token_text("$foo #bar");
        assert_eq!(rows[0], (TokenType::Var, "foo".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        // `#bar` should be an ESC, not a COMMENT.
        assert_eq!(rows[2], (TokenType::Esc, "#bar".into()));
    }

    #[test]
    fn esc_stops_at_dollar() {
        // `foo$bar` — ESC "foo", VAR "bar". The `$` terminates the
        // bare word rather than being consumed as a literal.
        let (rows, _) = var_token_text("foo$bar");
        assert_eq!(rows[0], (TokenType::Esc, "foo".into()));
        assert_eq!(rows[1], (TokenType::Var, "bar".into()));
    }

    #[test]
    fn var_span_positions() {
        // `$foo bar` — the VAR span covers the whole `$foo` (offset
        // 0..4): the span starts at the `$` and ends at the last char
        // of the name. `token_text` is how you get just the "foo"
        // part.
        let lexer = Lexer::new("$foo bar");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let var = tokens.iter().find(|t| t.kind == TokenType::Var).unwrap();
        assert_eq!(var.span.start(), 0);
        assert_eq!(var.span.end(), 4);
        let (start, end) = map.range_positions(var.span);
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(3), 3));
        assert_eq!(map.token_text(*var), "foo");
    }

    #[test]
    fn braced_var_span_covers_delimiter_and_name() {
        // `${name}` — span is [0, 6), covering "${name" but NOT the
        // closing `}`. The lexer consumes the `}` so the next
        // dispatch starts at offset 7. `token_text` strips the `${`
        // wrapper so the visible text is "name".
        let lexer = Lexer::new("${name}");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let var = tokens.iter().find(|t| t.kind == TokenType::Var).unwrap();
        assert_eq!(var.span.start(), 0);
        assert_eq!(var.span.end(), 6);
        assert_eq!(map.text(var.span), "${name");
        assert_eq!(map.token_text(*var), "name");
    }

    #[test]
    fn into_source_map_round_trip() {
        let source = "a\nb\nc";
        let lexer = Lexer::new(source);
        let map = lexer.into_source_map();
        assert_eq!(map.line_index().line_count(), 3);
    }

    // Command substitution

    fn cmd_token_rows(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
        let lexer = Lexer::new(source);
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("L5 lexer accepts fixture");
        let rows = tokens
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned()))
            .collect();
        (rows, map)
    }

    #[test]
    fn cmd_simple_body() {
        let (rows, _) = cmd_token_rows("[+ 1 2]");
        assert_eq!(
            rows,
            vec![
                (TokenType::Cmd, "+ 1 2".into()),
                (TokenType::Eol, String::new()),
            ]
        );
    }

    #[test]
    fn cmd_empty_body() {
        // Degenerate `[]` — exercises the empty-body end-position clamp.
        let (rows, _) = cmd_token_rows("[]");
        assert_eq!(rows[0], (TokenType::Cmd, String::new()));
    }

    #[test]
    fn cmd_nested_brackets() {
        let (rows, _) = cmd_token_rows("[+ 1 [+ 2 3]]");
        assert_eq!(rows[0], (TokenType::Cmd, "+ 1 [+ 2 3]".into()));
    }

    #[test]
    fn cmd_braced_var_name_handles_escapes_and_nesting() {
        // `[set ${a\}] x}]` — the `${…}` braced variable name is `a\}] x`
        // (the `\}` is a literal pair, the `]` is inside the name). The
        // command-sub scan must sub-scan `${…}` with the same brace-nesting +
        // backslash-pair rules as `parse_var`, so the inner `]` does not close
        // the command early (issue 163). One CMD token spans the whole thing.
        let (rows, _) = cmd_token_rows(r"[set ${a\}] x}]");
        assert_eq!(rows[0], (TokenType::Cmd, r"set ${a\}] x}".into()));
        assert_eq!(rows[1], (TokenType::Eol, String::new()));
        // Brace nesting inside `${…}` is also honoured.
        let (rows2, _) = cmd_token_rows(r"[set ${a{b}c} 1]");
        assert_eq!(rows2[0], (TokenType::Cmd, r"set ${a{b}c} 1".into()));
    }

    #[test]
    fn cmd_deeply_nested_brackets() {
        let (rows, _) = cmd_token_rows("[a [b [c [d]]]]");
        assert_eq!(rows[0], (TokenType::Cmd, "a [b [c [d]]]".into()));
    }

    #[test]
    fn cmd_followed_by_word() {
        let (rows, _) = cmd_token_rows("[cmd] tail");
        assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "tail".into()));
    }

    #[test]
    fn word_then_cmd() {
        // `foo[cmd]` — ESC then CMD. The `[` terminates the bare
        // word.
        let (rows, _) = cmd_token_rows("foo[cmd]");
        assert_eq!(rows[0], (TokenType::Esc, "foo".into()));
        assert_eq!(rows[1], (TokenType::Cmd, "cmd".into()));
    }

    #[test]
    fn cmd_then_word() {
        let (rows, _) = cmd_token_rows("[cmd]tail");
        assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
        assert_eq!(rows[1], (TokenType::Esc, "tail".into()));
    }

    #[test]
    fn cmd_with_quoted_substring() {
        // `"..."` inside a command body is NOT a quoted string at
        // the top level — parse_command just tracks it for its
        // bracket-nesting state so that `]` inside the quotes does
        // not close the command.
        let (rows, _) = cmd_token_rows(r#"[puts "hello world"]"#);
        assert_eq!(rows[0], (TokenType::Cmd, r#"puts "hello world""#.into()));
    }

    #[test]
    fn cmd_with_bracket_inside_quotes_does_not_close() {
        let (rows, _) = cmd_token_rows(r#"[puts "a]b"]"#);
        assert_eq!(rows[0], (TokenType::Cmd, r#"puts "a]b""#.into()));
    }

    #[test]
    fn cmd_with_braced_substring() {
        // `{…}` inside a command body adjusts `blevel` so `]`
        // inside the braces does not close the command.
        let (rows, _) = cmd_token_rows("[list {a b c}]");
        assert_eq!(rows[0], (TokenType::Cmd, "list {a b c}".into()));
    }

    #[test]
    fn cmd_with_bracket_inside_braces_does_not_close() {
        let (rows, _) = cmd_token_rows("[list {a ] b}]");
        assert_eq!(rows[0], (TokenType::Cmd, "list {a ] b}".into()));
    }

    #[test]
    fn cmd_with_nested_braces() {
        let (rows, _) = cmd_token_rows("[list {a {nested} b}]");
        assert_eq!(rows[0], (TokenType::Cmd, "list {a {nested} b}".into()));
    }

    #[test]
    fn cmd_with_backslash_escape() {
        // `\]` inside the body is inert — it doesn't close the
        // command. The backslash consumes two bytes as a pair.
        let (rows, _) = cmd_token_rows(r"[a \] b]");
        assert_eq!(rows[0], (TokenType::Cmd, r"a \] b".into()));
    }

    #[test]
    fn cmd_with_backslash_quote() {
        // `\"` inside the body is inert — the quote state does not
        // toggle because the `"` is escaped.
        let (rows, _) = cmd_token_rows(r#"[a \" b]"#);
        assert_eq!(rows[0], (TokenType::Cmd, r#"a \" b"#.into()));
    }

    #[test]
    fn cmd_with_dollar_braced_var_inside() {
        // `${...}` inside the command body is sub-scanned so a `}`
        // inside the braced name does not throw off the counter.
        let (rows, _) = cmd_token_rows("[set ${odd}name value]");
        assert_eq!(rows[0], (TokenType::Cmd, "set ${odd}name value".into()));
    }

    #[test]
    fn cmd_with_plain_dollar_var_inside() {
        let (rows, _) = cmd_token_rows("[expr $a + $b]");
        assert_eq!(rows[0], (TokenType::Cmd, "expr $a + $b".into()));
    }

    #[test]
    fn cmd_multiline_body() {
        let (rows, _) = cmd_token_rows("[a\nb\nc]");
        assert_eq!(rows[0], (TokenType::Cmd, "a\nb\nc".into()));
    }

    #[test]
    fn cmd_comment_hides_closing_bracket() {
        // The `]` after `#` is in a command-position comment.  The command
        // token must therefore extend through the following command and end
        // at the final `]` (issue #1483).
        let (rows, _) = cmd_token_rows("[\n# ] hidden\nset y \"b\"\n]");
        assert_eq!(
            rows[0],
            (TokenType::Cmd, "\n# ] hidden\nset y \"b\"\n".into())
        );
    }

    #[test]
    fn cmd_comment_after_separator_hides_closing_bracket() {
        let (rows, _) = cmd_token_rows("[set x 1; # ] hidden\nset y 2]");
        assert_eq!(
            rows[0],
            (TokenType::Cmd, "set x 1; # ] hidden\nset y 2".into())
        );
    }

    #[test]
    fn cmd_hash_mid_command_is_not_a_comment() {
        // A hash after the first word is ordinary word content.  This keeps
        // the command-position guard from becoming an over-broad `#` rule.
        let (rows, _) = cmd_token_rows("[set #not-a-comment]");
        assert_eq!(rows[0], (TokenType::Cmd, "set #not-a-comment".into()));
    }

    #[test]
    fn cmd_hash_in_quote_or_brace_is_not_a_comment() {
        let (rows, _) = cmd_token_rows("[\"# ]\"]");
        assert_eq!(rows[0], (TokenType::Cmd, "\"# ]\"".into()));
        let (rows, _) = cmd_token_rows("[{# ]}]");
        assert_eq!(rows[0], (TokenType::Cmd, "{# ]}".into()));
    }

    #[test]
    fn cmd_comment_continuation_hides_closing_bracket() {
        let (rows, _) = cmd_token_rows("[\n# hidden \\\n] still comment\nset y 2]");
        assert_eq!(
            rows[0],
            (
                TokenType::Cmd,
                "\n# hidden \\\n] still comment\nset y 2".into()
            )
        );
    }

    #[test]
    fn cmd_unterminated_tokenises_best_effort() {
        let (rows, _) = cmd_token_rows("[unterminated");
        assert_eq!(rows[0], (TokenType::Cmd, "unterminated".into()));
    }

    #[test]
    fn cmd_span_positions() {
        // `[cmd] rest` — the CMD span covers the whole `[cmd]` range
        // (offset 0..4: start at `[`, end at the last char of the
        // body, `token_text` strips the leading `[`).
        let lexer = Lexer::new("[cmd] rest");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let cmd = tokens.iter().find(|t| t.kind == TokenType::Cmd).unwrap();
        assert_eq!(cmd.span.start(), 0);
        assert_eq!(cmd.span.end(), 4);
        let (start, end) = map.range_positions(cmd.span);
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(3), 3));
        assert_eq!(map.token_text(*cmd), "cmd");
    }

    #[test]
    fn standalone_closing_bracket_is_part_of_word() {
        // `foo]bar` — `]` is not a deferred character, so it's
        // included in the bare word.
        let (rows, _) = cmd_token_rows("foo]bar");
        assert_eq!(rows[0], (TokenType::Esc, "foo]bar".into()));
    }

    #[test]
    fn cmd_resets_at_command_start() {
        let (rows, _) = cmd_token_rows("[cmd] #not-a-comment");
        assert_eq!(rows[0], (TokenType::Cmd, "cmd".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "#not-a-comment".into()));
    }

    #[test]
    fn cmd_after_eol_allows_comment_before() {
        let (rows, _) = cmd_token_rows("# c\n[cmd]");
        assert!(rows.iter().any(|(k, _)| *k == TokenType::Comment));
        assert!(rows.iter().any(|(k, _)| *k == TokenType::Cmd));
    }

    // Braced strings

    fn str_token_rows(source: &str) -> (Vec<(TokenType, String)>, SourceMap<'_>) {
        let lexer = Lexer::new(source);
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("L6 lexer accepts fixture");
        let rows = tokens
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned()))
            .collect();
        (rows, map)
    }

    #[test]
    fn braced_simple_body() {
        let (rows, _) = str_token_rows("{hello world}");
        assert_eq!(
            rows,
            vec![
                (TokenType::Str, "hello world".into()),
                (TokenType::Eol, String::new()),
            ]
        );
    }

    #[test]
    fn braced_empty_body() {
        let (rows, _) = str_token_rows("{}");
        assert_eq!(rows[0], (TokenType::Str, String::new()));
    }

    #[test]
    fn braced_nested_once() {
        let (rows, _) = str_token_rows("{a {b c} d}");
        assert_eq!(rows[0], (TokenType::Str, "a {b c} d".into()));
    }

    #[test]
    fn braced_deeply_nested() {
        let (rows, _) = str_token_rows("{a {b {c {d}}}}");
        assert_eq!(rows[0], (TokenType::Str, "a {b {c {d}}}".into()));
    }

    #[test]
    fn braced_after_word() {
        // `proc foo {body}` — SEP after "foo" makes `{body}` a
        // newword, so the `{` starts a braced string.
        let (rows, _) = str_token_rows("proc foo {body}");
        assert_eq!(rows[0], (TokenType::Esc, "proc".into()));
        assert_eq!(rows[2], (TokenType::Esc, "foo".into()));
        assert_eq!(rows[4], (TokenType::Str, "body".into()));
    }

    #[test]
    fn braced_midword_is_regular_character() {
        // `foo{bar}` — the `{` is NOT at a word boundary (the
        // previous token was ESC, not SEP/EOL/STR/EXPAND), so it's
        // a regular character in the bare word.
        let (rows, _) = str_token_rows("foo{bar}");
        assert_eq!(rows[0], (TokenType::Esc, "foo{bar}".into()));
    }

    #[test]
    fn close_brace_midword_is_regular_character() {
        let (rows, _) = str_token_rows("foo}bar");
        assert_eq!(rows[0], (TokenType::Esc, "foo}bar".into()));
    }

    #[test]
    fn braced_multiline_body() {
        let (rows, _) = str_token_rows("{line1\nline2}");
        assert_eq!(rows[0], (TokenType::Str, "line1\nline2".into()));
    }

    #[test]
    fn braced_with_dollar_is_literal() {
        // `{foo $bar}` — `$` inside a braced string is a literal
        // character, not a variable substitution. This happens
        // naturally because the brace scanner only looks for
        // balanced `{` / `}` and backslashes.
        let (rows, _) = str_token_rows("{foo $bar}");
        assert_eq!(rows[0], (TokenType::Str, "foo $bar".into()));
    }

    #[test]
    fn braced_with_brackets_is_literal() {
        let (rows, _) = str_token_rows("{foo [bar]}");
        assert_eq!(rows[0], (TokenType::Str, "foo [bar]".into()));
    }

    #[test]
    fn braced_with_backslash_is_literal_pair() {
        // `{foo\nbar}` — the backslash and `n` are preserved in
        // the token text because braces are verbatim in Tcl. The
        // scanner skips them as a pair so `\}` does not count as
        // a close brace.
        let (rows, _) = str_token_rows(r"{foo\nbar}");
        assert_eq!(rows[0], (TokenType::Str, r"foo\nbar".into()));
    }

    #[test]
    fn braced_with_backslash_close_brace_is_inert() {
        let (rows, _) = str_token_rows(r"{a\}b}");
        assert_eq!(rows[0], (TokenType::Str, r"a\}b".into()));
    }

    #[test]
    fn braced_with_backslash_open_brace_is_inert() {
        let (rows, _) = str_token_rows(r"{a\{b}");
        assert_eq!(rows[0], (TokenType::Str, r"a\{b".into()));
    }

    #[test]
    fn braced_followed_by_word() {
        let (rows, _) = str_token_rows("{foo} bar");
        assert_eq!(rows[0], (TokenType::Str, "foo".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "bar".into()));
    }

    #[test]
    fn braced_then_braced() {
        // After a STR token, `newword` is True again (STR is in
        // the `newword` set alongside SEP/EOL/EXPAND), so a second
        // `{` starts another braced string.
        let (rows, _) = str_token_rows("{a}{b}");
        assert_eq!(rows[0], (TokenType::Str, "a".into()));
        assert_eq!(rows[1], (TokenType::Str, "b".into()));
    }

    #[test]
    fn braced_unterminated_tokenises_best_effort() {
        let (rows, _) = str_token_rows("{unterminated");
        assert_eq!(rows[0], (TokenType::Str, "unterminated".into()));
    }

    #[test]
    fn braced_at_command_start() {
        // A `{` at the start of a line (initial at_command_start=
        // true, initial last_kind=Eol → newword=true) is a STR.
        let (rows, _) = str_token_rows("{hello}");
        assert_eq!(rows[0], (TokenType::Str, "hello".into()));
    }

    #[test]
    fn braced_inside_command_substitution() {
        // `[list {a b}]` — the braces inside a CMD body are handled
        // by `parse_command`'s own `blevel` tracking, not by
        // `parse_brace`. The STR stays one CMD token.
        let (rows, _) = str_token_rows("[list {a b}]");
        assert_eq!(rows[0], (TokenType::Cmd, "list {a b}".into()));
    }

    #[test]
    fn braced_span_positions() {
        // `{hello}` — span covers `{hello` (0..6), token_text
        // strips `{`, end position is at the 'o' not the `}`.
        let lexer = Lexer::new("{hello}");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let brace = tokens.iter().find(|t| t.kind == TokenType::Str).unwrap();
        assert_eq!(brace.span.start(), 0);
        assert_eq!(brace.span.end(), 6);
        let (start, end) = map.range_positions(brace.span);
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(5), 5));
        assert_eq!(map.token_text(*brace), "hello");
    }

    #[test]
    fn braced_resets_at_command_start() {
        // After a STR, `at_command_start` is False so `#` is not
        // a comment.
        let (rows, _) = str_token_rows("{body} #not-a-comment");
        assert_eq!(rows[0], (TokenType::Str, "body".into()));
        assert_eq!(rows[1], (TokenType::Sep, " ".into()));
        assert_eq!(rows[2], (TokenType::Esc, "#not-a-comment".into()));
    }

    #[test]
    fn braced_preserves_newword_for_next_token() {
        // `{a}{b}{c}` — every STR sets `last_kind = Str`, which is
        // in the `newword` set, so subsequent `{`s start new
        // braced strings.
        let (rows, _) = str_token_rows("{a}{b}{c}");
        assert_eq!(rows[0], (TokenType::Str, "a".into()));
        assert_eq!(rows[1], (TokenType::Str, "b".into()));
        assert_eq!(rows[2], (TokenType::Str, "c".into()));
    }

    // Quoted strings

    fn quoted_rows(source: &str) -> (Vec<(TokenType, String, bool)>, SourceMap<'_>) {
        let lexer = Lexer::new(source);
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("L7 lexer accepts fixture");
        let rows = tokens
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned(), t.in_quote))
            .collect();
        (rows, map)
    }

    #[test]
    fn quoted_simple() {
        let (rows, _) = quoted_rows(r#""hello""#);
        assert_eq!(
            rows,
            vec![
                (TokenType::Esc, "hello".into(), false),
                (TokenType::Eol, String::new(), false),
            ]
        );
    }

    #[test]
    fn quoted_with_space() {
        let (rows, _) = quoted_rows(r#""hello world""#);
        assert_eq!(rows[0], (TokenType::Esc, "hello world".into(), false));
    }

    #[test]
    fn quoted_empty() {
        // `""` — empty body. The span covers both `"`s (end clamp)
        // and `token_text` returns `""`.
        let (rows, _) = quoted_rows(r#""""#);
        assert_eq!(rows[0], (TokenType::Esc, String::new(), false));
    }

    #[test]
    fn quoted_contains_braces_literally() {
        // Inside a quoted string, `{` and `}` are regular content,
        // NOT braced-string delimiters.
        let (rows, _) = quoted_rows(r#""hello {world}""#);
        assert_eq!(rows[0], (TokenType::Esc, "hello {world}".into(), false));
    }

    #[test]
    fn quoted_contains_separators_literally() {
        let (rows, _) = quoted_rows(r#""tab  spaces""#);
        assert_eq!(rows[0], (TokenType::Esc, "tab  spaces".into(), false));
    }

    #[test]
    fn quoted_with_hash_is_literal() {
        let (rows, _) = quoted_rows("\"# not a comment\"");
        assert_eq!(rows[0], (TokenType::Esc, "# not a comment".into(), false));
    }

    #[test]
    fn quoted_with_var_interpolation() {
        let (rows, _) = quoted_rows(r#""hello $foo world""#);
        assert_eq!(rows[0], (TokenType::Esc, "hello ".into(), true));
        assert_eq!(rows[1], (TokenType::Var, "foo".into(), true));
        assert_eq!(rows[2], (TokenType::Esc, " world".into(), false));
    }

    #[test]
    fn quoted_with_cmd_interpolation() {
        let (rows, _) = quoted_rows(r#""a [cmd] b""#);
        assert_eq!(rows[0], (TokenType::Esc, "a ".into(), true));
        assert_eq!(rows[1], (TokenType::Cmd, "cmd".into(), true));
        assert_eq!(rows[2], (TokenType::Esc, " b".into(), false));
    }

    #[test]
    fn quoted_with_var_and_cmd() {
        let (rows, _) = quoted_rows(r#""a $b [c] d""#);
        assert_eq!(rows[0], (TokenType::Esc, "a ".into(), true));
        assert_eq!(rows[1], (TokenType::Var, "b".into(), true));
        assert_eq!(rows[2], (TokenType::Esc, " ".into(), true));
        assert_eq!(rows[3], (TokenType::Cmd, "c".into(), true));
        assert_eq!(rows[4], (TokenType::Esc, " d".into(), false));
    }

    #[test]
    fn quoted_opening_empty_with_var() {
        // `"$foo"` — empty first ESC, then VAR, then empty
        // closing ESC.
        let (rows, _) = quoted_rows(r#""$foo""#);
        assert_eq!(rows[0], (TokenType::Esc, String::new(), true));
        assert_eq!(rows[1], (TokenType::Var, "foo".into(), true));
        assert_eq!(rows[2], (TokenType::Esc, String::new(), false));
    }

    #[test]
    fn quoted_opening_empty_with_cmd() {
        let (rows, _) = quoted_rows(r#""[cmd]""#);
        assert_eq!(rows[0], (TokenType::Esc, String::new(), true));
        assert_eq!(rows[1], (TokenType::Cmd, "cmd".into(), true));
        assert_eq!(rows[2], (TokenType::Esc, String::new(), false));
    }

    #[test]
    fn quoted_mid_word_is_regular_character() {
        // `foo"bar"` — the `"` is NOT at a word boundary, so it's
        // a regular character in the bare word.
        let (rows, _) = quoted_rows(r#"foo"bar""#);
        assert_eq!(rows[0], (TokenType::Esc, r#"foo"bar""#.into(), false));
    }

    #[test]
    fn quoted_after_esc_then_space_is_word_start() {
        let (rows, _) = quoted_rows(r#"foo "bar""#);
        assert_eq!(rows[0], (TokenType::Esc, "foo".into(), false));
        assert_eq!(rows[1], (TokenType::Sep, " ".into(), false));
        assert_eq!(rows[2], (TokenType::Esc, "bar".into(), false));
    }

    #[test]
    fn quoted_then_mid_word_quote() {
        // `"ab""cd"` — first `"ab"` is a quoted string, then `"cd"`
        // is NOT (the preceding token was an ESC, so `newword =
        // false`). The second run becomes a bare ESC containing
        // literal quote characters.
        let (rows, _) = quoted_rows(r#""ab""cd""#);
        assert_eq!(rows[0], (TokenType::Esc, "ab".into(), false));
        assert_eq!(rows[1], (TokenType::Esc, r#""cd""#.into(), false));
    }

    #[test]
    fn quoted_unterminated_tokenises_best_effort() {
        // `"abc` — no closing quote. The lexer scans "abc" and
        // leaves `in_quote = true`. The trailing ghost EOL
        // inherits `in_quote = false` because it uses the default
        // `Token::new` which sets `in_quote = false`.
        let (rows, _) = quoted_rows(r#""abc"#);
        assert_eq!(rows[0], (TokenType::Esc, "abc".into(), true));
    }

    #[test]
    fn quoted_multiline_body() {
        let (rows, _) = quoted_rows("\"line1\nline2\"");
        assert_eq!(rows[0], (TokenType::Esc, "line1\nline2".into(), false));
    }

    #[test]
    fn quoted_span_positions() {
        // `"hello"` — span covers `"hello`, end position is at the
        // last content char (the 'o'), not at the closing `"`.
        let lexer = Lexer::new(r#""hello""#);
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let esc = tokens.iter().find(|t| t.kind == TokenType::Esc).unwrap();
        assert_eq!(esc.span.start(), 0);
        assert_eq!(esc.span.end(), 6);
        let (start, end) = map.range_positions(esc.span);
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(5), 5));
        assert_eq!(map.token_text(*esc), "hello");
    }

    #[test]
    fn quoted_inside_cmd_is_managed_by_parse_command() {
        // `[puts "hello"]` — the `"…"` inside a command body is
        // handled by `parse_command`'s own `in_quotes` tracking,
        // not by the outer `in_quote` state. The result is a
        // single CMD token.
        let (rows, _) = quoted_rows(r#"[puts "hello"]"#);
        assert_eq!(rows[0], (TokenType::Cmd, r#"puts "hello""#.into(), false));
    }

    #[test]
    fn quoted_resets_at_command_start() {
        let (rows, _) = quoted_rows(r#""body" #not-a-comment"#);
        assert_eq!(rows[0], (TokenType::Esc, "body".into(), false));
        assert_eq!(rows[1], (TokenType::Sep, " ".into(), false));
        assert_eq!(rows[2], (TokenType::Esc, "#not-a-comment".into(), false));
    }

    #[test]
    fn quoted_in_quote_propagates_to_sub_tokens() {
        // Verify explicitly that sub-tokens inside a quoted run
        // carry `in_quote = true`.
        let (rows, _) = quoted_rows(r#""$a [b] $c""#);
        assert_eq!(rows[0], (TokenType::Esc, String::new(), true));
        assert_eq!(rows[1], (TokenType::Var, "a".into(), true));
        assert_eq!(rows[2], (TokenType::Esc, " ".into(), true));
        assert_eq!(rows[3], (TokenType::Cmd, "b".into(), true));
        assert_eq!(rows[4], (TokenType::Esc, " ".into(), true));
        assert_eq!(rows[5], (TokenType::Var, "c".into(), true));
        assert_eq!(rows[6], (TokenType::Esc, String::new(), false));
    }

    // `{*}` expansion prefix + dialect flags

    fn expand_rows(source: &str) -> Vec<(TokenType, String)> {
        let lexer = Lexer::new(source);
        let map = lexer.source_map().clone();
        lexer
            .tokenise_all()
            .expect("L8 lexer accepts fixture")
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned()))
            .collect()
    }

    fn expand_rows_no_expand(source: &str) -> Vec<(TokenType, String)> {
        let config = LexerConfig {
            expand_syntax: false,
            ..LexerConfig::default()
        };
        let map = SourceMap::new(source);
        let lexer = Lexer::with_source_map(map.clone(), config);
        lexer
            .tokenise_all()
            .expect("L8 lexer accepts fixture")
            .iter()
            .map(|t| (t.kind, map.token_text(*t).to_owned()))
            .collect()
    }

    #[test]
    fn expand_prefix_before_bare_word() {
        let rows = expand_rows("{*}list");
        assert_eq!(rows[0], (TokenType::Expand, String::new()));
        assert_eq!(rows[1], (TokenType::Esc, "list".into()));
    }

    #[test]
    fn expand_prefix_before_var() {
        let rows = expand_rows("{*}$var");
        assert_eq!(rows[0], (TokenType::Expand, String::new()));
        assert_eq!(rows[1], (TokenType::Var, "var".into()));
    }

    #[test]
    fn expand_prefix_before_cmd() {
        let rows = expand_rows("{*}[cmd]");
        assert_eq!(rows[0], (TokenType::Expand, String::new()));
        assert_eq!(rows[1], (TokenType::Cmd, "cmd".into()));
    }

    #[test]
    fn expand_prefix_before_braced() {
        let rows = expand_rows("{*}{a b}");
        assert_eq!(rows[0], (TokenType::Expand, String::new()));
        assert_eq!(rows[1], (TokenType::Str, "a b".into()));
    }

    #[test]
    fn expand_prefix_mid_command() {
        let rows = expand_rows("cmd {*}$args");
        assert_eq!(rows[0], (TokenType::Esc, "cmd".into()));
        assert_eq!(rows[2], (TokenType::Expand, String::new()));
        assert_eq!(rows[3], (TokenType::Var, "args".into()));
    }

    #[test]
    fn expand_followed_by_separator_is_braced_string() {
        // `{*} list` — the space after `}` means it's NOT an
        // expansion prefix; it's a braced string `{*}` = STR("*").
        let rows = expand_rows("{*} list");
        assert_eq!(rows[0], (TokenType::Str, "*".into()));
    }

    #[test]
    fn expand_at_eol_is_braced_string() {
        // `{*}` alone (at EOF) — nothing follows, so it's STR("*").
        let rows = expand_rows("{*}");
        assert_eq!(rows[0], (TokenType::Str, "*".into()));
    }

    #[test]
    fn expand_syntax_disabled_always_parses_as_brace() {
        // With `expand_syntax = false`, `{*}list` is STR("*")
        // followed by ESC("list").
        let rows = expand_rows_no_expand("{*}list");
        assert_eq!(rows[0], (TokenType::Str, "*".into()));
        assert_eq!(rows[1], (TokenType::Esc, "list".into()));
    }

    #[test]
    fn expand_newword_is_true_after_expand() {
        // After EXPAND, `newword` should be true (EXPAND is in the
        // `is_newword` set), so a `{` starts a braced string.
        let rows = expand_rows("{*}{a b}");
        assert_eq!(rows[0], (TokenType::Expand, String::new()));
        assert_eq!(rows[1], (TokenType::Str, "a b".into()));
    }

    #[test]
    fn expand_span_is_empty() {
        // The EXPAND token's span covers just the `{` (one byte) —
        // a zero-width marker.
        let lexer = Lexer::new("{*}list");
        let map = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().unwrap();
        let expand = tokens.iter().find(|t| t.kind == TokenType::Expand).unwrap();
        assert_eq!(expand.span.start(), 0);
        assert_eq!(expand.span.end(), 0);
        let (start, end) = map.range_positions(expand.span);
        assert_eq!(start, SourcePosition::new(0, ByteCol::new(0), 0));
        assert_eq!(end, SourcePosition::new(0, ByteCol::new(0), 0));
    }

    #[test]
    fn for_dialect_tcl84_disables_expand_syntax() {
        let cfg = LexerConfig::for_dialect("tcl8.4");
        assert!(!cfg.expand_syntax);
        assert!(!cfg.irules_brace_separator);
    }

    #[test]
    fn for_dialect_tcl86_keeps_defaults() {
        let cfg = LexerConfig::for_dialect("tcl8.6");
        assert!(cfg.expand_syntax);
        assert!(!cfg.irules_brace_separator);
    }

    #[test]
    fn for_dialect_irules_enables_brace_separator() {
        let cfg = LexerConfig::for_dialect("f5-irules");
        assert!(!cfg.expand_syntax);
        assert!(cfg.irules_brace_separator);
    }

    #[test]
    fn for_dialect_unknown_falls_back_to_defaults() {
        let cfg = LexerConfig::for_dialect("not-a-real-dialect");
        let default = LexerConfig::default();
        assert_eq!(cfg.expand_syntax, default.expand_syntax);
        assert_eq!(cfg.irules_brace_separator, default.irules_brace_separator);
        assert_eq!(cfg.braced_var, default.braced_var);
    }

    #[test]
    fn for_dialect_braced_var_follows_the_embedded_runtime() {
        // 8.x runtimes — plain, F5 (tmsh included), EDA, and Expect (an
        // embedded Tcl 8.6) — use the first-close `${…}` rule. Expect and
        // f5-tmsh are the dialect-profile fix: the old string-keyed table
        // missed them and fell through to the modern nesting rule.
        for d in [
            "tcl8.4",
            "tcl8.5",
            "tcl8.6",
            "f5-irules",
            "f5-iapps",
            "f5-tmsh",
            "expect",
            "xilinx-eda-tcl",
            "synopsys-eda-tcl",
        ] {
            assert_eq!(
                LexerConfig::for_dialect(d).braced_var,
                BracedVarStyle::FirstClose,
                "{d}"
            );
        }
        // 9.x runtimes nest — bpf embeds Tcl 9.0 (D7).
        for d in ["tcl9.0", "tcl9.1", "bpf"] {
            assert_eq!(
                LexerConfig::for_dialect(d).braced_var,
                BracedVarStyle::Tcl9Nesting,
                "{d}"
            );
        }
    }

    #[test]
    fn expect_braced_var_lexes_to_first_close() {
        // Behavioural proof of the flip, not just config plumbing:
        // `${a{b}c}` under expect names the variable `a{b` (the 8.x
        // first-close rule, 8.6.14 tclParse.c:1466) — the trailing `c}` is
        // ordinary word text.
        let src = "set x ${a{b}c}\n";
        let cfg = LexerConfig::for_dialect("expect");
        let map = SourceMap::new(src);
        let tokens = Lexer::with_source_map(map, cfg)
            .tokenise_all()
            .expect("lexes");
        let sm = SourceMap::new(src);
        let var_texts: Vec<&str> = tokens
            .iter()
            .filter(|t| t.kind == TokenType::Var)
            .map(|t| sm.token_text(*t))
            .collect();
        assert_eq!(var_texts, ["a{b"], "expect uses the first-close rule");
    }

    // ghost-token recovery

    #[test]
    fn ghost_bracket_terminates_unclosed_command() {
        // `set x [foo` — inject a ghost `]` at offset 10 (one past
        // "foo"). The lexer should lex a terminated Cmd token spanning
        // `[foo` and emit no "missing close-bracket" warning.
        let src = "set x [foo";
        let mut ghosts = std::collections::BTreeMap::new();
        ghosts.insert(10u32, b']');
        let lexer = Lexer::new(src).with_ghosts(ghosts);
        let (tokens, warnings) = lexer.tokenise_all_with_warnings().expect("lex ok");
        let cmd = tokens
            .iter()
            .find(|t| t.kind == TokenType::Cmd)
            .expect("a Cmd token");
        // The ghost `]` is zero-width, so the Cmd span ends at the ghost
        // offset (no extra byte consumed) and no real char is skipped.
        assert_eq!(cmd.span.start(), 6);
        assert_eq!(cmd.span.end(), 10);
        assert!(
            warnings
                .iter()
                .all(|w| w.message != "missing close-bracket"),
            "ghost should suppress the unterminated warning: {warnings:?}",
        );
    }

    #[test]
    fn ghost_bracket_at_eof_with_deep_nesting_stays_in_bounds() {
        // A ghost `]` consumed while nesting is ≥ 2 must be
        // zero-width — it must not advance `pos` past the real byte (here past
        // EOF to `len + 1`), which produced a token span one past the buffer and
        // panicked `SourceMap::text`.
        let src = "[a[b"; // outer `[` (level 1) then inner `[b` (level 2)
        let len = u32::try_from(src.len()).unwrap();
        let mut ghosts = std::collections::BTreeMap::new();
        ghosts.insert(len, b']'); // a single ghost at EOF — closes one level
        let lexer = Lexer::new(src).with_ghosts(ghosts);
        let (tokens, _warnings) = lexer.tokenise_all_with_warnings().expect("lex ok");
        for t in &tokens {
            assert!(
                t.span.end() <= len,
                "token span end {} exceeds source length {len}: {t:?}",
                t.span.end()
            );
        }
    }

    #[test]
    fn ghost_bracket_mid_stream_with_nesting_does_not_skip_real_byte() {
        // A ghost `]` at an interior offset with level ≥ 2 must not swallow the
        // real byte sitting at that offset.
        let src = "[a[bc"; // `[`0 `a`1 `[`2 `b`3 `c`4
        let mut ghosts = std::collections::BTreeMap::new();
        ghosts.insert(3u32, b']'); // ghost at `b`, level is 2 here
        let lexer = Lexer::new(src).with_ghosts(ghosts);
        let (tokens, _warnings) = lexer.tokenise_all_with_warnings().expect("lex ok");
        // The `b` byte must still fall within the emitted Cmd token (not skipped
        // out of the span).
        let cmd = tokens
            .iter()
            .find(|t| t.kind == TokenType::Cmd)
            .expect("a Cmd token");
        assert!(cmd.span.end() <= u32::try_from(src.len()).unwrap());
    }

    #[test]
    fn ghost_bracket_splits_swallowed_following_command() {
        // `[foo bar\nputs done` with a ghost `]` after "bar" re-lexes as
        // a terminated `[foo bar]` followed by a clean `puts done`.
        let src = "[foo bar\nputs done";
        let mut ghosts = std::collections::BTreeMap::new();
        ghosts.insert(8u32, b']'); // one past "bar", at the '\n'
        let lexer = Lexer::new(src).with_ghosts(ghosts);
        let (tokens, _) = lexer.tokenise_all_with_warnings().expect("lex ok");
        // The first Cmd token covers `[foo bar` (zero-width close at 8).
        let cmd = tokens.iter().find(|t| t.kind == TokenType::Cmd).unwrap();
        assert_eq!((cmd.span.start(), cmd.span.end()), (0, 8));
        // `puts` and `done` are lexed as ordinary words after the split,
        // with their original absolute offsets intact.
        let words: Vec<&str> = tokens
            .iter()
            .filter(|t| t.kind == TokenType::Esc)
            .map(|t| &src[t.span.start() as usize..t.span.end() as usize])
            .collect();
        assert!(words.contains(&"puts"), "{words:?}");
        assert!(words.contains(&"done"), "{words:?}");
    }

    #[test]
    fn no_ghosts_is_identical_to_plain_lexing() {
        let src = "set x [foo]\nputs $x\n";
        let plain = Lexer::new(src).tokenise_all().unwrap();
        let with_empty = Lexer::new(src)
            .with_ghosts(std::collections::BTreeMap::new())
            .tokenise_all()
            .unwrap();
        assert_eq!(plain, with_empty);
    }

    /// Regression coverage for issue #996: `scan_array_index_body` and
    /// `skip_var_in_index` are mutually recursive on nested `$a($b($c(…)))`
    /// array-index references, with no depth cap before this fix.
    /// Empirically, unguarded input overflowed the native stack (SIGABRT)
    /// around depth 20,000-25,000 on a 2 MiB thread (`cargo test`'s
    /// per-test default). 5000 is comfortably past both that crash range
    /// and `MAX_ARRAY_INDEX_DEPTH` (64); the assertion is that lexing
    /// returns at all, not what it returns.
    #[test]
    fn deeply_nested_array_index_survives_lexing() {
        const DEPTH: usize = 5000;
        let mut src = String::from("set x $a0");
        for i in 0..DEPTH {
            src.push('(');
            let _ = write!(src, "a{}", i + 1);
        }
        src.push('1');
        for _ in 0..DEPTH {
            src.push(')');
        }
        src.push('\n');
        let _ = Lexer::new(&src).tokenise_all_with_warnings();
    }

    /// A moderately nested array index (well under `MAX_ARRAY_INDEX_DEPTH`)
    /// still lexes as a single `Var` token — the safety net must not fire
    /// on realistic nesting depths.
    #[test]
    fn moderately_nested_array_index_still_lexes_as_one_var() {
        let src = "set x $a($b($c(1)))\n";
        let lexed = Lexed::run(src);
        let var = lexed
            .tokens
            .iter()
            .zip(lexed.texts())
            .find(|(t, _)| t.kind == TokenType::Var)
            .expect("expected a Var token");
        assert_eq!(var.1, "$a($b($c(1)))");
    }

    // `Lexer::as_quoted_body` — issue #923 idx 125.

    #[test]
    fn as_quoted_body_treats_embedded_braces_as_literal_content() {
        // `{$a}` is ordinary literal-then-substitution content, not a fresh
        // brace-quoted word: no `Str` token at all, and `$a` surfaces as its
        // own `Var` token.
        let src = "prefix {$a} suffix";
        let lexer = Lexer::new(src).as_quoted_body();
        let sm = lexer.source_map().clone();
        let tokens = lexer
            .tokenise_all()
            .expect("quoted-body lexing is best-effort, never fails");
        assert!(
            !tokens.iter().any(|t| t.kind == TokenType::Str),
            "no token should be brace-quoted in quoted-body mode: {tokens:?}"
        );
        let var = tokens
            .iter()
            .find(|t| t.kind == TokenType::Var)
            .expect("$a must surface as its own Var token");
        assert_eq!(sm.token_text(*var), "a");
    }

    #[test]
    fn default_lexing_of_the_same_text_brace_quotes_it_instead() {
        // Contrast case: without `as_quoted_body`, the identical text is
        // lexed as fresh top-level command words, so `{$a}` is one
        // non-substituting `Str` token and `$a` never becomes a `Var` token
        // at all — this is the exact mis-tokenisation idx 125 fixed by
        // giving callers `as_quoted_body` to opt out of.
        let src = "prefix {$a} suffix";
        let tokens = Lexer::new(src).tokenise_all().expect("lexes fine");
        assert!(
            tokens.iter().any(|t| t.kind == TokenType::Str),
            "the default top-level lexer should brace-quote {{$a}}: {tokens:?}"
        );
        assert!(
            !tokens.iter().any(|t| t.kind == TokenType::Var),
            "so $a must NOT surface as a Var token: {tokens:?}"
        );
    }

    #[test]
    fn as_quoted_body_still_dispatches_dollar_and_bracket_normally() {
        let src = "$x [foo $y]";
        let lexer = Lexer::new(src).as_quoted_body();
        let sm = lexer.source_map().clone();
        let tokens = lexer.tokenise_all().expect("lexes fine");
        let var_texts: Vec<&str> = tokens
            .iter()
            .filter(|t| t.kind == TokenType::Var)
            .map(|t| sm.token_text(*t))
            .collect();
        assert_eq!(var_texts, vec!["x"], "only the top-level $x is a Var here");
        let cmd = tokens
            .iter()
            .find(|t| t.kind == TokenType::Cmd)
            .expect("[foo $y] must still be recognised as a Cmd token");
        assert_eq!(sm.token_text(*cmd), "foo $y");
    }

    #[test]
    fn as_quoted_body_treats_the_missing_close_quote_as_a_soft_warning() {
        // There is no real closing `"` to find (the caller already stripped
        // it, if there ever was one) — this must be a best-effort, non-fatal
        // warning under the default (non-`strict_quoting`) config, not a
        // `LexError`, since every value-body scan otherwise relies on
        // `tokenise_all()` succeeding.
        let lexer = Lexer::new("plain bareword value").as_quoted_body();
        let result = lexer.tokenise_all();
        assert!(
            result.is_ok(),
            "a missing close-quote must not be a hard error by default: {result:?}"
        );
    }

    #[test]
    fn smoke_lex_canonical_snippet() {
        // Word, braced word, var substitution, command substitution, and a
        // comment, in one pass — the cheapest "the lexer still starts up and
        // produces sane output" check.
        let lexed = Lexed::run("set x {a b}\nputs $x ;# comment\nset y [llength $x]\n");
        let kinds = lexed.kinds();
        assert!(kinds.contains(&TokenType::Esc), "{kinds:?}");
        assert!(kinds.contains(&TokenType::Str), "{kinds:?}");
        assert!(kinds.contains(&TokenType::Var), "{kinds:?}");
        assert!(kinds.contains(&TokenType::Cmd), "{kinds:?}");
        assert!(kinds.contains(&TokenType::Comment), "{kinds:?}");
    }
}
