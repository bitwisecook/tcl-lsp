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

//! Readonly iRulesLX method source candidates and JavaScript registrations.
//! The Tcl side consumes complete current original source vectors and guarded
//! Registry metadata. Literal method words retain source geometry; source
//! constructors supply separate possible extension labels. Neither nearby set
//! words nor control-body syntax proves an evaluated handle or current cell.

use tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords;
use tcl_lexer::{ExecutablePart, Span};
use tcl_registry::CommandRegistry;
use tcl_registry::remote_method::{MethodWord, RemoteDispatch, RemoteFamily};

/// The separately retained literal plugin and extension source labels.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IlxExtension {
    /// Plugin source value; no live workspace association follows.
    pub plugin: String,
    /// Extension source value; independent of an evaluated RPC handle.
    pub extension: String,
}

/// An independently unresolved premise of a readonly ILX source candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IlxSourceObligation {
    /// Conditional source metadata does not establish the entered handler.
    HandlerApplicability,
    /// A written constructor does not establish its successful evaluated result.
    HandleResultUnavailable,
    /// Source writes do not establish the actual handle cell, read or observers.
    HandleCellUnavailable,
}

/// A literal method word under the actual guarded remote-call source schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlxMethodCall {
    image: tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    context: tcl_registry::model::ResolvedContext,
    registry: tcl_registry::RegistrySemanticKey,
    /// Canonical source-schema label for presentation; no live command identity.
    pub command: String,
    /// Actual original command extent.
    pub command_span: Span,
    /// Supported literal method source value.
    pub method: String,
    /// Genuine original written method operand extent.
    pub method_span: Span,
    /// Independently resolved extension identity. Source syntax does not issue it.
    pub target: Option<IlxExtension>,
    /// Possible inline constructor labels, separate from evaluated handle identity.
    pub source_target: Option<IlxExtension>,
    /// Unresolved applicability and handle premises remain explicit.
    pub obligations: Vec<IlxSourceObligation>,
    /// Authored remote-call kind; does not establish reached behavior.
    pub dispatch: RemoteDispatch,
}

/// Literal method source cards from the supplied immutable command store.
#[must_use]
pub fn ilx_method_calls(source: &str, registry: &CommandRegistry) -> Vec<IlxMethodCall> {
    if registry.remote_method_commands().is_empty() {
        return Vec::new();
    }
    let Some(context) = crate::OriginalIrulesSourceContext::capture(source, registry) else {
        return Vec::new();
    };
    ilx_method_calls_from_source_context(source, &context)
}

/// Readonly source cards from the document's actual retained analysis. Missing
/// or stale original ownership cannot reopen standalone capture defaults.
#[must_use]
pub fn ilx_method_calls_from_analysis(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
) -> Vec<IlxMethodCall> {
    let Some(context) = crate::OriginalIrulesSourceContext::from_source_analysis(source, analysis)
    else {
        return Vec::new();
    };
    ilx_method_calls_from_source_context(source, &context)
}

/// Select readonly ILX candidates from an independently retained source context.
/// Changed complete source cannot reuse a method, constructor or word extent.
#[must_use]
pub fn ilx_method_calls_from_source_context(
    source: &str,
    context: &crate::OriginalIrulesSourceContext,
) -> Vec<IlxMethodCall> {
    // Implementation contract: naming.consumer.original-ilx-method-source-candidates
    // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
    if !context.matches_source(source) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (span, words) in context.source_vectors() {
        let selected = words
            .with_source_schema(context.context_registry(), |schema| {
                let spec = context
                    .context_registry()
                    .commands()
                    .remote_method(schema.canonical_command)?
                    .calls_method()?;
                if spec.family != RemoteFamily::IRulesLxNode
                    || !schema
                        .argument_count_for_arity()
                        .is_some_and(|count| schema.semantics.arity.accepts(count))
                {
                    return None;
                }
                let method = match spec.method {
                    MethodWord::At(index) => usize::from(index),
                    MethodWord::AfterOptions(index) => {
                        let mut options = schema.semantics.options;
                        options.positional_prefix_words = usize::from(index);
                        options.leading_word_count(schema.words.arguments())?
                    }
                };
                Some((method, usize::from(spec.handle_arg), spec.dispatch))
            })
            .flatten();
        let Some((method_index, handle_index, dispatch)) = selected else {
            continue;
        };
        let Some(method) = literal(words, method_index) else {
            continue;
        };
        let Some(operand) = words.operands().get(method_index).and_then(Option::as_ref) else {
            continue;
        };
        let Some(original_word) = operand.word() else {
            continue;
        };
        let method_span = operand.span();
        let source_target = inline_constructor_source(context, words, handle_index);
        let mut obligations = vec![
            IlxSourceObligation::HandlerApplicability,
            IlxSourceObligation::HandleResultUnavailable,
        ];
        if source_target.is_none() {
            obligations.push(IlxSourceObligation::HandleCellUnavailable);
        }
        out.push(IlxMethodCall {
            image: original_word.image().clone(),
            config: original_word.config(),
            context: context.context().clone(),
            registry: context
                .context_registry()
                .commands()
                .snapshot()
                .semantic_key(),
            command: words.command().to_owned(),
            command_span: *span,
            method: method.to_owned(),
            method_span,
            target: None,
            source_target,
            obligations,
            dispatch,
        });
    }
    out.sort_by_key(|call| (call.method_span.start(), call.method_span.end()));
    out.dedup();
    out
}

