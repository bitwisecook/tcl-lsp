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

//! Shared decomposition of written words, substitution templates and array indices.
//!
//! The scanner uses the native variable, array-index, quote and bracket boundary
//! owners and one literal escape decoder. `WordBody`/`WordPart` provide a borrowed
//! advisory tree; index nesting beyond its retained budget has no executable
//! authority. `decompose_spanned_checked` separately reports that capability
//! boundary instead of accepting advisory literal fallback.
//!
//! `ExecutablePartArena` is the full-depth executable owner. It retains one exact
//! `SourceImage`, original component/name/body spans and ordered list IDs for
//! array-index children. Construction queues indices through the same scanner,
//! without recursively allocating or destroying a component tree. Consumers
//! traverse lists explicitly and preserve native evaluation order.
//!
//! Syntax errors remain parts after preceding components. A reached `subst`
//! template may execute earlier substitutions before reporting that error.
//! A script parser validates every word before argument evaluation and reports
//! the first authentic syntax error before dispatch. The command grouper still
//! owns whole-word boundaries; decomposition owns only their components.

mod cursor;
pub use cursor::{
    ArgumentCursor, VariableCursorPrefix, VariableCursorUnavailable, WordCursorPrefix,
    WordCursorUnavailable, argument_cursor_in, variable_cursor_prefix_in, word_cursor_prefix_in,
};

use std::borrow::Cow;

use tcl_dialect::EscapeSyntax;

use crate::ParseIssueKind;
use crate::lexer::LexerConfig;
use crate::ranges::{ArrayIndexEnd, BracedVarEnd, braced_var_name_end, scan_array_index_in};
use crate::substitution::backslash_subst_bytes_in;

pub use crate::executable_parts::{
    ExecutableInput, ExecutablePart, ExecutablePartArena, ExecutablePartsUnavailable,
    ExecutableText, PartListId, SpannedExecutablePart,
};
pub use crate::native_word::{NativeWord, NativeWordError};

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

/// C Tcl's error for text welded straight onto a `{…}` word's close-brace
/// (`Tcl_ParseCommand` → `TCL_PARSE_BRACE_EXTRA`, `tclParse.c`: after
/// `Tcl_ParseBraces` returns, the next byte must be a separator or a command
/// terminator).
///
/// Measured on 8.6.16 and 9.0.4: `{a}b`, `{a}$b`, `{a}[b]`, `{}x`, `{a}{b}`
/// and `{a}{*}$b` all report `extra characters after close-brace`, and the
/// error is raised while the *command* is parsed — no word of that command is
/// substituted (`list [side] {a}b` never runs `side`) — though earlier
/// commands of the same script have already run, since `Tcl_EvalEx` parses one
/// command at a time.
///
/// The lexer reports the same text as a recoverable warning
/// (`Lexer::tokenise_all_with_warnings`) because the LSP must keep tokenising
/// broken source; `tcl_lexer::script::WordSpan::welded_after_close` is the
/// boundary owner's spelling of the same fact, and `runtime/rust` turns it
/// into this error.
pub const EXTRA_AFTER_CLOSE_BRACE: &str = "extra characters after close-brace";

/// C Tcl's error for a `$name(` array reference whose index never closes
/// (`Tcl_ParseVarName` → `TCL_PARSE_MISSING_PAREN`, `tclParse.c`).
/// `subst {$x(}` reports `missing )` on 8.6.16 and 9.0.4.
pub const MISSING_PAREN: &str = "missing )";

/// Scan Jim expression sugar at a dollar byte under the actual native grammar.
/// The expression retains both parentheses; the returned offset follows the
/// closing parenthesis. Other grammars and other dollar forms return `None`.
/// An unterminated selected sugar reports the native parse error.
pub fn scan_expression_sugar(
    source: &[u8],
    at: usize,
    config: LexerConfig,
) -> Result<Option<(&[u8], usize)>, &'static str> {
    scan_expression_sugar_with_issue(source, at, config).map_err(ParseIssueKind::message)
}

fn scan_expression_sugar_with_issue(
    source: &[u8],
    at: usize,
    config: LexerConfig,
) -> Result<Option<(&[u8], usize)>, ParseIssueKind> {
    if !config.var_syntax.has_expr_sugar()
        || !source.get(at..).is_some_and(|tail| tail.starts_with(b"$("))
    {
        return Ok(None);
    }
    let end = crate::ranges::jim_parenthesis_body_end(source, at + 2);
    if source.get(end) != Some(&b')') {
        return Err(ParseIssueKind::MissingExpressionParenthesis);
    }
    Ok(Some((&source[at + 1..=end], end + 1)))
}

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
    /// Whether an unterminated `[` is literal data rather than C's
    /// `missing close-bracket`.
    ///
    /// False everywhere C Tcl parses source: there an unclosed bracket is a
    /// parse error. True for the same one consumer as
    /// [`Self::bare_var_refs`] — `tcl-vm`'s compiled-word `PUSH` operands —
    /// because the codegen has already decoded that word's source escapes,
    /// so the `\[` a source word wrote to *prevent* substitution arrives
    /// here as a bare `[` with no closer. C never reaches this state: it
    /// parsed and balanced the source long before, so a surviving unclosed
    /// `[` in a compiled word is always data (`expr {$ch eq "\["}`), never
    /// a parse error to re-raise.
    pub unclosed_bracket_is_data: bool,
}

