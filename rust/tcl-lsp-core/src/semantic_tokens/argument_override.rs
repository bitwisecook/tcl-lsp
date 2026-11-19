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

//! The role a command's specification gives one argument word.

use super::emission::closing_delimiter;
use super::legend::TokenKind;
use tcl_lexer::{Token, TokenType};

/// How a specific argument token should be classified, overriding the
/// default lexer-kind classification.
#[derive(Debug, Clone)]
pub(super) enum ArgOverride {
    /// Classify the whole token as this kind (e.g. an event name).
    Kind(TokenKind),
    /// Sub-tokenise the token as a regex pattern (groups / classes /
    /// quantifiers / …); falls back to a single `regexp` token when the
    /// pattern has no metacharacters.
    RegexPattern,
    /// Sub-tokenise the token as a `format`/`scan` conversion string
    /// (`%[pos$][flags][width][.prec][len]type`); falls back to the
    /// default classification when it has no `%` specifiers.
    SprintfFormat,
    /// Sub-tokenise the token as a `clock format`/`scan` field string
    /// (`%Y` / `%Ey` / …); falls back to the default classification when
    /// it has no `%` specifiers.
    ClockFormat,
    /// Sub-tokenise the token as a `binary format`/`scan` field string
    /// (`a3` / `Su` / `c*` / …); falls back to the default classification
    /// when no specifier is recognised.
    BinaryFormat,
    /// Embedded-language token roles mapped back to original source bytes.
    TemplateParts(Vec<(std::ops::Range<usize>, TokenKind)>),
    /// Recurse into a braced command-body argument (`ArgRole::Body`),
    /// re-segmenting its inner script so nested commands / vars / strings
    /// are tokenised rather than emitted as one opaque `string`.
    BodyScript,
    /// Recurse into a braced expression argument (`ArgRole::Expr`),
    /// tokenising it via the expression sub-lexer (variables / numbers /
    /// operators / functions / nested `[cmd]` substitutions).
    ExprScript,
    /// A recognised `-option` switch → `Decorator`.
    Decorator,
    /// A variable name a command declares / writes (`ArgRole::VarWrite`) →
    /// `Variable` + `declaration` modifier.
    VarDecl,
    /// A variable name a command reads by reference (`ArgRole::VarRead` —
    /// `info exists arr(key)`, `array names arr`, `dict with $d`) → `Variable`
    /// with **no** `declaration` modifier: it references an existing variable
    /// rather than declaring one.
    VarRef,
    /// A command name passed as an argument (`ArgRole::CommandPrefix`, or a proc
    /// parameter the analyser inferred to be a `Command`) → `Function`: the
    /// literal names a command the callee invokes.
    CommandRef,
    /// The `{params body ?ns?}` lambda literal argument of a command
    /// carrying `ArgRole::LambdaLiteral` (`apply` today) — its second list
    /// element (the body) is re-segmented as a script.  Reached either as
    /// the call's own argument or, indirectly, through the `[list apply
    /// {…} $x]` deferred-command idiom — see
    /// the shared original produced-prefix owner.
    LambdaLiteral,
    /// A known subcommand word (arg index 1) → `Keyword` + `defaultLibrary`.
    SubcommandKeyword,
    /// The name argument of a `proc` definition → `Function` + `definition`.
    ProcNameDef,
    /// The braced clause-list argument of a `switch … { pat body … }` or an
    /// Expect `expect { ?-flags? pat body … }`: pattern elements are classified
    /// (as regexes when the shape says so) and body elements recursed as
    /// scripts.  Without this the whole `{ pat body … }` list would be walked as
    /// one literal word, leaving every body opaque and unhighlighted.  The
    /// [`CaseListSpec`] is registry data, so the walker names no command; the
    /// `bool` is whether *this call* put the list in regex mode
    /// (`switch -regexp`).
    CaseList(Box<tcl_registry::CaseListSpec>, bool),
    /// A structural keyword word at an argument position (`if`'s
    /// `then`/`elseif`/`else`, `try`'s `on`/`trap`/`finally`), carried
    /// by `ArgRole::Keyword` → highlighted as `Keyword` rather than a
    /// string.
    KeywordArg,
    /// The variable-spec word of a `foreach` / `lmap` / `dict for` loop — a
    /// single bareword (`foreach item …`) or a braced list of names
    /// (`foreach {k v} …`).  Each name is a variable the loop assigns on every
    /// iteration, so it is emitted as `Variable` + `declaration`.
    LoopVarList,
    /// A procedure parameter list (`proc p {a b {c 5} args} …`).  Each
    /// parameter name is emitted as `Parameter` + `declaration`; a `{name
    /// default}` pair emits the name as a parameter and classifies its default.
    ParamList,
    /// The declared name of a definition-body member (`method foo …`,
    /// `typemethod`, `property`), carried by `ArgRole::Name` → emitted as
    /// `Method` + `definition` rather than falling through to a plain string.
    MemberName,
    /// The class name at a *declaring* definer (`oo::class create Shape`,
    /// `snit::type Name`, `itcl::class Name`) → `Class` + `definition`.
    ClassNameDef,
    /// The class name at a *referencing* definer (`oo::define Shape`) → `Class`.
    ClassNameRef,
}