fn literal(words: &OriginalRegistryWords, argument: usize) -> Option<&str> {
    let bytes = words.arguments().get(argument)?.literal_bytes()?;
    if bytes.contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

fn inline_constructor_source(
    context: &crate::OriginalIrulesSourceContext,
    words: &OriginalRegistryWords,
    handle: usize,
) -> Option<IlxExtension> {
    let original = words.operands().get(handle)?.as_ref()?.word()?;
    let arena = original.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let ExecutablePart::Command { body } = part.part else {
        return None;
    };
    let mut candidates = context
        .source_vectors()
        .iter()
        .filter(|(span, _)| body.start() <= span.start() && span.end() <= body.end());
    let (_, constructor) = candidates.next()?;
    if candidates.next().is_some() {
        return None;
    }
    let indices = constructor
        .with_source_schema(context.context_registry(), |schema| {
            let spec = context
                .context_registry()
                .commands()
                .remote_method(schema.canonical_command)?
                .opens_handle()?;
            (spec.family == RemoteFamily::IRulesLxNode
                && schema.words.arguments().exact_argv_len()? == usize::from(spec.exact_argc))
            .then_some((usize::from(spec.scope_arg), usize::from(spec.extension_arg)))
        })
        .flatten()?;
    Some(IlxExtension {
        plugin: literal(constructor, indices.0)?.to_owned(),
        extension: literal(constructor, indices.1)?.to_owned(),
    })
}

// The JavaScript side.

/// One `ILXServer.addMethod("name", …)` registration in an extension source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlxMethodRegistration {
    /// The registered method name (the literal's value).
    pub name: String,
    /// Byte span of the name literal **including** its quotes, so an editor
    /// highlights the whole word the way it highlights a Tcl method word.
    pub name_span: Span,
    /// The receiver variable the registration was written on (`ilx`).
    pub receiver: String,
}

/// The `main` entry point of an extension, from its `package.json` text.
///
/// VERIFIED against the tmsh `ilx workspace` reference: "node will look in
/// package.json for a main field that identifies the main entry point of the
/// plugin. If the main field is not present node will look for the file
/// index.js."
///
/// Abstains — falling back to `index.js` — for anything it cannot read
/// literally: malformed JSON, a non-string `main`, an absolute path, or a path
/// climbing out of the extension directory.
#[must_use]
pub fn extension_entry_file(package_json: Option<&str>) -> String {
    let fallback = || "index.js".to_owned();
    let Some(text) = package_json else {
        return fallback();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return fallback();
    };
    let Some(main) = value.get("main").and_then(serde_json::Value::as_str) else {
        return fallback();
    };
    let main = main.trim();
    let unsafe_path = main.is_empty()
        || main.starts_with('/')
        || main.starts_with('\\')
        || main.contains("..")
        || main.contains(':');
    if unsafe_path {
        return fallback();
    }
    main.trim_start_matches("./").to_owned()
}