impl Default for SubstFlags {
    /// Everything on — a bare/quoted source word, and plain `subst`.
    fn default() -> Self {
        Self {
            vars: true,
            cmds: true,
            backslashes: true,
            bare_var_refs: true,
            unclosed_bracket_is_data: false,
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
            unclosed_bracket_is_data: true,
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
    /// Jim's `$(expr)` substitution. The borrowed expression includes its
    /// original parentheses; evaluation uses the expression engine directly,
    /// independently of the command named `expr`.
    Expression(&'s [u8]),
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

impl VarRef<'_> {
    /// Recover this borrowed reference's exact extent in its original source.
    /// The native scanner verifies both wrapper forms and closing delimiters;
    /// references made from another source allocation have no source proof.
    #[must_use]
    pub fn source_range_in(
        &self,
        source: &[u8],
        config: LexerConfig,
    ) -> Option<std::ops::Range<usize>> {
        let name_at = (self.name.as_ptr() as usize).checked_sub(source.as_ptr() as usize)?;
        if name_at.checked_add(self.name.len())? > source.len() {
            return None;
        }
        for prefix in [1, 2] {
            let Some(start) = name_at.checked_sub(prefix) else {
                continue;
            };
            if source.get(start) != Some(&b'$') {
                continue;
            }
            if let Ok(Some(reference)) = scan_var_ref(source, start, config)
                && std::ptr::eq(reference.name.as_ptr(), self.name.as_ptr())
                && reference.name.len() == self.name.len()
                && reference.index.is_some() == self.index.is_some()
            {
                return Some(start..reference.next);
            }
        }
        None
    }

    /// The exact native token span of this borrowed reference in its source.
    /// This shares the braced/empty-name convention with source word builders.
    #[must_use]
    pub fn source_span_in(
        &self,
        source: &[u8],
        base: u32,
        config: LexerConfig,
    ) -> Option<crate::Span> {
        let range = self.source_range_in(source, config)?;
        let reference = scan_var_ref(source, range.start, config).ok()??;
        reference.source_span(source, range.start, base)
    }
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

impl RawVarRef<'_> {
    /// Exact name extent in its original source allocation, including literal
    /// braced names. Equal bytes in another allocation do not provide geometry.
    /// This grants no receiver identity, lookup or edit permission.
    #[must_use]
    pub fn name_range_in(self, source: &[u8]) -> Option<std::ops::Range<usize>> {
        original_borrowed_range(self.name, source)
    }

    /// The exact borrowed index extent in its original source allocation.
    /// An empty index retains its position; another equal-valued allocation
    /// cannot supply source geometry for this reference.
    #[must_use]
    pub fn index_range_in(self, source: &[u8]) -> Option<std::ops::Range<usize>> {
        original_borrowed_range(self.index?, source)
    }

    /// Project the native token extent into the source map's span convention.
    /// Bare references retain their exclusive end. A nonempty braced name's
    /// end sits on its closer; an empty braced name includes the closer.
    #[must_use]
    pub fn source_span(self, source: &[u8], at: usize, base: u32) -> Option<crate::Span> {
        variable_source_span(source, at, self.next, base, self.name.is_empty())
    }
}

fn original_borrowed_range(bytes: &[u8], source: &[u8]) -> Option<std::ops::Range<usize>> {
    let start = (bytes.as_ptr() as usize).checked_sub(source.as_ptr() as usize)?;
    let end = start.checked_add(bytes.len())?;
    let original = source.get(start..end)?;
    (std::ptr::eq(original.as_ptr(), bytes.as_ptr()) && original == bytes).then_some(start..end)
}

fn variable_source_span(
    source: &[u8],
    at: usize,
    next: usize,
    base: u32,
    empty_name: bool,
) -> Option<crate::Span> {
    if source.get(at) != Some(&b'$') || next > source.len() {
        return None;
    }
    let end = if source.get(at + 1) == Some(&b'{') && !empty_name {
        next.checked_sub(1)?
    } else {
        next
    };
    mapped_source_span(at, end, base)
}

fn mapped_source_span(start: usize, end: usize, base: u32) -> Option<crate::Span> {
    let start = base.checked_add(u32::try_from(start).ok()?)?;
    let end = base.checked_add(u32::try_from(end).ok()?)?;
    (start <= end).then(|| crate::Span::new(start, end))
}

/// A word's content: a literal (nothing to substitute) or a component list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordBody<'s> {
    /// The bytes are the value — C's `TCL_TOKEN_SIMPLE_WORD`.
    Literal(&'s [u8]),
    /// Components to substitute and concatenate.
    Parts(Vec<WordPart<'s>>),
}

/// A top-level component of [`decompose_spanned`] with its byte extent in the
/// scanned source.
///
/// The extent is what a consumer that keeps *positions* needs — the compiler's
/// `WordExpr` carries a source span per part — and it is not recoverable from
/// the part alone: a [`WordPart::Text`] whose escapes were decoded owns its
/// bytes, and a [`WordPart::ParseError`] borrows nothing. A `Text` extent is
/// the raw, undecoded run; a `Variable` extent covers `$` through the end of
/// the reference (the closing `}` or `)` included); a `Command` extent covers
/// both brackets; a `ParseError` extent runs from the construct C rejected to
/// the end of the source, which is where C stopped parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedPart<'s> {
    /// The component.
    pub part: WordPart<'s>,
    /// Byte offset of the component's first source byte.
    pub start: usize,
    /// Byte offset one past the component's last source byte.
    pub end: usize,
    /// Exact rejected delimiter within the source, independently of the raw component extent.
    pub error_term: Option<usize>,
    /// Structured issue retained by the component parser, not inferred from its text.
    pub error_kind: Option<ParseIssueKind>,
}

/// Cap on `$name(index)` nesting depth [`decompose`] recurses into while
/// parsing an index's own components.
///
/// The index parse is self-recursive with no natural bound — `$a($b($c(…)))`
/// costs one native frame group per `(` — and is reachable from ordinary
/// `subst` / word substitution with no special syntax, so unbounded input
/// aborts the process with an uncatchable stack overflow.
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
/// template. Returns [`WordBody::Literal`] — a borrow of `src`, with no
/// allocation at all — when no enabled substitution actually occurs. That
/// check (`triggers`) is hoisted here, ahead of `scan_parts`'s walk, so
/// this fast path never builds the one-element `Vec` [`decompose_spanned`]'s
/// contract requires; see the module docs' "zero-copy" claim.
#[must_use]
pub fn decompose(src: &[u8], flags: SubstFlags, config: LexerConfig) -> WordBody<'_> {
    if !triggers(src, flags) {
        return WordBody::Literal(src);
    }
    body_of(scan_parts(
        src,
        flags,
        config,
        0,
        TemplateVariableSyntax::WrittenWord,
        &mut false,
    ))
}

/// Variable acceptance of a reached substitution template, separate from words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateVariableSyntax {
    /// Written-word variable grammar, including its required closing delimiters.
    WrittenWord,
    /// C `TclSubstParse` token boundaries, including single-byte literal
    /// substitution sigils when their operation is disabled. Unlike written
    /// words, template text/backslash tokens are not merged before emission.
    CTcl,
    /// Jim 0.84 `JimParseSubst` / `JimParseVar`: `${name` reads through input end.
    /// Braced names stop at the first literal close brace.
    Jim084,
}

/// Native template syntax whose execution protocol is not yet represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedTemplateSyntax {
    /// Jim's `$[...]` expression substitution is independent of `-nocommands`.
    ExpressionSugar,
}