/// The inner content (delimiters stripped via `content_offset`) of a
/// braced/quoted literal token, plus its absolute byte start, or `None`
/// for a non-literal token / out-of-bounds span.  Shared by the
/// sub-language scanners.
///
/// Applies the same clamp-trim as [`push_token`]: the lexer extends a quoted
/// `Esc` fragment's span by one byte over the `$` / `[` that introduces the
/// *following* substitution (keeping `token_text` empty), so a fragment like
/// the `"$` of `"$x"` reports content `$`.  That introducer byte belongs to
/// the next `Var` / `Cmd` token; leaving it in would make a sub-language
/// scanner (regex, …) mis-read it (a `$` as an anchor) and overlap the
/// substitution token.  Trim it back to the leading delimiter here so every
/// consumer sees substitution-free literal content.
pub(super) fn subspec_content(source: &str, tok: Token) -> Option<(usize, &str)> {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return None;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let mut cend = (tok.span.end() as usize).min(source.len());
    if tok.kind == TokenType::Esc
        && (tok.span.end() - tok.span.start()) == u32::from(tok.content_offset) + 1
        && source
            .as_bytes()
            .get(tok.span.end() as usize - 1)
            .is_some_and(|&b| b == b'$' || b == b'[')
    {
        cend = cstart.min(cend);
    }
    // An **empty** delimited word (`{}`, `""`) is the one shape whose
    // `span.end()` lands *past* its closing delimiter rather than at it, so the
    // closer would otherwise be handed back as the word's content.  For a body
    // argument that content is then re-segmented as a script, and the stray `}`
    // is classified as a command head — `proc p {args} {}` would emit
    // `'}':function`.  Recognised exactly (span length == content_offset + 1, last
    // byte is the matching closer), so a non-empty word ending in an *escaped*
    // quote (`"a\""`) is untouched.
    if tok.content_offset > 0
        && (tok.span.end() - tok.span.start()) == u32::from(tok.content_offset) + 1
        && closing_delimiter(source, tok.span.start())
            .is_some_and(|c| source.as_bytes().get(cend - 1) == Some(&c))
    {
        cend = cstart.min(cend);
    }
    source.get(cstart..cend).map(|inner| (cstart, inner))
}

/// Whether an option value's role is re-coloured by another semantic-token
/// pass (`insert_role_overrides` for `Body`/`Expr`, `insert_var_decl_overrides`
/// for `VarWrite`, the retained format projection for a conversion string). Such
/// values must not be claimed as `OptionValue` by the option pass, which would
/// block the more specific role token.
pub(super) fn role_claimed_by_token_pass(role: Option<tcl_registry::ArgRole>) -> bool {
    use tcl_registry::ArgRole;
    matches!(
        role,
        Some(
            ArgRole::Body
                | ArgRole::Expr
                | ArgRole::VarWrite
                | ArgRole::FormatString
                | ArgRole::ScanFormat
        )
    )
}

/// Whether a command's *head word* is a runtime-computed (non-static) command
/// name rather than a statically-resolvable one: a `$var` / `[cmd]`
/// substitution head, or a multi-fragment word (`chartV$node`, `${prefix}cmd`).
///
/// The command name of such a call is only known at runtime — an object handle
/// dispatched through a variable (`$chart method …`), a `[Class new]` /
/// `[dict get …]` constructor-or-lookup result, a computed command
/// name — so it must not be classified as a resolved command-head token, nor
/// consulted against the registry's declared option tables.  A plain
/// single-token bareword head (`puts`, a user proc) is *not* computed and stays
/// on the registry-precise path.
pub(super) fn head_is_computed(seg: &tcl_compiler::segmenter::SegmentedCommand) -> bool {
    seg.argv
        .first()
        .is_some_and(|t| matches!(t.kind, TokenType::Var | TokenType::Cmd))
        || !seg.single_token_word.first().copied().unwrap_or(true)
}
