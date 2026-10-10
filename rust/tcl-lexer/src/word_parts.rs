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

use std::borrow::Cow;

use tcl_dialect::EscapeSyntax;

use crate::lexer::LexerConfig;
use crate::ranges::{
    ArrayIndexEnd, BracedVarEnd, INVALID_CHARACTER_IN_ARRAY_INDEX, MISSING_CLOSE_BRACE_FOR_VAR,
    braced_var_name_end, close_quote_offset, command_substitution_end_bytes, scan_array_index,
};
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
    if !config.var_syntax.has_expr_sugar()
        || !source.get(at..).is_some_and(|tail| tail.starts_with(b"$("))
    {
        return Ok(None);
    }
    let end = crate::ranges::jim_parenthesis_body_end(source, at + 2);
    if source.get(end) != Some(&b')') {
        return Err("missing close-paren for expression substitution");
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
    /// The exact borrowed index extent in its original source allocation.
    /// An empty index retains its position; another equal-valued allocation
    /// cannot supply source geometry for this reference.
    #[must_use]
    pub fn index_range_in(self, source: &[u8]) -> Option<std::ops::Range<usize>> {
        let index = self.index?;
        let start = (index.as_ptr() as usize).checked_sub(source.as_ptr() as usize)?;
        let end = start.checked_add(index.len())?;
        let original = source.get(start..end)?;
        (std::ptr::eq(original.as_ptr(), index.as_ptr()) && original == index).then_some(start..end)
    }

    /// Project the native token extent into the source map's span convention.
    /// Bare references retain their exclusive end. A nonempty braced name's
    /// end sits on its closer; an empty braced name includes the closer.
    #[must_use]
    pub fn source_span(self, source: &[u8], at: usize, base: u32) -> Option<crate::Span> {
        variable_source_span(source, at, self.next, base, self.name.is_empty())
    }
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
/// [`close_quote_offset`], which steps over `\X` pairs and complete `[…]`
/// substitutions so a `"` inside either does not close the word.
pub fn quoted_word_close(
    source: impl AsRef<[u8]>,
    open_quote: usize,
) -> Result<usize, &'static str> {
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
    scan_var_ref_with_term(src, at, config).map_err(|error| error.message)
}

#[derive(Debug, Clone, Copy)]
struct ComponentParseError {
    message: &'static str,
    term: usize,
}

impl ComponentParseError {
    const fn new(message: &'static str, term: usize) -> Self {
        Self { message, term }
    }
    fn rebased(self, base: usize) -> Self {
        Self {
            term: self.term + base,
            ..self
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
                MISSING_CLOSE_BRACE_FOR_VAR,
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
                return Err(ComponentParseError::new(MISSING_PAREN, name_end));
            }
            return Ok(Some(RawVarRef {
                name: &src[start..name_end],
                index: Some(&src[name_end + 1..end]),
                next: end + 1,
            }));
        }
        let scan = scan_array_index(src, name_end, config.array_index, config.braced_var);
        if let Some(term) = scan.invalid {
            return Err(ComponentParseError::new(
                INVALID_CHARACTER_IN_ARRAY_INDEX,
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
                            || ComponentParseError::new(MISSING_PAREN, name_end),
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
    command_subst_close_with_term(src, at, flags, config).map_err(|error| error.message)
}

fn command_subst_close_with_term(
    src: &[u8],
    at: usize,
    flags: SubstFlags,
    config: LexerConfig,
) -> Result<usize, ComponentParseError> {
    match command_substitution_end_bytes(src, at) {
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
    let (part, end) = match scan_expression_sugar(src, at, config) {
        Ok(Some((expression, next))) => (WordPart::Expression(expression), next),
        Err(message) => (WordPart::ParseError(message), src.len()),
        Ok(None) => unreachable!("selected expression sugar prefix"),
    };
    let error_term = matches!(part, WordPart::ParseError(_)).then_some(at + 1);
    SpannedPart {
        part,
        start: at,
        end,
        error_term,
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
                    parts.push(SpannedPart {
                        part: WordPart::ParseError(error.message),
                        start: i,
                        end: len,
                        error_term: Some(error.term),
                    });
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
                    parts.push(SpannedPart {
                        part: WordPart::ParseError(error.message),
                        start: i,
                        end: len,
                        error_term: Some(error.term),
                    });
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
        || ComponentParseError::new(MISSING_CLOSE_BRACKET, at),
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
                if let Some(end) = command_substitution_end_bytes(src, i) {
                    i = end;
                    command_position = false;
                    in_word = true;
                } else {
                    pending = Some(ComponentParseError::new(MISSING_CLOSE_BRACKET, i));
                    closable = src[i..].contains(&b']');
                    i += 1;
                    // The unclosed bracket opens a nested script.
                    command_position = true;
                    in_word = false;
                }
            }
            b'[' => {
                pending = Some(ComponentParseError::new(MISSING_CLOSE_BRACKET, i));
                i += 1;
                command_position = true;
                in_word = false;
            }
            b'"' if !in_word => match close_quote_offset_bytes(src, i) {
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
                None => return Some(ComponentParseError::new(MISSING_CLOSE_BRACE, i)),
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
/// Steps over `\X` pairs and complete `[…]` substitutions. An **incomplete**
/// nested substitution is the bracket's error, not the quote's: C reports
/// `missing close-bracket` for `subst {[list "[foo}`.
fn close_quote_offset_bytes(src: &[u8], at: usize) -> Result<usize, ComponentParseError> {
    let mut i = at + 1;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'[' => match command_substitution_end_bytes(src, i) {
                Some(end) => i = end,
                None => return Err(ComponentParseError::new(MISSING_CLOSE_BRACKET, i)),
            },
            b'"' => return Ok(i),
            _ => i += 1,
        }
    }
    Err(ComponentParseError::new(MISSING_QUOTE, at))
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
        });
    }
}

/// Byte escape value formation delegates the shared native decoder. Opaque
/// literal bytes remain unchanged while their adjacent escapes are decoded.
fn decode_bytes_in(raw: &[u8], escapes: EscapeSyntax) -> Cow<'_, [u8]> {
    backslash_subst_bytes_in(raw, escapes)
}