/// Decompose an actual template under its independently selected native parser.
/// Ordinary word parsing never consumes this policy. Array and command parts
/// retain their existing nested grammar and require separate execution proofs.
pub fn decompose_template_spanned(
    src: &[u8],
    flags: SubstFlags,
    config: LexerConfig,
    variables: TemplateVariableSyntax,
) -> Result<Vec<SpannedPart<'_>>, UnsupportedTemplateSyntax> {
    if variables == TemplateVariableSyntax::Jim084 && flags.vars {
        let mut at = 0;
        while at < src.len() {
            if src[at] == b'\\' && at + 1 < src.len() {
                at += 2;
            } else if src[at] == b'$' {
                if matches!(src.get(at + 1), Some(b'[' | b'(')) {
                    return Err(UnsupportedTemplateSyntax::ExpressionSugar);
                }
                at = scan_template_var_ref(src, at, config, variables)
                    .ok()
                    .flatten()
                    .map_or(at + 1, |reference| reference.next);
            } else {
                at += 1;
            }
        }
    }
    if !triggers(src, flags) && variables != TemplateVariableSyntax::CTcl {
        return Ok(vec![SpannedPart {
            part: WordPart::Text(Cow::Borrowed(src)),
            start: 0,
            end: src.len(),
            error_term: None,
            error_kind: None,
        }]);
    }
    Ok(scan_parts(src, flags, config, 0, variables, &mut false))
}

fn scan_template_var_ref(
    src: &[u8],
    at: usize,
    config: LexerConfig,
    variables: TemplateVariableSyntax,
) -> Result<Option<RawVarRef<'_>>, ComponentParseError> {
    if variables == TemplateVariableSyntax::Jim084 && src.get(at + 1) == Some(&b'{') {
        let start = at + 2;
        let end = src[start..]
            .iter()
            .position(|byte| *byte == b'}')
            .map_or(src.len(), |end| start + end);
        return Ok(Some(RawVarRef {
            name: &src[start..end],
            index: None,
            next: if end < src.len() { end + 1 } else { end },
        }));
    }
    scan_var_ref_with_term(src, at, config)
}

