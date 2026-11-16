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

//! The lexer configuration: the grammar axes, the dialect and file presets and the leading byte-order mark.

use tcl_dialect::{ArrayIndexSyntax, BracedVarStyle};

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
    /// `docs/design/f5/bigip-irule-parser-measurements.md` §1, §3): a word
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
    /// `docs/design/f5/bigip-irule-parser-measurements.md` §2): under
    /// [`BraceLineContinuation::Continues`](crate::BraceLineContinuation::Continues),
    /// a newline whose next line's first non-whitespace character is `{`
    /// does not terminate the command — it lexes as a SEP instead of an
    /// EOL, unconditionally (N2) and at any nesting depth (N3, via body
    /// re-lexing under the same config). Blank, whitespace-only, and
    /// comment lines still terminate (N4); backslash-newline handling is
    /// unchanged. An `f5-tcl` trunk axis.
    pub brace_line_continuation: tcl_dialect::BraceLineContinuation,
    /// When true, certain unterminated constructs (missing
    /// close-brace, missing close-bracket, extra chars after
    /// close-quote/brace) are reported as `LexError` instead of
    /// best-effort warnings. Used by the VM's compilation path.
    pub strict_quoting: bool,
    /// How a `${…}` variable name is delimited — see [`BracedVarStyle`].
    pub braced_var: BracedVarStyle,
    /// Which literal source bytes an array-index read accepts — see
    /// [`ArrayIndexSyntax`].
    pub array_index: ArrayIndexSyntax,
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
    /// Which characters end a command word — see
    /// [`tcl_dialect::WordSeparators`]. `JimTcl` omits `\v` from the set;
    /// every build of the Tcl core includes it.
    pub word_separators: tcl_dialect::WordSeparators,
    /// Whether text may follow a word's closing `"` — see
    /// [`tcl_dialect::QuoteTermination`]. Under `Concatenating` the
    /// "extra characters after close-quote" diagnostic never fires and the
    /// following run joins the same word.
    pub quote_termination: tcl_dialect::QuoteTermination,
    /// How a `$` substitution's name, index and sugar forms are parsed —
    /// see [`tcl_dialect::VarSyntax`].
    pub var_syntax: tcl_dialect::VarSyntax,
    /// What a backslash-newline inside a `{braced}` word becomes — see
    /// [`tcl_dialect::BraceBackslashNewline`].
    ///
    /// The lexer never folds anything, so this does not move a token
    /// boundary; it rides here for the same reason [`Self::escapes`] does —
    /// the consumers that build a braced word's *value* already thread a
    /// `LexerConfig` from the dialect grammar.
    pub brace_backslash_newline: tcl_dialect::BraceBackslashNewline,
    /// Whether malformed *list* text raises or is split anyway. The lexer
    /// itself never splits a list — this rides on the config so the value
    /// consumers that do (`tcl_syntax::list`, via lowering's word-list
    /// helpers) can read the document's answer from the same place they read
    /// every other lexical axis, instead of defaulting to `Strict`.
    pub list_parse: tcl_dialect::ListParse,
    /// How the dialect's `source` reads a script file's line endings — see
    /// [`tcl_dialect::SourceLineEndings`]. It applies only to a
    /// [`SourceChannel::Document`](crate::SourceChannel::Document) buffer:
    /// under `TranslateAuto` a lone CR ends a command or comment, as in C Tcl,
    /// and under `Preserve` (`JimTcl`) it stays a word separator. A native
    /// value is never translated.
    pub source_line_endings: tcl_dialect::SourceLineEndings,
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
/// skips it, Tcl 8.x's does not).
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
            array_index: ArrayIndexSyntax::Tcl9,
            base_offset: 0,
            base_line: 0,
            base_col: 0,
            leading_bom: LeadingBom::Content,
            escapes: tcl_dialect::EscapeSyntax::Tcl90,
            word_separators: tcl_dialect::WordSeparators::Tcl,
            quote_termination: tcl_dialect::QuoteTermination::Strict,
            var_syntax: tcl_dialect::VarSyntax::Tcl,
            brace_backslash_newline: tcl_dialect::BraceBackslashNewline::Folds,
            list_parse: tcl_dialect::ListParse::Strict,
            source_line_endings: tcl_dialect::SourceLineEndings::TranslateAuto,
        }
    }
}

/// The UTF-8 encoding of U+FEFF, the byte-order mark.
pub const UTF8_BOM: &str = "\u{FEFF}";