/// Readonly literal JavaScript method registrations in `source`, with
/// conservative removal exclusions. This source inventory supplies no reached
/// registration, evaluated receiver, live method table or dispatch identity.
///
/// Supported, and nothing else:
///
/// * `var ilx = new f5.ILXServer();` / `new ILXServer()` — any `new`
///   expression whose constructor path ends in `ILXServer`, assigned to a
///   `var` / `let` / `const` / bare identifier;
/// * `ilx.addMethod('name', …)` / `ilx.addMethod("name", …)` on such a
///   receiver, with a **literal** first argument.
///
/// Explicitly *not* recognised, and therefore an abstention rather than a
/// wrong answer: a computed name (`addMethod(name, …)`, a template literal, a
/// concatenation), a method map passed to a constructor, and
/// `setDefaultMethod` (which registers no name, so a call that reaches the
/// default handler has no target to navigate to).
///
/// # `removeMethod` is a subtraction, not a form to ignore
///
/// A written `ilx.removeMethod('m')` keeps the earlier literal registration
/// from supplying an unconditional source candidate. Source order supplies no
/// execution order: removal can occur in a branch, callback or another module.
/// A literal removal excludes that name; a computed removal
/// (`ilx.removeMethod(whatever)`) excludes every source registration because its
/// target is unknown. These exclusions retain uncertainty rather than proving
/// a reached deletion or a current runtime table.
#[must_use]
pub fn extension_registrations(source: &str) -> Vec<IlxMethodRegistration> {
    let tokens = lex_js(source);
    let receivers = ilx_server_receivers(&tokens);
    let removals = method_removals(&tokens, &receivers);
    if removals.removes_an_unknown_name {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        // `<receiver> . addMethod ( "name" ,`
        if token.text != "addMethod" || token.kind != JsTokenKind::Ident {
            continue;
        }
        let Some(dot) = index.checked_sub(1).and_then(|i| tokens.get(i)) else {
            continue;
        };
        let Some(receiver) = index.checked_sub(2).and_then(|i| tokens.get(i)) else {
            continue;
        };
        if dot.text != "." || receiver.kind != JsTokenKind::Ident {
            continue;
        }
        if !receivers.iter().any(|name| name == &receiver.text) {
            continue;
        }
        if tokens.get(index + 1).is_none_or(|t| t.text != "(") {
            continue;
        }
        let Some(name) = tokens.get(index + 2).filter(|t| t.kind == JsTokenKind::Str) else {
            continue;
        };
        // A registration with no second argument registers no handler; a
        // template literal or a concatenation never reaches here because it is
        // not a `Str` token.
        if tokens.get(index + 3).is_none_or(|t| t.text != ",") {
            continue;
        }
        let Some(value) = js_string_value(&name.text) else {
            continue;
        };
        if removals.names.iter().any(|removed| removed == &value) {
            continue;
        }
        out.push(IlxMethodRegistration {
            name: value,
            name_span: name.span,
            receiver: receiver.text.clone(),
        });
    }
    out
}

/// What an extension source takes back out of its own method table.
struct MethodRemovals {
    /// The literal names `removeMethod('name')` removes.
    names: Vec<String>,
    /// Whether any `removeMethod` names something this scanner cannot read as
    /// a literal — in which case the whole table is unknowable.
    removes_an_unknown_name: bool,
}

/// Scan `tokens` for `removeMethod` calls on an `ILXServer` receiver.
///
/// Only the receiver gate and the first argument are read; where the call sits
/// is deliberately ignored — see [`extension_registrations`] on why source
/// order is not execution order.
fn method_removals(tokens: &[JsToken], receivers: &[String]) -> MethodRemovals {
    let mut out = MethodRemovals {
        names: Vec::new(),
        removes_an_unknown_name: false,
    };
    for (index, token) in tokens.iter().enumerate() {
        if token.text != "removeMethod" || token.kind != JsTokenKind::Ident {
            continue;
        }
        let on_ilx_server = index
            .checked_sub(1)
            .and_then(|i| tokens.get(i))
            .is_some_and(|dot| dot.text == ".")
            && index
                .checked_sub(2)
                .and_then(|i| tokens.get(i))
                .is_some_and(|receiver| {
                    receiver.kind == JsTokenKind::Ident
                        && receivers.iter().any(|name| name == &receiver.text)
                });
        if !on_ilx_server || tokens.get(index + 1).is_none_or(|t| t.text != "(") {
            continue;
        }
        match tokens
            .get(index + 2)
            .filter(|t| t.kind == JsTokenKind::Str)
            .and_then(|t| js_string_value(&t.text))
        {
            Some(name) => out.names.push(name),
            None => out.removes_an_unknown_name = true,
        }
    }
    out
}