/// [`decompose`] with each top-level component's byte extent in `src`.
///
/// A word with nothing to substitute comes back as exactly one borrowed
/// [`WordPart::Text`] spanning all of `src` — the same span [`decompose`]
/// collapses to [`WordBody::Literal`]. Index components of a `$arr(index)`
/// reference are nested inside the [`WordPart::Variable`] without extents.
#[must_use]
pub fn decompose_spanned(
    src: &[u8],
    flags: SubstFlags,
    config: LexerConfig,
) -> Vec<SpannedPart<'_>> {
    decompose_at_depth(src, flags, config, 0, &mut false)
}

/// A bounded decomposition could not retain every executable substitution.
/// This is a host capability boundary, not a Tcl syntax error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecompositionUnavailable {
    /// Nested array indices exceed the shared recursive decomposition budget.
    IndexNesting,
}

/// Decompose an executable word without accepting advisory literal fallback.
/// Original syntax errors remain `WordPart::ParseError`; a nested index whose
/// substitutions cannot all be retained returns a separate capability error.
///
/// # Errors
/// Returns `IndexNesting` when the shared index budget would conceal enabled
/// substitutions. It never turns that input into a literal or guest error.
pub fn decompose_spanned_checked(
    src: &[u8],
    flags: SubstFlags,
    config: LexerConfig,
) -> Result<Vec<SpannedPart<'_>>, DecompositionUnavailable> {
    let mut unavailable = false;
    let parts = decompose_at_depth(src, flags, config, 0, &mut unavailable);
    if unavailable {
        Err(DecompositionUnavailable::IndexNesting)
    } else {
        Ok(parts)
    }
}

impl SpannedPart<'_> {
    /// Map a template component without borrowing written-word closing rules.
    #[must_use]
    pub fn template_source_span(
        &self,
        source: &[u8],
        base: u32,
        variables: TemplateVariableSyntax,
    ) -> Option<crate::Span> {
        if variables == TemplateVariableSyntax::Jim084
            && matches!(&self.part, WordPart::Variable(_))
            && source.get(self.start + 1) == Some(&b'{')
            && source.get(self.end.checked_sub(1)?) != Some(&b'}')
        {
            mapped_source_span(self.start, self.end, base)
        } else {
            self.source_span(source, 0, base)
        }
    }

    /// Map this parsed component to its native source token span.
    /// `region_at` locates the decomposed region inside `source`; `base` locates
    /// that source buffer inside the document. Closing conventions are owned
    /// here, so variable/index consumers and word builders share exact sites.
    #[must_use]
    pub fn source_span(&self, source: &[u8], region_at: usize, base: u32) -> Option<crate::Span> {
        let start = region_at.checked_add(self.start)?;
        let end = region_at.checked_add(self.end)?;
        match &self.part {
            WordPart::Variable(variable) => {
                variable_source_span(source, start, end, base, variable.name.is_empty())
            }
            WordPart::Expression(expression) => mapped_source_span(
                start,
                if expression.len() == 2 {
                    end
                } else {
                    end.checked_sub(1)?
                },
                base,
            ),
            WordPart::Command(script) => mapped_source_span(
                start,
                if script.is_empty() {
                    end
                } else {
                    end.checked_sub(1)?
                },
                base,
            ),
            _ => mapped_source_span(start, end, base),
        }
    }
}

/// C's `TCL_TOKEN_SIMPLE_WORD` collapse. A span whose only component is one
/// *borrowed* text run had nothing to substitute after all — a `$` with no
/// name behind it, a `[` that the flags left inert — so it goes back as a
/// zero-copy `Literal`. An *owned* run (escapes were decoded) cannot: it no
/// longer borrows the source.
fn body_of(parts: Vec<SpannedPart<'_>>) -> WordBody<'_> {
    if let [
        SpannedPart {
            part: WordPart::Text(Cow::Borrowed(b)),
            ..
        },
    ] = parts.as_slice()
    {
        return WordBody::Literal(b);
    }
    WordBody::Parts(parts.into_iter().map(|spanned| spanned.part).collect())
}

/// Where the `"`-delimited word opening at `open_quote` closes, or
/// [`MISSING_QUOTE`].
///
/// The one place the `missing "` spelling is decided. Delegates the search to
/// [`crate::close_quote_offset`], which steps over `\X` pairs and complete `[…]`
/// substitutions so a `"` inside either does not close the word.
pub fn quoted_word_close(
    source: impl AsRef<[u8]>,
    open_quote: usize,
) -> Result<usize, &'static str> {
    quoted_word_close_with_issue(source, open_quote).map_err(ParseIssueKind::message)
}

/// Closing quote of the original word under the supplied complete lexical grammar.
/// Substitution and escape boundaries use the shared configured delimiter owner.
pub fn quoted_word_close_in(
    source: impl AsRef<[u8]>,
    open_quote: usize,
    config: LexerConfig,
) -> Result<usize, &'static str> {
    quoted_word_close_with_issue_in(source, open_quote, config).map_err(ParseIssueKind::message)
}