#[cfg(test)]
mod template_policy_tests {
    use super::*;

    #[test]
    fn c_template_retains_disabled_sigil_and_escape_token_boundaries() {
        // Source proof: naming.substitution.template-token-and-index-source
        // docs/design/analysis/name-resolution-proofs/substitution-template-token-and-index-source.md
        // Native proof: naming.substitution.counted-template-completions
        // docs/design/analysis/name-resolution-proofs/substitution-counted-template-completions.md
        // v6 source0/flags5 has four native pushes before strcat, so C9
        // returns a String rather than one unchanged literal with no primary.
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let config = LexerConfig::for_dialect(dialect);
            let flags = SubstFlags {
                vars: false,
                ..SubstFlags::default()
            };
            let source = b"${x}${y}";
            let parts =
                decompose_template_spanned(source, flags, config, TemplateVariableSyntax::CTcl)
                    .unwrap();
            assert_eq!(
                parts
                    .iter()
                    .map(|part| (part.start, part.end))
                    .collect::<Vec<_>>(),
                [(0, 1), (1, 4), (4, 5), (5, 8)]
            );
            let values: Vec<_> = parts
                .iter()
                .map(|part| match &part.part {
                    WordPart::Text(value) => value.as_ref(),
                    _ => panic!("disabled variable is a native text token"),
                })
                .collect();
            assert_eq!(values, [b"$".as_slice(), b"{x}", b"$", b"{y}"]);
            assert_eq!(
                decompose_template_spanned(
                    source,
                    flags,
                    config,
                    TemplateVariableSyntax::WrittenWord
                )
                .unwrap()
                .len(),
                1
            );
            for (source, expected) in [
                (br"a\nb".as_slice(), [b"a".as_slice(), b"\n", b"b"]),
                (b"a\0b".as_slice(), [b"a".as_slice(), b"\0", b"b"]),
            ] {
                let parts = decompose_template_spanned(
                    source,
                    SubstFlags::default(),
                    config,
                    TemplateVariableSyntax::CTcl,
                )
                .unwrap();
                let values: Vec<_> = parts
                    .iter()
                    .map(|part| match &part.part {
                        WordPart::Text(value) => value.as_ref(),
                        _ => panic!("retained literal/escape token"),
                    })
                    .collect();
                assert_eq!(values, expected);
            }
            let disabled = SubstFlags {
                vars: false,
                cmds: false,
                backslashes: false,
                ..SubstFlags::default()
            };
            let parts =
                decompose_template_spanned(b"$[\\", disabled, config, TemplateVariableSyntax::CTcl)
                    .unwrap();
            assert_eq!(parts.len(), 3);
            assert!(
                parts
                    .iter()
                    .all(|part| matches!(part.part, WordPart::Text(_)))
            );
        }
    }

    #[test]
    fn jim_expression_components_retain_parentheses_and_native_token_sites() {
        // Native proof: naming.variable.empty-array-root-lexical-reference
        // docs/design/analysis/name-resolution-proofs/empty-array-root-lexical-reference.md
        let source = b"pre$(1+(2))post";
        let jim = LexerConfig::for_dialect("jim");
        let parts = decompose_spanned(source, SubstFlags::default(), jim);
        assert_eq!(parts[1].part, WordPart::Expression(b"(1+(2))"));
        assert_eq!((parts[1].start, parts[1].end), (3, 11));
        assert_eq!(
            parts[1].source_span(source, 0, 100),
            Some(crate::Span::new(103, 110))
        );
        assert_eq!(
            scan_expression_sugar(b"$()", 0, jim),
            Ok(Some((b"()".as_slice(), 3)))
        );
        assert_eq!(
            scan_expression_sugar(b"$(1", 0, jim),
            Err("missing close-paren for expression substitution")
        );
        let disabled = SubstFlags {
            vars: false,
            ..SubstFlags::default()
        };
        assert_eq!(decompose(source, disabled, jim), WordBody::Literal(source));
        assert!(
            matches!(decompose(b"$(k)", SubstFlags::default(), LexerConfig::default()), WordBody::Parts(parts) if matches!(&parts[0], WordPart::Variable(reference) if reference.name.is_empty()))
        );
    }

    #[test]
    fn jim_template_acceptance_does_not_relax_written_words() {
        let source = b"prefix ${x y";
        let config = LexerConfig::default();
        let parts = decompose_template_spanned(
            source,
            SubstFlags::default(),
            config,
            TemplateVariableSyntax::Jim084,
        )
        .unwrap();
        let WordPart::Variable(reference) = &parts[1].part else {
            panic!("native scalar reference");
        };
        assert_eq!(reference.name, b"x y");
        assert_eq!(
            parts[1].template_source_span(source, 10, TemplateVariableSyntax::Jim084),
            Some(crate::Span::new(17, 22))
        );
        assert!(matches!(
            decompose_spanned(source, SubstFlags::default(), config)
                .last()
                .unwrap()
                .part,
            WordPart::ParseError(_)
        ));
        let parts = decompose_template_spanned(
            b"${a{b}c}",
            SubstFlags::default(),
            config,
            TemplateVariableSyntax::Jim084,
        )
        .unwrap();
        assert!(
            matches!(&parts[0].part, WordPart::Variable(reference) if reference.name == b"a{b")
        );
        assert!(matches!(&parts[1].part, WordPart::Text(bytes) if bytes.as_ref() == b"c}"));
        let flags = SubstFlags {
            vars: false,
            ..SubstFlags::default()
        };
        assert!(
            matches!(&decompose_template_spanned(source, flags, config, TemplateVariableSyntax::Jim084).unwrap()[0].part, WordPart::Text(bytes) if bytes.as_ref() == source)
        );
        let flags = SubstFlags {
            backslashes: false,
            cmds: false,
            ..SubstFlags::default()
        };
        assert!(
            matches!(&decompose_template_spanned(b"\\${x", flags, config, TemplateVariableSyntax::Jim084).unwrap()[0].part, WordPart::Text(bytes) if bytes.as_ref() == b"\\${x")
        );
        assert_eq!(
            decompose_template_spanned(b"$[1+2]", flags, config, TemplateVariableSyntax::Jim084),
            Err(UnsupportedTemplateSyntax::ExpressionSugar)
        );
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn outer_substitution_mask_does_not_disable_array_index_commands() {
        // Source proof: naming.substitution.template-token-and-index-source
        // docs/design/analysis/name-resolution-proofs/substitution-template-token-and-index-source.md
        // Native proof: naming.substitution.counted-template-completions
        // docs/design/analysis/name-resolution-proofs/substitution-counted-template-completions.md
        // The C v6 array-index control runs its command with outer commands
        // disabled. JimExpandDictSugar separately selects JIM_NONE.
        for dialect in ["tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config = LexerConfig::for_dialect(dialect);
            let flags = SubstFlags {
                cmds: false,
                backslashes: false,
                ..SubstFlags::default()
            };
            let WordBody::Parts(parts) = decompose(br"$a($k\x21[side])", flags, config) else {
                panic!("one array reference");
            };
            let [
                WordPart::Variable(VarRef {
                    index: Some(index), ..
                }),
            ] = parts.as_slice()
            else {
                panic!("retained original index");
            };
            assert!(matches!(index[0], WordPart::Variable(_)));
            assert!(matches!(&index[1], WordPart::Text(text) if text.as_ref() == b"!"));
            assert_eq!(index[2], WordPart::Command(b"side"));
            assert_eq!(
                decompose(
                    br"$a($k\x21[side])",
                    SubstFlags {
                        vars: false,
                        ..flags
                    },
                    config,
                ),
                WordBody::Literal(br"$a($k\x21[side])"),
            );
        }
    }

    /// An unterminated construct reports the *innermost* cause, as C does:
    /// `subst {[list "abc}` is `missing "` and `subst {$a([foo)}` is
    /// `missing close-bracket` on tclsh 8.6.16 and 9.0.4 — not the outer
    /// `missing close-bracket` / `missing )`.
    #[test]
    fn an_unterminated_construct_reports_its_innermost_cause() {
        let config = LexerConfig::default();
        let flags = SubstFlags::default();
        let err = |src: &[u8]| match decompose(src, flags, config) {
            WordBody::Parts(parts) => parts.iter().find_map(|p| match p {
                WordPart::ParseError(m) => Some(*m),
                _ => None,
            }),
            WordBody::Literal(_) => None,
        };
        assert_eq!(err(br#"[list "abc"#), Some(MISSING_QUOTE));
        assert_eq!(err(b"[list {abc"), Some(MISSING_CLOSE_BRACE));
        assert_eq!(err(b"$a([foo)"), Some(MISSING_CLOSE_BRACKET));
        // A bracket whose words all close still reports the missing bracket,
        // and a closed one is no error at all.
        assert_eq!(err(br#"[list "abc" def"#), Some(MISSING_CLOSE_BRACKET));
        assert_eq!(err(br#"[list "abc"]"#), None);
        assert_eq!(err(b"$a(plain"), Some(MISSING_PAREN));
        // An incomplete substitution *inside* a quoted word is the bracket's
        // error, not the quote's, and a `"` inside a comment opens nothing.
        // tclsh 9.0.4 answers `missing close-bracket` for both.
        assert_eq!(err(br#"[list "[foo"#), Some(MISSING_CLOSE_BRACKET));
        assert_eq!(
            err(br#"[list
# a " comment
"#),
            Some(MISSING_CLOSE_BRACKET)
        );
        // An escaped byte is word content, so the `#` after `\;` is data (the
        // `{` then opens a word) and the `"` after the escaped blank is inside
        // the `foo` word, not a word opener. tclsh 9.0.4 answers
        // `missing close-brace` and `missing close-bracket`.
        assert_eq!(err(br"[\;# {"), Some(MISSING_CLOSE_BRACE));
        assert_eq!(err(br#"[foo\ "abc"#), Some(MISSING_CLOSE_BRACKET));
    }

    /// The walk over an unterminated bracket is iterative: a template of many
    /// unmatched `[` must return C's catchable error, not exhaust the native
    /// stack. Recursing per bracket aborted the process instead.
    #[test]
    fn many_unmatched_brackets_do_not_exhaust_the_stack() {
        let src = vec![b'['; 100_000];
        assert_eq!(
            decompose(&src, SubstFlags::default(), LexerConfig::default()),
            WordBody::Parts(vec![WordPart::ParseError(MISSING_CLOSE_BRACKET)])
        );
    }

    /// A compiled word's unclosed `[` is data, not `missing close-bracket`:
    /// the codegen decoded the source's `\[` to a bare `[` before the VM saw
    /// it, so re-raising C's parse error breaks `expr {$ch eq "\["}`
    /// (tclsh 8.6.16 / 9.0.4 both answer with the comparison, not an error).
    /// A source word keeps the error.
    #[test]
    fn an_unclosed_bracket_is_data_only_for_a_compiled_word() {
        let config = LexerConfig::default();
        let compiled = SubstFlags::compiled_word();
        // Nothing substituted after all, so both collapse to a borrowed
        // `Literal` by the `TCL_TOKEN_SIMPLE_WORD` rule.
        assert_eq!(decompose(b"[", compiled, config), WordBody::Literal(b"["));
        assert_eq!(
            decompose(b"a[b", compiled, config),
            WordBody::Literal(b"a[b")
        );
        // A closed bracket still substitutes, and text after an unclosed one
        // is not emitted twice.
        assert_eq!(
            decompose(b"x[y]z[w", compiled, config),
            WordBody::Parts(vec![
                WordPart::Text(Cow::Borrowed(b"x")),
                WordPart::Command(b"y"),
                WordPart::Text(Cow::Borrowed(b"z[w")),
            ])
        );
        // A source word keeps C's parse error.
        assert_eq!(
            decompose(b"a[b", SubstFlags::default(), config),
            WordBody::Parts(vec![
                WordPart::Text(Cow::Borrowed(b"a")),
                WordPart::ParseError(MISSING_CLOSE_BRACKET),
            ])
        );
    }
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

    /// `Tcl_ParseVarName`'s `${…}` close rule differs
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

    /// Tcl 9 rejects raw `{`, `"`, `(`, `}` written in an
    /// array index; Tcl 8 passes them through. The mask applies to *source*
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

    /// A literal run decodes under the emulated release's escape grammar:
    /// TIP 388 capped `\x` at two hex digits from 8.6 and added
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
        // …and the escapes of that literal run are still decoded:
        // `string length "x\$y"` is 3, not 4.
        assert_eq!(
            decompose(b"x\\$y", SubstFlags::compiled_word(), nine()),
            WordBody::Parts(vec![text(b"x$y")])
        );
    }

    /// The extents [`decompose_spanned`] reports are the raw source runs:
    /// a decoded text run keeps its undecoded width, a reference covers its
    /// closer, and a parse error runs to the end of the source (where C
    /// stopped). Nested index components carry no extents of their own.
    #[test]
    fn spanned_parts_carry_their_raw_extents() {
        let src = b"a\\tb${x}[c]$arr($i)\\$z";
        let spanned = decompose_spanned(src, SubstFlags::default(), nine());
        let extents: Vec<(usize, usize)> = spanned.iter().map(|p| (p.start, p.end)).collect();
        assert_eq!(extents, vec![(0, 4), (4, 8), (8, 11), (11, 19), (19, 22)]);
        assert_eq!(&src[4..8], b"${x}");
        assert_eq!(&src[8..11], b"[c]");
        assert_eq!(&src[11..19], b"$arr($i)");
        assert_eq!(spanned[0].part, text(b"a\tb"));
        assert_eq!(spanned[4].part, text(b"$z"));
        // Every extent tiles the source without gaps.
        assert!(spanned.windows(2).all(|w| w[0].end == w[1].start));

        let stopped = decompose_spanned(b"x[side][b", SubstFlags::default(), nine());
        assert_eq!(
            stopped.last().map(|p| (p.part.clone(), p.start, p.end)),
            Some((WordPart::ParseError(MISSING_CLOSE_BRACKET), 7, 9))
        );
        // The literal fast path is one borrowed run over the whole source.
        assert_eq!(
            decompose_spanned(b"plain", SubstFlags::default(), nine()),
            vec![SpannedPart {
                part: text(b"plain"),
                start: 0,
                end: 5,
                error_term: None,
            }]
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
    fn whole_variable_references_follow_native_empty_name_and_jim_grammar() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let config = LexerConfig::for_dialect(dialect);
            let empty_array = whole_var_ref(b"$(k)", config).unwrap().unwrap();
            assert_eq!(empty_array.name, b"");
            assert_eq!(empty_array.index, Some(b"k".as_slice()));
            assert_eq!(
                parts(b"$(k)", config),
                vec![WordPart::Variable(VarRef {
                    name: b"",
                    index: Some(vec![text(b"k")]),
                })]
            );
            assert_eq!(
                decompose("$é".as_bytes(), SubstFlags::default(), config),
                WordBody::Literal("$é".as_bytes())
            );
            assert_eq!(whole_var_ref(b"${}", config).unwrap().unwrap().name, b"");
            assert!(whole_var_ref("$café".as_bytes(), config).unwrap().is_none());
            if matches!(dialect, "tcl9.0" | "tcl9.1") {
                assert_eq!(
                    whole_var_ref(b"$a(k(x))", config),
                    Err(INVALID_CHARACTER_IN_ARRAY_INDEX)
                );
            } else {
                assert!(whole_var_ref(b"$a(k(x))", config).unwrap().is_none());
            }
            assert!(whole_var_ref(b"$a(k)suffix", config).unwrap().is_none());
        }
        let jim = LexerConfig::for_dialect("jim");
        assert!(jim.var_syntax.has_expr_sugar());
        assert_eq!(
            parts("$é".as_bytes(), jim),
            vec![WordPart::Variable(VarRef {
                name: "é".as_bytes(),
                index: None,
            })]
        );
        assert!(whole_var_ref(b"$(1+2)", jim).unwrap().is_none());
        assert_eq!(
            whole_var_ref("$café".as_bytes(), jim)
                .unwrap()
                .unwrap()
                .name,
            "café".as_bytes()
        );
        assert_eq!(
            whole_var_ref(b"$a(k(x))", jim).unwrap().unwrap().index,
            Some(b"k(x)".as_slice())
        );
        assert_eq!(whole_var_ref(b"${(k)}", jim).unwrap().unwrap().name, b"(k)");
        assert!(whole_var_ref(b"$a(k", jim).is_err());
    }

    #[test]
    fn checked_decomposition_keeps_index_capacity_distinct_from_parse_errors() {
        let mut source = Vec::new();
        for _ in 0..80 {
            source.extend_from_slice(b"$a(");
        }
        source.extend_from_slice(b"[compileMe]");
        source.extend(std::iter::repeat_n(b')', 80));
        assert_eq!(
            decompose_spanned_checked(&source, SubstFlags::default(), nine()),
            Err(DecompositionUnavailable::IndexNesting),
        );
        // The advisory API retains its documented bounded projection.
        assert_ne!(
            decompose_spanned(&source, SubstFlags::default(), nine()),
            [] as [SpannedPart<'_>; 0]
        );
        let malformed = decompose_spanned_checked(b"$a(", SubstFlags::default(), nine()).unwrap();
        assert!(
            malformed
                .iter()
                .any(|part| matches!(part.part, WordPart::ParseError(_)))
        );
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

    #[test]
    fn raw_index_geometry_keeps_empty_and_foreign_source_distinct() {
        // Implementation contract: naming.grammar.checked-expression-substitution-context
        // docs/design/analysis/name-resolution-proofs/checked-expression-substitution-context.md

        let source = b"$a()".to_vec();
        let reference = scan_var_ref(&source, 0, nine()).unwrap().unwrap();
        assert_eq!(reference.index_range_in(&source), Some(3..3));
        let equal_source = source.clone();
        assert_eq!(reference.index_range_in(&equal_source), None);

        let source = "$café($clé)".as_bytes();
        let mut config = nine();
        config.var_syntax = tcl_dialect::VarSyntax::Jim;
        let reference = scan_var_ref(source, 0, config).unwrap().unwrap();
        let range = reference.index_range_in(source).unwrap();
        assert_eq!(&source[range], "$clé".as_bytes());
    }

    /// The index parse recurses once per
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

#[cfg(test)]
mod variable_source_range_tests {
    use super::*;

    #[test]
    fn borrowed_reference_extents_use_the_native_scanner() {
        let config = LexerConfig::default();
        for source in [b"$name".as_slice(), b"${braced}", b"$a($i)", b"${}"] {
            let WordBody::Parts(parts) = decompose(source, SubstFlags::default(), config) else {
                panic!("source contains a variable reference");
            };
            let WordPart::Variable(variable) = &parts[0] else {
                panic!("first component is a reference");
            };
            assert_eq!(
                variable.source_range_in(source, config),
                Some(0..source.len())
            );
            let different_allocation = source.to_vec();
            assert_eq!(
                variable.source_range_in(&different_allocation, config),
                None
            );
            if let Some(index) = &variable.index {
                let WordPart::Variable(nested) = &index[0] else {
                    panic!("nested index reference");
                };
                assert_eq!(nested.source_range_in(source, config), Some(3..5));
            }
        }
    }
}