/// The identifiers this source binds to a `new …ILXServer(…)`.
fn ilx_server_receivers(tokens: &[JsToken]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != JsTokenKind::Ident || token.text != "ILXServer" {
            continue;
        }
        // Walk left over `f5.` / `require('f5-nodejs').` qualification to the
        // `new` keyword; anything else in between means this is not a
        // construction.
        let Some(new_at) = constructor_start(tokens, index) else {
            continue;
        };
        // `… NAME = new …` — the assignment target is two tokens left of `new`.
        let Some(eq) = new_at.checked_sub(1).and_then(|i| tokens.get(i)) else {
            continue;
        };
        let Some(name) = new_at.checked_sub(2).and_then(|i| tokens.get(i)) else {
            continue;
        };
        if eq.text == "=" && name.kind == JsTokenKind::Ident && !out.contains(&name.text) {
            out.push(name.text.clone());
        }
    }
    out
}

/// Index of the `new` keyword introducing the constructor whose final path
/// segment is at `index`, if there is one.
fn constructor_start(tokens: &[JsToken], index: usize) -> Option<usize> {
    let mut at = index;
    loop {
        let previous = at.checked_sub(1)?;
        match tokens.get(previous) {
            Some(token) if token.kind == JsTokenKind::Ident && token.text == "new" => {
                return Some(previous);
            }
            // `f5 . ILXServer` — step over one qualification hop.
            Some(token) if token.text == "." => {
                let owner = previous.checked_sub(1)?;
                let owner_token = tokens.get(owner)?;
                // `require('f5-nodejs').ILXServer` — the hop's owner may be a
                // call, which the paren scan below steps over.
                if owner_token.text == ")" {
                    at = call_callee(tokens, owner)?;
                    continue;
                }
                if owner_token.kind != JsTokenKind::Ident {
                    return None;
                }
                at = owner;
            }
            _ => return None,
        }
    }
}

/// Index of the **callee** of the call whose `)` is at `close` — i.e. the
/// identifier immediately before the matching `(`.
///
/// That, not the paren itself, is what [`constructor_start`] must continue
/// from: it is walking a member chain leftwards, and `require('f5-nodejs')` is
/// one link of it.  `None` when the parens do not balance, or when the callee
/// is not a plain identifier (a computed callee is not a form this scanner
/// claims to understand).
fn call_callee(tokens: &[JsToken], close: usize) -> Option<usize> {
    let mut depth = 0_i32;
    let mut at = close;
    loop {
        let token = tokens.get(at)?;
        if token.text == ")" {
            depth += 1;
        } else if token.text == "(" {
            depth -= 1;
            if depth == 0 {
                // Step past the callee identifier, if any.
                return at
                    .checked_sub(1)
                    .filter(|i| tokens.get(*i).is_some_and(|t| t.kind == JsTokenKind::Ident));
            }
        }
        at = at.checked_sub(1)?;
    }
}

/// What a scanned JavaScript token is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsTokenKind {
    /// An identifier or keyword.
    Ident,
    /// A single-quoted or double-quoted string literal, quotes included.
    Str,
    /// Anything else that matters structurally (`.`, `(`, `,`, `=`, …).
    Punct,
    /// A number, a template literal, or a regular-expression literal — kept as
    /// one opaque token so it can never be mistaken for a name.
    Opaque,
}

/// One scanned JavaScript token.
#[derive(Debug, Clone)]
struct JsToken {
    kind: JsTokenKind,
    text: String,
    span: Span,
}

/// Scan `source` into the coarse token stream [`extension_registrations`] reads.
///
/// Deliberately *not* a JavaScript parser: it skips comments and string bodies
/// so a `//` inside a string cannot swallow a line, and it keeps every
/// identifier, string literal and single punctuation character.  Template
/// literals and regular-expression literals become opaque tokens — they carry
/// no name this module can trust, and swallowing them whole is what keeps a
/// `/["']/` regex from being mis-read as an unterminated string.
fn lex_js(source: &str) -> Vec<JsToken> {
    let bytes = source.as_bytes();
    let mut out: Vec<JsToken> = Vec::new();
    let mut at = 0_usize;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if byte == b'/' && matches!(bytes.get(at + 1), Some(b'/')) {
            at = skip_to(bytes, at + 2, |b| b == b'\n');
            continue;
        }
        if byte == b'/' && matches!(bytes.get(at + 1), Some(b'*')) {
            at = skip_block_comment(bytes, at + 2);
            continue;
        }
        if byte == b'/' && regex_can_start_here(out.last()) {
            let end = skip_delimited(bytes, at + 1, b'/');
            out.push(token(JsTokenKind::Opaque, source, at, end));
            at = end;
            continue;
        }
        if byte == b'`' {
            let end = skip_delimited(bytes, at + 1, b'`');
            out.push(token(JsTokenKind::Opaque, source, at, end));
            at = end;
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            let end = skip_delimited(bytes, at + 1, byte);
            out.push(token(JsTokenKind::Str, source, at, end));
            at = end;
            continue;
        }
        if byte.is_ascii_digit() {
            let end = skip_to(bytes, at, |b| {
                !(b.is_ascii_alphanumeric() || b == b'.' || b == b'_')
            });
            out.push(token(JsTokenKind::Opaque, source, at, end));
            at = end;
            continue;
        }
        if is_ident_byte(byte) {
            let end = skip_to(bytes, at, |b| !is_ident_byte(b));
            out.push(token(JsTokenKind::Ident, source, at, end));
            at = end;
            continue;
        }
        // One structural byte at a time: only `.`, `(`, `)`, `,` and `=` are
        // ever read, and a multi-byte operator's first byte is enough to keep
        // the stream aligned. Non-ASCII bytes (an identifier outside ASCII, an
        // emoji in a comment-free position) advance by their whole character so
        // the scan never splits a UTF-8 sequence.
        let end = at + utf8_len(byte);
        out.push(token(JsTokenKind::Punct, source, at, end.min(bytes.len())));
        at = end;
    }
    out
}