pub(crate) fn quoted_word_close_with_issue(
    source: impl AsRef<[u8]>,
    open_quote: usize,
) -> Result<usize, ParseIssueKind> {
    quoted_word_close_with_issue_in(source, open_quote, LexerConfig::default())
}

pub(crate) fn quoted_word_close_with_issue_in(
    source: impl AsRef<[u8]>,
    open_quote: usize,
    config: LexerConfig,
) -> Result<usize, ParseIssueKind> {
    crate::ranges::close_quote_offset_in(source, open_quote, config)
        .ok_or(ParseIssueKind::MissingQuote)
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
    scan_var_ref_with_term(src, at, config).map_err(|error| error.kind.message())
}

#[derive(Debug, Clone, Copy)]
struct ComponentParseError {
    kind: ParseIssueKind,
    term: usize,
}

impl ComponentParseError {
    const fn new(kind: ParseIssueKind, term: usize) -> Self {
        Self { kind, term }
    }
    fn rebased(self, base: usize) -> Self {
        Self {
            term: self.term + base,
            ..self
        }
    }
    /// The error component that terminates a walk from `start` to `end`.
    fn spanned_part<'s>(self, start: usize, end: usize) -> SpannedPart<'s> {
        SpannedPart {
            part: WordPart::ParseError(self.kind.message()),
            start,
            end,
            error_term: Some(self.term),
            error_kind: Some(self.kind),
        }
    }
}

fn scan_var_ref_with_term(
    src: &[u8],
    at: usize,
    config: LexerConfig,
) -> Result<Option<RawVarRef<'_>>, ComponentParseError> {
    debug_assert_eq!(src.get(at), Some(&b'$'), "caller must point at a `$`");
    if src.get(at + 1) == Some(&b'{') {
        let name_start = at + 2;
        return match braced_var_name_end(src, name_start, config.braced_var) {
            BracedVarEnd::Closed(end) => Ok(Some(RawVarRef {
                name: &src[name_start..end],
                index: None,
                next: end + 1,
            })),
            BracedVarEnd::Unterminated => Err(ComponentParseError::new(
                ParseIssueKind::MissingVariableBrace,
                at + 1,
            )),
        };
    }
    let start = at + 1;
    let name_end = tcl_core_types::naming::scan_var_name_end_with(
        src,
        start,
        config.var_syntax.name_allows_high_bytes(),
    );
    if name_end == start && (src.get(name_end) != Some(&b'(') || config.var_syntax.has_expr_sugar())
    {
        return Ok(None);
    }
    if src.get(name_end) == Some(&b'(') {
        if config.var_syntax.index_parens_nest() {
            let end = crate::ranges::jim_parenthesis_body_end(src, name_end + 1);
            if src.get(end) != Some(&b')') {
                return Err(ComponentParseError::new(
                    ParseIssueKind::MissingParenthesis,
                    name_end,
                ));
            }
            return Ok(Some(RawVarRef {
                name: &src[start..name_end],
                index: Some(&src[name_end + 1..end]),
                next: end + 1,
            }));
        }
        let scan = scan_array_index_in(src, name_end, config);
        if let Some(term) = scan.invalid {
            return Err(ComponentParseError::new(
                ParseIssueKind::InvalidArrayIndexCharacter,
                term,
            ));
        }
        return match scan.end {
            ArrayIndexEnd::Closed(end) => Ok(Some(RawVarRef {
                name: &src[start..name_end],
                index: Some(&src[name_end + 1..end - 1]),
                next: end,
            })),
            // An index can be unterminated because a construct *inside* it
            // is: C reports that inner error, not the missing `)`.
            // `subst {$a([foo)}` is `missing close-bracket` on 8.6.16/9.0.4.
            ArrayIndexEnd::Unterminated => {
                // An index substitutes commands and variables as the
                // surrounding source word does.
                Err(
                    error_inside_unterminated(&src[name_end + 1..], SubstFlags::default(), config)
                        .map_or_else(
                            || {
                                ComponentParseError::new(
                                    ParseIssueKind::MissingParenthesis,
                                    name_end,
                                )
                            },
                            |error| error.rebased(name_end + 1),
                        ),
                )
            }
        };
    }
    Ok(Some(RawVarRef {
        name: &src[start..name_end],
        index: None,
        next: name_end,
    }))
}

/// Recognise exactly one original variable reference under the actual grammar.
/// Non-reference or compound text returns `Ok(None)`; malformed reference
/// syntax retains the scanner's native parse error. Jim expression sugar is
/// deliberately outside this variable-only projection.
pub fn whole_var_ref(
    source: &[u8],
    config: LexerConfig,
) -> Result<Option<RawVarRef<'_>>, &'static str> {
    if source.first() != Some(&b'$') {
        return Ok(None);
    }
    Ok(scan_var_ref(source, 0, config)?.filter(|reference| reference.next == source.len()))
}