impl LexerConfig {
    /// Select native lexical axes without changing the source location or
    /// caller's parser mode. Use this before segmentation: changing the
    /// grammar after words have been split cannot repair their components.
    #[must_use]
    pub fn with_grammar(self, grammar: tcl_dialect::LexerGrammar) -> Self {
        Self {
            base_offset: self.base_offset,
            base_line: self.base_line,
            base_col: self.base_col,
            strict_quoting: self.strict_quoting,
            leading_bom: self.leading_bom,
            ..Self::from_grammar(grammar)
        }
    }

    /// Return the same lexical policy in local source coordinates.
    ///
    /// Cache keys for relocated/nested bodies must retain every semantic
    /// grammar knob while discarding only the caller's position bookkeeping;
    /// otherwise moving an unchanged body would mint a different key.  BOM and
    /// strictness policy are intentionally preserved because they can change
    /// how the bytes are read.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self {
            base_offset: 0,
            base_line: 0,
            base_col: 0,
            ..self
        }
    }

    /// Apply this config's lexical axes to an independently selected grammar.
    /// Numeral and expression policies remain those of `base`: a lexer config
    /// does not identify a native interpreter or carry those semantic axes.
    #[must_use]
    pub const fn grammar_over(self, base: tcl_dialect::LexerGrammar) -> tcl_dialect::LexerGrammar {
        tcl_dialect::LexerGrammar {
            expand_syntax: self.expand_syntax,
            irules_brace_separator: self.irules_brace_separator,
            brace_line_continuation: self.brace_line_continuation,
            braced_var: self.braced_var,
            array_index: self.array_index,
            escapes: self.escapes,
            word_separators: self.word_separators,
            quote_termination: self.quote_termination,
            var_syntax: self.var_syntax,
            brace_backslash_newline: self.brace_backslash_newline,
            list_parse: self.list_parse,
            source_line_endings: self.source_line_endings,
            ..base
        }
    }

    /// Build a config from a dialect profile's
    /// [`LexerGrammar`](tcl_dialect::LexerGrammar) — the dialect-derived
    /// fields come from the grammar; the call-site knobs (strict quoting,
    /// sub-lexing offsets) keep their defaults.
    #[must_use]
    pub fn from_grammar(grammar: tcl_dialect::LexerGrammar) -> Self {
        Self {
            expand_syntax: grammar.expand_syntax,
            irules_brace_separator: grammar.irules_brace_separator,
            brace_line_continuation: grammar.brace_line_continuation,
            braced_var: grammar.braced_var,
            array_index: grammar.array_index,
            escapes: grammar.escapes,
            word_separators: grammar.word_separators,
            quote_termination: grammar.quote_termination,
            var_syntax: grammar.var_syntax,
            brace_backslash_newline: grammar.brace_backslash_newline,
            list_parse: grammar.list_parse,
            source_line_endings: grammar.source_line_endings,
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
    ///   (`docs/design/f5/bigip-irule-parser-measurements.md` §4a) showed
    ///   `f5-tmsh` and `f5-iapps` are 8.4.6 forks like `f5-irules`, not 8.5
    ///   embeds, and on all three `{*}$l` lexes as a literal `*` plus the
    ///   unexpanded word.
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
    /// Resolves through [`tcl_dialect::grammar_of_dialect_name`] — the one
    /// place a name becomes a grammar — so an environment with no catalogue
    /// row (`jim`) lexes under its own grammar here exactly as it does
    /// through the ingress. Unknown names resolve to the permissive default
    /// (modern-Tcl semantics) so a typo in a workspace's `languageId`
    /// doesn't change parsing behaviour.
    #[must_use]
    pub fn for_dialect(dialect: &str) -> Self {
        Self::from_grammar(tcl_dialect::grammar_of_dialect_name(Some(dialect)))
    }

    /// The body-lexing config for the profile a compiler layer threads —
    /// `profile.grammar` when there is one, the default grammar for a
    /// dialect-less compile. This is the one constructor a re-lex of
    /// document text (a body, a list word, a braced argument) should use
    /// when its context carries an `Option<&DialectProfile>`; the grammar is
    /// read off the profile and never resolved again from its name.
    #[must_use]
    pub fn for_profile(profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        Self::from_grammar(profile.map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar))
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
    /// document links / inlay hints / references miss.
    #[must_use]
    pub fn for_file_dialect(dialect: &str) -> Self {
        Self::for_file_grammar(tcl_dialect::grammar_of_dialect_name(Some(dialect)))
    }

    /// This config demoted to a **nested** re-lex: identical, except a leading
    /// byte-order mark is ordinary content again.
    ///
    /// A whole-file config may skip a mark at byte 0 because that is the
    /// script's prologue; a mark at the head of a nested body slice, an `eval`
    /// argument, or a VM string is data. A provider that threads one config
    /// through its own body recursion calls this below the top level, so the
    /// file rule cannot leak into a nested slice.
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