/// Whether a `/` at this point starts a regular-expression literal.
///
/// The standard heuristic: a `/` after a value (identifier, literal, closing
/// bracket) is division, and after anything else it opens a regex. Keywords
/// that *are* followed by a regex (`return /re/`) are the reason this looks at
/// the previous token's spelling for the two that matter here.
fn regex_can_start_here(previous: Option<&JsToken>) -> bool {
    match previous {
        None => true,
        Some(token) => match token.kind {
            JsTokenKind::Str | JsTokenKind::Opaque => false,
            JsTokenKind::Ident => matches!(
                token.text.as_str(),
                "return" | "typeof" | "case" | "in" | "of" | "new" | "delete" | "void"
            ),
            JsTokenKind::Punct => !matches!(token.text.as_str(), ")" | "]" | "}"),
        },
    }
}

/// The byte length of the UTF-8 sequence starting with `lead`.
const fn utf8_len(lead: u8) -> usize {
    match lead {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

/// Advance from `at` until `stop` holds or the input ends; the returned index
/// is one past the stopping byte when one was found.
fn skip_to(bytes: &[u8], at: usize, stop: impl Fn(u8) -> bool) -> usize {
    let mut index = at;
    while index < bytes.len() {
        if stop(bytes[index]) {
            return index + usize::from(bytes[index] == b'\n');
        }
        index += 1;
    }
    bytes.len()
}

fn skip_block_comment(bytes: &[u8], at: usize) -> usize {
    let mut index = at;
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

/// Advance past a `close`-delimited run started at `at`, honouring `\` escapes
/// and stopping at a newline for a quote that never closes.
fn skip_delimited(bytes: &[u8], at: usize, close: u8) -> usize {
    let mut index = at;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'\n' if close != b'`' => return index,
            byte if byte == close => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

fn token(kind: JsTokenKind, source: &str, start: usize, end: usize) -> JsToken {
    let end = end.min(source.len());
    JsToken {
        kind,
        text: source.get(start..end).unwrap_or_default().to_owned(),
        span: Span::new(
            u32::try_from(start).unwrap_or(0),
            u32::try_from(end).unwrap_or(0),
        ),
    }
}

/// The value of a quoted JavaScript string literal, or `None` when it is not a
/// closed literal or carries an escape this module will not interpret.
///
/// Escapes abstain rather than being decoded: an ILX method name is matched
/// byte-for-byte against a Tcl word, and a half-decoded name would match the
/// wrong thing.
fn js_string_value(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let quote = chars.next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let body = text.strip_prefix(quote)?.strip_suffix(quote)?;
    if body.is_empty() || body.contains('\\') {
        return None;
    }
    Some(body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        IlxExtension, extension_entry_file, extension_registrations, ilx_method_calls,
        js_string_value,
    };
    use tcl_registry::CommandRegistry;
    use tcl_registry::remote_method::RemoteDispatch;

    fn irules_registry() -> std::sync::Arc<CommandRegistry> {
        tcl_registry::model::ingress::static_context_for("f5-irules")
            .commands()
            .snapshot()
            .shared_registry()
    }

    fn calls(source: &str) -> Vec<(String, Option<IlxExtension>, RemoteDispatch)> {
        ilx_method_calls(source, &irules_registry())
            .into_iter()
            .map(|call| (call.method, call.target, call.dispatch))
            .collect()
    }

    #[test]
    fn a_written_handle_does_not_issue_runtime_extension_identity() {
        let got = calls(concat!(
            "when HTTP_REQUEST {\n",
            "  set h [ILX::init my_plugin my_extension]\n",
            "  set reply [ILX::call $h my_js_function arg1]\n",
            "}\n",
        ));
        assert_eq!(
            got,
            vec![(
                "my_js_function".to_owned(),
                None,
                RemoteDispatch::Synchronous
            )]
        );
    }

    #[test]
    fn the_timeout_option_and_terminator_are_not_the_method() {
        let got = calls(concat!(
            "when HTTP_REQUEST {\n",
            "  set h [ILX::init p e]\n",
            "  ILX::call $h -timeout 3000 -- real_method x\n",
            "}\n",
        ));
        assert_eq!(
            got,
            vec![("real_method".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn notify_is_a_notification_sharing_the_method_target() {
        let got = calls(concat!(
            "when HTTP_REQUEST {\n",
            "  set h [ILX::init p e]\n",
            "  ILX::notify $h fire_and_forget a b\n",
            "}\n",
        ));
        assert_eq!(
            got,
            vec![(
                "fire_and_forget".to_owned(),
                None,
                RemoteDispatch::Notification
            )]
        );
    }

    #[test]
    fn an_inline_construction_keeps_handle_result_authority_independent() {
        let got = calls("when RULE_INIT {\n  ILX::call [ILX::init p e] m\n}\n");
        assert_eq!(
            got,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn dynamic_plugin_extension_or_reassignment_abstains() {
        // Every one of these keeps the method word (hover can name it) and
        // drops the target (navigation must not guess).
        for source in [
            "when X {\n set h [ILX::init $p e]\n ILX::call $h m\n}\n",
            "when X {\n set h [ILX::init p $e]\n ILX::call $h m\n}\n",
            "when X {\n set h [ILX::init p e]\n set h $other\n ILX::call $h m\n}\n",
            "when X {\n set h [something_else p e]\n ILX::call $h m\n}\n",
            "when X {\n ILX::call $undefined m\n}\n",
            // The one-word `ILX::init` spelling F5 does not document.
            "when X {\n set h [ILX::init e]\n ILX::call $h m\n}\n",
        ] {
            let got = calls(source);
            assert_eq!(got.len(), 1, "{source}");
            assert_eq!(got[0].0, "m", "{source}");
            assert_eq!(got[0].1, None, "the target must abstain: {source}");
        }
    }

    #[test]
    fn a_computed_method_word_is_not_a_site_at_all() {
        for source in [
            "when X {\n set h [ILX::init p e]\n ILX::call $h $method\n}\n",
            "when X {\n set h [ILX::init p e]\n ILX::call $h m$suffix\n}\n",
            "when X {\n set h [ILX::init p e]\n ILX::call $h [get_method]\n}\n",
        ] {
            assert!(calls(source).is_empty(), "{source}");
        }
    }

    #[test]
    fn a_body_that_opens_a_new_frame_does_not_inherit_the_handle() {
        // A `proc` body runs in a fresh local frame, so `$h` is *undefined*
        // when `f` runs — resolving it from the enclosing scope would be a
        // false go-to-definition. Which bodies inherit the caller's frame
        // is registry data (`CommandSpec::body_kind`).
        let got = calls(concat!(
            "set h [ILX::init p e]\n",
            "proc f {} { ILX::call $h m }\n",
        ));
        assert_eq!(
            got,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );

        // Its own written constructor still needs an independent evaluated result.
        let own = calls("proc f {} { set h [ILX::init p e]; ILX::call $h m }\n");
        assert_eq!(
            own,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn control_body_source_cards_do_not_issue_handle_cell_inheritance() {
        // Script geometry selects source cards without proving an entered frame or cell.
        for source in [
            "when X {\n set h [ILX::init p e]\n if {1} { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n foreach i {1 2} { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n catch { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n while {0} { ILX::call $h m }\n}\n",
        ] {
            assert_eq!(
                calls(source),
                vec![("m".to_owned(), None, RemoteDispatch::Synchronous)],
                "{source}"
            );
        }
    }

    #[test]
    fn a_sibling_event_handler_does_not_leak_its_handle() {
        let got = calls(concat!(
            "when CLIENT_ACCEPTED {\n  set h [ILX::init p e]\n}\n",
            "when HTTP_REQUEST {\n  ILX::call $h m\n}\n",
        ));
        assert_eq!(
            got,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn a_switch_arm_is_walked() {
        let got = calls(concat!(
            "when HTTP_REQUEST {\n",
            "  set h [ILX::init p e]\n",
            "  switch [HTTP::uri] {\n",
            "    \"/api\" { ILX::call $h api_method }\n",
            "  }\n",
            "}\n",
        ));
        assert_eq!(
            got,
            vec![("api_method".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn plain_tcl_has_no_ilx_relation() {
        // Criterion 5: the descriptors live on the iRules surface, so a stock
        // Tcl registry finds no command of the name and nothing resolves.
        let registry = CommandRegistry::build_default();
        let got = ilx_method_calls("set h [ILX::init p e]\nILX::call $h m\n", &registry);
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn original_ilx_source_cards_keep_inline_candidates_and_current_owners() {
        // Implementation contract: naming.consumer.original-ilx-method-source-candidates
        // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
        let registry = irules_registry();
        let source = "when HTTP_REQUEST { ILX::call [ILX::init p e] -timeout 3000 -- m }";
        let context = crate::OriginalIrulesSourceContext::capture(source, &registry).unwrap();
        let got = super::ilx_method_calls_from_source_context(source, &context);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].method, "m");
        assert_eq!(
            got[0].source_target,
            Some(IlxExtension {
                plugin: "p".to_owned(),
                extension: "e".to_owned()
            })
        );
        assert_eq!(got[0].target, None);
        assert!(!got[0].obligations.is_empty());
        assert_eq!(&source[got[0].method_span.as_range()], "m");
        assert!(
            super::ilx_method_calls_from_source_context(&format!("{source}\n# changed"), &context)
                .is_empty()
        );
    }

    #[test]
    fn original_ilx_source_cards_do_not_derive_handles_from_cell_or_label_guesses() {
        // Implementation contract: naming.consumer.original-ilx-method-source-candidates
        // docs/design/analysis/name-resolution-proofs/original-ilx-method-source-candidates.md
        for source in [
            "when HTTP_REQUEST {set h [ILX::init p e]; ILX::call $h m}",
            "when HTTP_REQUEST {set h [ILX::init p e]; unknown; ILX::call $h m}",
            "when HTTP_REQUEST { ILX::call [unknown p e] m }",
        ] {
            let got = ilx_method_calls(source, &irules_registry());
            assert_eq!(got.len(), 1, "{source}");
            assert_eq!(got[0].target, None);
            assert_eq!(got[0].source_target, None);
        }
        assert!(
            ilx_method_calls(
                "when HTTP_REQUEST {ILX::call $h $method}",
                &irules_registry()
            )
            .is_empty()
        );
        assert!(
            ilx_method_calls(
                "when HTTP_REQUEST {ILX::call $h m}",
                &CommandRegistry::build_default()
            )
            .is_empty()
        );
    }

    #[test]
    fn addmethod_registrations_are_found_on_an_ilxserver_receiver() {
        let source = concat!(
            "var f5 = require('f5-nodejs');\n",
            "var ilx = new f5.ILXServer();\n",
            "ilx.addMethod('my_js_function', function (req, res) {\n",
            "  res.reply('ok');\n",
            "});\n",
            "ilx.listen();\n",
        );
        let got = extension_registrations(source);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].name, "my_js_function");
        assert_eq!(&source[got[0].name_span.as_range()], "'my_js_function'");
    }

    #[test]
    fn a_bare_or_required_constructor_is_recognised() {
        for source in [
            "const ilx = new ILXServer();\nilx.addMethod(\"m\", cb);\n",
            "let ilx = new require('f5-nodejs').ILXServer();\nilx.addMethod(\"m\", cb);\n",
        ] {
            let got = extension_registrations(source);
            assert_eq!(got.len(), 1, "{source}: {got:?}");
            assert_eq!(got[0].name, "m", "{source}");
        }
    }

    #[test]
    fn unsupported_registration_forms_abstain() {
        for source in [
            // Dynamic name.
            "var ilx = new f5.ILXServer();\nilx.addMethod(name, cb);\n",
            // Template literal / concatenation.
            "var ilx = new f5.ILXServer();\nilx.addMethod(`m`, cb);\n",
            "var ilx = new f5.ILXServer();\nilx.addMethod('a' + 'b', cb);\n",
            // Not an ILXServer receiver.
            "var other = new SomethingElse();\nother.addMethod('m', cb);\n",
            // Different API entirely.
            "var ilx = new f5.ILXServer();\nilx.removeMethod('m');\n",
            "var ilx = new f5.ILXServer();\nilx.setDefaultMethod(cb);\n",
            // Commented out.
            "var ilx = new f5.ILXServer();\n// ilx.addMethod('m', cb);\n",
            "var ilx = new f5.ILXServer();\n/* ilx.addMethod('m', cb); */\n",
            // Inside a string.
            "var ilx = new f5.ILXServer();\nvar s = \"ilx.addMethod('m', cb)\";\n",
        ] {
            assert!(
                extension_registrations(source).is_empty(),
                "must abstain: {source}"
            );
        }
    }

    #[test]
    fn a_regex_literal_does_not_derail_the_scan() {
        let source = concat!(
            "var ilx = new f5.ILXServer();\n",
            "var quote = /[\"']/;\n",
            "ilx.addMethod('after_regex', cb);\n",
        );
        let got = extension_registrations(source);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].name, "after_regex");
    }

    #[test]
    fn duplicate_registrations_are_both_reported() {
        // Reporting both is what lets the caller *see* the ambiguity and
        // abstain; collapsing them here would hide it.
        let source = concat!(
            "var ilx = new f5.ILXServer();\n",
            "ilx.addMethod('dup', a);\n",
            "ilx.addMethod('dup', b);\n",
        );
        assert_eq!(extension_registrations(source).len(), 2);
    }

    #[test]
    fn a_literal_removal_takes_the_method_back_out() {
        // The extension's *running* table has no `m`, so offering the earlier
        // registration would be a wrong answer, not a missing one.
        let removed = concat!(
            "var ilx = new f5.ILXServer();\n",
            "ilx.addMethod('m', cb);\n",
            "ilx.removeMethod('m');\n",
        );
        assert!(extension_registrations(removed).is_empty(), "{removed}");

        // Order is not consulted — a removal written *before* the
        // registration still suppresses it, because source order is not
        // execution order.
        let reordered = concat!(
            "var ilx = new f5.ILXServer();\n",
            "ilx.removeMethod('m');\n",
            "ilx.addMethod('m', cb);\n",
        );
        assert!(extension_registrations(reordered).is_empty(), "{reordered}");

        // Only the named method goes; the rest of the table stands.
        let one_of_two = concat!(
            "var ilx = new f5.ILXServer();\n",
            "ilx.addMethod('m', cb);\n",
            "ilx.addMethod('kept', cb);\n",
            "ilx.removeMethod('m');\n",
        );
        let names: Vec<String> = extension_registrations(one_of_two)
            .into_iter()
            .map(|registration| registration.name)
            .collect();
        assert_eq!(names, vec!["kept".to_owned()]);
    }

    #[test]
    fn a_removal_of_an_unreadable_name_suppresses_the_whole_table() {
        // `removeMethod(whatever)` could take out any name, so nothing in this
        // extension can be resolved.
        for source in [
            "var ilx = new f5.ILXServer();\nilx.addMethod('m', cb);\nilx.removeMethod(name);\n",
            "var ilx = new f5.ILXServer();\nilx.addMethod('m', cb);\nilx.removeMethod(`m`);\n",
        ] {
            assert!(extension_registrations(source).is_empty(), "{source}");
        }

        // A `removeMethod` on something that is not an ILXServer receiver is
        // not this API at all, and changes nothing.
        let unrelated = concat!(
            "var ilx = new f5.ILXServer();\n",
            "ilx.addMethod('m', cb);\n",
            "other.removeMethod(name);\n",
        );
        assert_eq!(extension_registrations(unrelated).len(), 1);
    }

    #[test]
    fn the_entry_point_follows_package_main_or_falls_back() {
        assert_eq!(extension_entry_file(None), "index.js");
        assert_eq!(extension_entry_file(Some("{}")), "index.js");
        assert_eq!(extension_entry_file(Some("not json")), "index.js");
        assert_eq!(
            extension_entry_file(Some(r#"{"main": "./lib/server.js"}"#)),
            "lib/server.js"
        );
        assert_eq!(
            extension_entry_file(Some(r#"{"main": "../escape.js"}"#)),
            "index.js"
        );
        assert_eq!(extension_entry_file(Some(r#"{"main": 7}"#)), "index.js");
    }

    #[test]
    fn escaped_string_values_abstain() {
        assert_eq!(js_string_value("'plain'"), Some("plain".to_owned()));
        assert_eq!(js_string_value("'with\\u0041'"), None);
        assert_eq!(js_string_value("''"), None);
    }
}