/// Whether `word` is exactly one command substitution, `[` through its own
/// closing `]` with nothing after: `[a b]` is, `[a][b]` and `[a]x` are not
/// (their first bracket closes before the end). The close is
/// [`command_subst_close`]'s, under `config`.
#[must_use]
pub fn is_whole_command_substitution(word: &[u8], config: LexerConfig) -> bool {
    word.first() == Some(&b'[')
        && command_subst_close(word, 0, SubstFlags::default(), config) == Ok(word.len())
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
    command_subst_close_with_term(src, at, flags, config).map_err(|error| error.kind.message())
}

fn command_subst_close_with_term(
    src: &[u8],
    at: usize,
    flags: SubstFlags,
    config: LexerConfig,
) -> Result<usize, ComponentParseError> {
    match crate::ranges::command_substitution_end_bytes_in(src, at, config) {
        Some(end) => Ok(end),
        None => Err(error_inside_unterminated_bracket(src, at, flags, config)),
    }
}

/// Whether any byte of `src` could start a substitution `flags` has enabled —
/// the "nothing to do" test shared by [`decompose`]'s fast path and
/// [`decompose_at_depth`]'s.
fn triggers(src: &[u8], flags: SubstFlags) -> bool {
    src.iter().any(|&c| {
        (flags.vars && c == b'$') || (flags.cmds && c == b'[') || (flags.backslashes && c == b'\\')
    })
}

/// Scan one component list while retaining every index as original raw text.
/// The executable arena queues those indices, rather than recursing here.
pub(crate) fn shallow_spanned_parts(
    src: &[u8],
    flags: SubstFlags,
    config: LexerConfig,
    variables: TemplateVariableSyntax,
) -> Vec<SpannedPart<'_>> {
    if !triggers(src, flags) && variables != TemplateVariableSyntax::CTcl {
        return vec![SpannedPart {
            part: WordPart::Text(Cow::Borrowed(src)),
            start: 0,
            end: src.len(),
            error_term: None,
            error_kind: None,
        }];
    }
    scan_parts(src, flags, config, MAX_INDEX_DEPTH, variables, &mut false)
}

fn decompose_at_depth<'s>(
    src: &'s [u8],
    flags: SubstFlags,
    config: LexerConfig,
    depth: u32,
    unavailable: &mut bool,
) -> Vec<SpannedPart<'s>> {
    if !triggers(src, flags) {
        return vec![SpannedPart {
            part: WordPart::Text(Cow::Borrowed(src)),
            start: 0,
            end: src.len(),
            error_term: None,
            error_kind: None,
        }];
    }
    scan_parts(
        src,
        flags,
        config,
        depth,
        TemplateVariableSyntax::WrittenWord,
        unavailable,
    )
}

/// Package the selected sugar's exact span, including its terminal parse error.
fn expression_component(src: &[u8], at: usize, config: LexerConfig) -> SpannedPart<'_> {
    let (part, end, error_kind) = match scan_expression_sugar_with_issue(src, at, config) {
        Ok(Some((expression, next))) => (WordPart::Expression(expression), next, None),
        Err(kind) => (WordPart::ParseError(kind.message()), src.len(), Some(kind)),
        Ok(None) => unreachable!("selected expression sugar prefix"),
    };
    let error_term = matches!(part, WordPart::ParseError(_)).then_some(at + 1);
    SpannedPart {
        part,
        start: at,
        end,
        error_term,
        error_kind,
    }
}

fn variable_component<'s>(
    at: usize,
    raw: RawVarRef<'s>,
    config: LexerConfig,
    depth: u32,
    unavailable: &mut bool,
) -> SpannedPart<'s> {
    let index = raw.index.map(|idx| {
        // Index evaluation enables every substitution independently of the
        // enclosing mask; the nesting cap retains literal bytes and refusal.
        let flags = SubstFlags::default();
        if depth >= MAX_INDEX_DEPTH {
            *unavailable |= triggers(idx, flags);
            vec![WordPart::Text(Cow::Borrowed(idx))]
        } else {
            match body_of(decompose_at_depth(
                idx,
                flags,
                config,
                depth + 1,
                unavailable,
            )) {
                WordBody::Literal(bytes) => vec![WordPart::Text(Cow::Borrowed(bytes))],
                WordBody::Parts(parts) => parts,
            }
        }
    });
    SpannedPart {
        part: WordPart::Variable(VarRef {
            name: raw.name,
            index,
        }),
        start: at,
        end: raw.next,
        error_term: None,
        error_kind: None,
    }
}

/// The substitution walk itself, once [`triggers`] has confirmed there is
/// something to scan for. Shared by [`decompose`] (triggered case) and
/// [`decompose_at_depth`] (always, including the recursive `$name(index)`
/// call) so the walk exists exactly once.
fn scan_parts<'s>(
    src: &'s [u8],
    flags: SubstFlags,
    config: LexerConfig,
    depth: u32,
    variables: TemplateVariableSyntax,
    unavailable: &mut bool,
) -> Vec<SpannedPart<'s>> {
    let len = src.len();
    let mut parts: Vec<SpannedPart> = Vec::new();
    let mut lit_start = 0usize;
    let mut i = 0usize;

    while i < len {
        let c = src[i];
        if variables == TemplateVariableSyntax::CTcl
            && ((c == b'$' && (!flags.vars || !starts_var_ref(src, i, flags, config)))
                || (c == b'[' && !flags.cmds)
                || (c == b'\\' && (!flags.backslashes || i + 1 == len))
                || c == 0)
        {
            // ParseTokens still emits a separate one-byte TEXT token for a
            // disabled substitution sigil or raw zero. TclSubstCompile keeps
            // that token separate; concatenation can change result storage.
            flush_text(&mut parts, src, lit_start, i, flags, config.escapes);
            parts.push(SpannedPart {
                part: WordPart::Text(Cow::Borrowed(&src[i..=i])),
                start: i,
                end: i + 1,
                error_term: None,
                error_kind: None,
            });
            i += 1;
            lit_start = i;
        } else if variables == TemplateVariableSyntax::CTcl && c == b'\\' {
            flush_text(&mut parts, src, lit_start, i, flags, config.escapes);
            let end = crate::substitution::backslash_escape_end_bytes_in(src, i, config.escapes);
            flush_text(&mut parts, src, i, end, flags, config.escapes);
            i = end;
            lit_start = i;
        } else if flags.vars
            && c == b'$'
            && config.var_syntax.has_expr_sugar()
            && src.get(i + 1) == Some(&b'(')
        {
            flush_text(&mut parts, src, lit_start, i, flags, config.escapes);
            let component = expression_component(src, i, config);
            i = component.end;
            let failed = matches!(component.part, WordPart::ParseError(_));
            parts.push(component);
            if failed {
                return parts;
            }
            lit_start = i;
        } else if flags.vars && c == b'$' && starts_var_ref(src, i, flags, config) {
            flush_text(&mut parts, src, lit_start, i, flags, config.escapes);
            match scan_template_var_ref(src, i, config, variables) {
                Ok(Some(raw)) => {
                    parts.push(variable_component(i, raw, config, depth, unavailable));
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
                Err(error) => {
                    parts.push(error.spanned_part(i, len));
                    return parts;
                }
            }
            lit_start = i;
        } else if flags.cmds && c == b'[' {
            match command_subst_close_with_term(src, i, flags, config) {
                Ok(end) => {
                    flush_text(&mut parts, src, lit_start, i, flags, config.escapes);
                    parts.push(SpannedPart {
                        part: WordPart::Command(&src[i + 1..end - 1]),
                        start: i,
                        end,
                        error_term: None,
                        error_kind: None,
                    });
                    i = end;
                    lit_start = i;
                }
                // A compiled word's unclosed `[` is data the codegen already
                // decoded, not a parse error. Nothing is flushed: the bracket
                // stays inside the literal run it sits in, so a word with
                // nothing else to substitute still collapses to one `Literal`.
                Err(_) if flags.unclosed_bracket_is_data => i += 1,
                Err(error) => {
                    flush_text(&mut parts, src, lit_start, i, flags, config.escapes);
                    parts.push(error.spanned_part(i, len));
                    return parts;
                }
            }
        } else if (flags.backslashes || variables == TemplateVariableSyntax::Jim084)
            && c == b'\\'
            && i + 1 < len
        {
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
    parts
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
) -> ComponentParseError {
    error_inside_unterminated(&src[at + 1..], flags, config).map_or_else(
        || ComponentParseError::new(ParseIssueKind::MissingBracket, at),
        |error| error.rebased(at + 1),
    )
}

/// The error C reports for the first unterminated construct in `src`, or
/// `None` when every construct inside it closes.
///
/// The substituted text of an unterminated `[…]` — and of an unterminated
/// `$a(…)` — is a script C parses before it can complain about the missing
/// closer, so an unterminated construct *inside* is the error it reports:
/// `subst {[list "abc}` is `missing "`, not `missing close-bracket`.
///
/// The walk is **iterative**: an unclosed `[` does not recurse, it records
/// itself as the fallback and keeps scanning the same buffer, since everything
/// after it is inside it. Recursing here would cost one native stack frame
/// per unmatched bracket, so a template of many `[` bytes would abort the
/// process instead of returning a catchable error.
///
/// It is also deliberately shallow — one word level, no command splitting —
/// because full script segmentation belongs to the boundary owner. It
/// covers the constructs whose closers are unambiguous from here and leaves
/// the outer error for everything else.
fn error_inside_unterminated(
    src: &[u8],
    flags: SubstFlags,
    config: LexerConfig,
) -> Option<ComponentParseError> {
    // The innermost unclosed `[` seen so far: the answer if nothing deeper
    // turns out to be unterminated.
    let mut pending: Option<ComponentParseError> = None;
    let mut i = 0usize;
    let mut command_position = true;
    let mut closable = true;
    // Whether the scan is inside a word. A `"` or `{` only opens one at a word
    // boundary, and a `#` only opens a comment at the start of a *word* that
    // is also at command position — an escaped byte is word content, so
    // `[\;# {` is a word `\;#` then a `{` word (C: `missing close-brace`),
    // not a comment.
    let mut in_word = false;
    while i < src.len() {
        let c = src[i];
        match c {
            // An escape is word content: it starts a word if one was not
            // open, and it is never a separator.
            b'\\' => {
                in_word = true;
                command_position = false;
                i += 2;
            }
            // A comment runs to the end of the line, and a `"` or `{` inside
            // it opens nothing.
            b'#' if command_position && !in_word => {
                while i < src.len() && src[i] != b'\n' {
                    i += 1;
                }
            }
            b'$' if flags.vars && starts_var_ref(src, i, flags, config) => {
                match scan_var_ref_with_term(src, i, config) {
                    Ok(Some(raw)) => i = raw.next,
                    Ok(None) => i += 1,
                    Err(msg) => return Some(msg),
                }
                command_position = false;
                in_word = true;
            }
            // `closable` collapses the pathological all-`[` template to a
            // linear walk: with no `]` left in the buffer no bracket from here
            // can close, so there is nothing to search for.
            b'[' if flags.cmds && closable => {
                if let Some(end) = crate::ranges::command_substitution_end_bytes_in(src, i, config)
                {
                    i = end;
                    command_position = false;
                    in_word = true;
                } else {
                    pending = Some(ComponentParseError::new(ParseIssueKind::MissingBracket, i));
                    closable = src[i..].contains(&b']');
                    i += 1;
                    // The unclosed bracket opens a nested script.
                    command_position = true;
                    in_word = false;
                }
            }
            b'[' => {
                pending = Some(ComponentParseError::new(ParseIssueKind::MissingBracket, i));
                i += 1;
                command_position = true;
                in_word = false;
            }
            b'"' if !in_word => match close_quote_offset_bytes(src, i, config) {
                Ok(end) => {
                    i = end + 1;
                    command_position = false;
                    in_word = true;
                }
                Err(msg) => return Some(msg),
            },
            b'{' if !in_word => match brace_word_close(src, i) {
                Some(end) => {
                    i = end + 1;
                    command_position = false;
                    in_word = true;
                }
                None => return Some(ComponentParseError::new(ParseIssueKind::MissingBrace, i)),
            },
            b'\n' | b';' => {
                command_position = true;
                in_word = false;
                i += 1;
            }
            _ => {
                if c.is_ascii_whitespace() {
                    in_word = false;
                } else {
                    command_position = false;
                    in_word = true;
                }
                i += 1;
            }
        }
    }
    pending
}

/// Whether the `$` at `at` opens a variable reference under `flags`.
fn starts_var_ref(src: &[u8], at: usize, flags: SubstFlags, config: LexerConfig) -> bool {
    if src.get(at + 1) == Some(&b'{') {
        return true;
    }
    if !flags.bare_var_refs {
        return false;
    }
    let start = at + 1;
    tcl_core_types::naming::scan_var_name_end_with(
        src,
        start,
        config.var_syntax.name_allows_high_bytes(),
    ) > start
        || (src.get(start) == Some(&b'(') && !config.var_syntax.has_expr_sugar())
}

/// The index of the `"` closing the quoted word opening at `at`.
///
/// Steps over `\X` pairs, variable references and complete `[…]`
/// substitutions, as C parses the word's tokens left to right: an
/// **incomplete** nested substitution is the bracket's error, not the
/// quote's (`subst {[list "[foo}` is `missing close-bracket`), and an
/// unterminated `${` or array index is the variable reference's.
fn close_quote_offset_bytes(
    src: &[u8],
    at: usize,
    config: LexerConfig,
) -> Result<usize, ComponentParseError> {
    let mut i = at + 1;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'$' => match scan_var_ref_with_term(src, i, config)? {
                Some(reference) => i = reference.next,
                None => i += 1,
            },
            b'[' => match crate::ranges::command_substitution_end_bytes_in(src, i, config) {
                Some(end) => i = end,
                None => return Err(ComponentParseError::new(ParseIssueKind::MissingBracket, i)),
            },
            b'"' => return Ok(i),
            _ => i += 1,
        }
    }
    Err(ComponentParseError::new(ParseIssueKind::MissingQuote, at))
}

/// The index of the `}` closing the braced word opening at `at`, counting
/// nesting and stepping over `\X` pairs.
fn brace_word_close(src: &[u8], at: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut i = at + 1;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'{' => {
                depth += 1;
                i += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

/// Push `src[start..end]` as a [`WordPart::Text`], decoding its escapes under
/// the release's grammar when backslash substitution is on (borrowing else).
fn flush_text<'s>(
    parts: &mut Vec<SpannedPart<'s>>,
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
        parts.push(SpannedPart {
            part: WordPart::Text(text),
            start,
            end,
            error_term: None,
            error_kind: None,
        });
    }
}

/// Byte escape value formation delegates the shared native decoder. Opaque
/// literal bytes remain unchanged while their adjacent escapes are decoded.
fn decode_bytes_in(raw: &[u8], escapes: EscapeSyntax) -> Cow<'_, [u8]> {
    backslash_subst_bytes_in(raw, escapes)
}

#[cfg(test)]
mod template_policy_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod variable_source_range_tests;
