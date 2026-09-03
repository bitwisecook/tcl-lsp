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

//! iRulesLX remote-method sites — the Tcl half of the Tcl↔JavaScript symbol
//! model, and the JavaScript scanner that finds the other half (issue #1707).
//!
//! An iRule reaches a Node.js extension by name:
//!
//! ```tcl
//! when HTTP_REQUEST {
//!     set handle [ILX::init my_plugin my_extension]
//!     set reply [ILX::call $handle my_js_function [HTTP::uri]]
//! }
//! ```
//!
//! and the extension registers that name on an `ILXServer`:
//!
//! ```javascript
//! var f5 = require('f5-nodejs');
//! var ilx = new f5.ILXServer();
//! ilx.addMethod('my_js_function', function (req, res) { res.reply('ok'); });
//! ilx.listen();
//! ```
//!
//! # What is modelled, and what abstains
//!
//! The shape of both calls is **registry data**
//! ([`tcl_registry::remote_method`]), so nothing here matches on a command
//! name: the walk asks the registry which word carries the handle and which
//! carries the method, exactly as the object-reference walk asks which word
//! names a pool.  That is also the dialect gate — the ILX specs exist only on
//! the iRules surface, so a stock-Tcl registry answers "no descriptor" and this
//! module yields nothing.
//!
//! Everything is resolved from *literals*; nothing is guessed:
//!
//! | Written | Result |
//! |---|---|
//! | `set h [ILX::init p e]` … `ILX::call $h m` | resolved to `(p, e, m)` |
//! | `ILX::call [ILX::init p e] m` | resolved — the construction is right there |
//! | `ILX::call $h -timeout 500 -- m` | resolved; options are consumed from the spec's own table |
//! | `ILX::init $p e`, `ILX::init p $e` | handle is unknown → the call abstains |
//! | `set h [something_else]`, `set h $other` | binding widens → the call abstains |
//! | `ILX::call $h $method`, `ILX::call $h m$suffix` | no literal method → no site at all |
//! | `ILX::init e` (one word) | undocumented form → abstains (see [`RemoteHandleSpec::exact_argc`]) |
//! | a handle bound outside a `proc` / `when` body | that body opens a new frame → abstains |
//!
//! A call whose method word is literal but whose handle is unknown is still
//! *reported* — with [`IlxMethodCall::target`] `None` — because hover can
//! honestly say "method name, extension unknown" while navigation abstains.
//!
//! # Bindings follow frames, and frames are registry data
//!
//! A binding is inherited by a nested body only when that body runs in the
//! *caller's* frame, which the registry already says
//! (`CommandSpec::body_kind`): an `if` / `foreach` / `catch` / `switch`-arm
//! body does, and a `proc` body, a `when` event handler, an `oo::define`
//! script and an `uplevel` body do not.  So
//! `set h [ILX::init p e]; proc f {} { ILX::call $h m }` resolves nothing —
//! `$h` is undefined where `f` runs, and offering a target would be the guess
//! criterion 4 forbids.  The walk still *descends* into such a body; it just
//! starts it empty, so an `ILX::init` of its own resolves normally.
//!
//! [`RemoteHandleSpec::exact_argc`]: tcl_registry::remote_method::RemoteHandleSpec::exact_argc

use std::collections::HashMap;

use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{LexerConfig, Span, Token, TokenType};
use tcl_registry::CommandRegistry;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::handle_binding::{HandleClassSource, HandleName};
use tcl_registry::hover::first_positional_index;
use tcl_registry::remote_method::{MethodWord, RemoteDispatch, RemoteFamily};

use crate::walker::{
    MAX_WALK_DEPTH, case_list_body_tokens, case_list_word, content_range, inner_is_empty,
    literal_arg_value, resolve_head, semantic_head, var_token_name,
};

// ---------------------------------------------------------------------------
// The Tcl side
// ---------------------------------------------------------------------------

/// The `(plugin, extension)` pair an ILX handle names.
///
/// Scoped by construction: a method name is unique only *within* one
/// extension, which is why nothing here is keyed by method name alone
/// (issue #1707 criterion 1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IlxExtension {
    /// The ILX plugin name, as written in `ILX::init`.
    pub plugin: String,
    /// The extension name within that plugin.
    pub extension: String,
}

/// One `ILX::call` / `ILX::notify` site with a **literal** method word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlxMethodCall {
    /// The registry command this call resolved to (`ILX::call` / `ILX::notify`),
    /// after aliases and renames — never the raw spelling.
    pub command: String,
    /// Byte span of the whole command.
    pub command_span: Span,
    /// The method name.
    pub method: String,
    /// Byte span of the method word.
    pub method_span: Span,
    /// The extension the handle names, or `None` when the handle is not
    /// statically known.
    pub target: Option<IlxExtension>,
    /// Whether the call waits for a reply.
    pub dispatch: RemoteDispatch,
}

/// Every `ILX::call` / `ILX::notify` site in `source` whose method word is a
/// literal, with the extension its handle names when that is statically known.
///
/// Segmented with the iRules lexer preset, like every other walk in this crate:
/// `if {expr}{body}` is valid in TMM and must split into distinct words.
#[must_use]
pub fn ilx_method_calls(source: &str, registry: &CommandRegistry) -> Vec<IlxMethodCall> {
    if !source_can_hold_a_site(source, registry) {
        return Vec::new();
    }
    let identities = tcl_compiler::realm::document_realm_bindings_with_config(
        source,
        LexerConfig::for_dialect("f5-irules"),
        registry,
    );
    let ctx = IlxCtx {
        full: source,
        registry,
        identities: &identities,
    };
    let mut out = Vec::new();
    let mut scope = HandleScope::default();
    walk(&ctx, source, 0, &mut scope, &mut out, 0);
    out.sort_by_key(|call| (call.method_span.start(), call.method_span.end()));
    out
}

/// Whether `source` can hold a remote-method site at all — the cheap gate in
/// front of the walk.
///
/// Both halves come from the registry, never from a command name written here:
/// a dialect with no RPC commands (every stock-Tcl profile) skips the walk
/// outright, and a document that never spells one of them cannot contain a
/// site — even through a `rename` or an `interp alias`, since creating that
/// binding spells the target once.  Navigation runs this on every hover and
/// every go-to-definition, so it must not cost a segmentation pass.
fn source_can_hold_a_site(source: &str, registry: &CommandRegistry) -> bool {
    let commands = registry.remote_method_commands();
    !commands.is_empty() && commands.iter().any(|name| source.contains(name))
}

/// Variable → the extension its ILX handle names, per scope.
///
/// `None` marks a widened binding — the variable was re-assigned from something
/// that is not a static `ILX::init`, so later reads must fail closed rather than
/// keep the stale association.
#[derive(Clone, Default)]
struct HandleScope {
    bindings: HashMap<String, Option<IlxExtension>>,
}

impl HandleScope {
    fn child(&self) -> Self {
        self.clone()
    }
    fn bind(&mut self, var: &str, target: Option<IlxExtension>) {
        self.bindings.insert(var.to_owned(), target);
    }
    fn lookup(&self, var: &str) -> Option<&IlxExtension> {
        self.bindings.get(var).and_then(Option::as_ref)
    }
}

/// Everything the ILX walk needs that does not change as it recurses.
struct IlxCtx<'a> {
    /// The whole document, so a nested slice's token spans stay absolute.
    full: &'a str,
    /// The dialect registry every layout fact is read from.
    registry: &'a CommandRegistry,
    /// Statically proven command-identity facts, so `::ILX::call` and a proven
    /// `interp alias` / `rename` of it resolve like the bare spelling, and a
    /// spelling whose binding was taken over resolves like nothing.
    identities: &'a tcl_compiler::realm::CommandBindingRealm,
}

/// Segment `slice` (a substring starting at byte `base`), record handle
/// bindings, collect method calls, and recurse into bodies, clause bodies and
/// command substitutions with child scopes.
fn walk(
    ctx: &IlxCtx<'_>,
    slice: &str,
    base: u32,
    scope: &mut HandleScope,
    out: &mut Vec<IlxMethodCall>,
    depth: u32,
) {
    if MAX_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for cmd in
        segment_commands_with_offset_and_config(slice, base, LexerConfig::for_dialect("f5-irules"))
    {
        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
        let head = semantic_head(ctx.registry, resolve_head(ctx.identities, &cmd));
        // Resolve the call *before* this command's own binding effect: an
        // `ILX::call` cannot be re-bound by itself, and a `set` that widens a
        // variable must not widen the call it is not.
        if let Some(call) = method_call(ctx, &cmd, &head, &args, scope) {
            out.push(call);
        }
        record_handle_binding(ctx, &cmd, &head, &args, scope);
        recurse(ctx, &cmd, &head, &args, scope, out, depth);
    }
}

/// Recurse into every region of `cmd` that carries a script: its declared body
/// arguments, a clause list's braced bodies, and any `[…]` substitution.
fn recurse(
    ctx: &IlxCtx<'_>,
    cmd: &SegmentedCommand,
    head: &str,
    args: &[&str],
    scope: &mut HandleScope,
    out: &mut Vec<IlxMethodCall>,
    depth: u32,
) {
    let mut recursed: Vec<(u32, u32)> = Vec::new();
    // Which of this command's body arguments run in the *caller's* frame is
    // registry data (`CommandSpec::body_kind`): `if` / `while` / `foreach` /
    // `switch` bodies do, and a `proc` / `oo::define` / `uplevel` /
    // `namespace eval` body does not.  A body that does not inherit the frame
    // must not inherit the handle bindings either — `set h [ILX::init p e];
    // proc f {} { ILX::call $h m }` reads an *undefined* `$h` when `f` runs,
    // and resolving it from the enclosing scope would be exactly the guess
    // criterion 4 forbids (issue #1707 review).  Asked of the registry, so no
    // command name appears here.
    let inherits_frame = ctx.registry.plain_body_arg_indices(head, args);
    for body_idx in ctx.registry.arg_indices_for_role(head, args, ArgRole::Body) {
        if let Some(tok) = cmd.argv.get(body_idx + 1)
            && matches!(tok.kind, TokenType::Str | TokenType::Cmd)
            && !inner_is_empty(ctx.full, tok)
        {
            if tok.kind == TokenType::Cmd {
                // A substitution supplying a body runs now, in the caller's
                // frame, so its writes are visible to what follows.
                recurse_token(ctx, tok, scope, out, depth + 1);
                recursed.push((tok.span.start(), tok.span.end()));
            } else if inherits_frame.contains(&body_idx) {
                let mut child = scope.child();
                recurse_token(ctx, tok, &mut child, out, depth + 1);
            } else {
                // A new frame: the body still gets walked (an `ILX::init` of
                // its own resolves normally), it just starts with nothing.
                let mut fresh = HandleScope::default();
                recurse_token(ctx, tok, &mut fresh, out, depth + 1);
            }
        }
    }
    if let Some((tok, spec)) = case_list_word(cmd, head, args, ctx.registry)
        && !inner_is_empty(ctx.full, &tok)
    {
        for body in case_list_body_tokens(ctx.full, &tok, &spec) {
            let mut child = scope.child();
            recurse_token(ctx, &body, &mut child, out, depth + 1);
        }
    }
    for tok in &cmd.all_tokens {
        if tok.kind != TokenType::Cmd || inner_is_empty(ctx.full, tok) {
            continue;
        }
        let key = (tok.span.start(), tok.span.end());
        if recursed.contains(&key) {
            continue;
        }
        recursed.push(key);
        recurse_token(ctx, tok, scope, out, depth + 1);
    }
}

/// Recurse into a token's inner content, keeping spans absolute.
fn recurse_token(
    ctx: &IlxCtx<'_>,
    tok: &Token,
    scope: &mut HandleScope,
    out: &mut Vec<IlxMethodCall>,
    depth: u32,
) {
    let (start, end) = content_range(ctx.full, tok);
    if start >= end {
        return;
    }
    let inner = &ctx.full[start..end];
    if inner.trim().is_empty() {
        return;
    }
    walk(
        ctx,
        inner,
        u32::try_from(start).unwrap_or(0),
        scope,
        out,
        depth,
    );
}

/// The method call `cmd` makes, when it makes one with a literal method word.
fn method_call(
    ctx: &IlxCtx<'_>,
    cmd: &SegmentedCommand,
    head: &str,
    args: &[&str],
    scope: &HandleScope,
) -> Option<IlxMethodCall> {
    let spec = ctx.registry.remote_method(head)?.calls_method()?;
    if spec.family != RemoteFamily::IRulesLxNode {
        return None;
    }
    let method_index = match spec.method {
        MethodWord::At(index) => usize::from(index),
        // The option table is the command's own, so `-timeout 500` consumes two
        // words and `--` ends the option run — the method is whatever follows.
        MethodWord::AfterOptions(index) => {
            first_positional_index(ctx.registry.get(head)?.options, args, usize::from(index))
        }
    };
    let (method, method_span) = literal_arg_value(ctx.full, cmd, method_index)?;
    Some(IlxMethodCall {
        command: head.to_owned(),
        command_span: cmd.span,
        method,
        method_span,
        target: handle_target(ctx, cmd, usize::from(spec.handle_arg), scope),
        dispatch: spec.dispatch,
    })
}

/// The extension the handle word at `arg_index` names, when it is statically
/// known: a `$var` bound to an `ILX::init` construction, or the construction
/// written inline.
fn handle_target(
    ctx: &IlxCtx<'_>,
    cmd: &SegmentedCommand,
    arg_index: usize,
    scope: &HandleScope,
) -> Option<IlxExtension> {
    let word_index = arg_index + 1;
    if !cmd
        .single_token_word
        .get(word_index)
        .copied()
        .unwrap_or(false)
    {
        return None;
    }
    let tok = cmd.argv.get(word_index)?;
    match tok.kind {
        TokenType::Var => {
            let var = var_token_name(ctx.full, tok)?;
            scope.lookup(&var).cloned()
        }
        TokenType::Cmd => construction_target(ctx, tok),
        _ => None,
    }
}

/// The extension a `[ILX::init PLUGIN EXTENSION]` construction names.
///
/// Abstains unless the substitution holds exactly one command, that command's
/// registry identity opens a remote handle, its argument count is exactly the
/// documented one, and both name words are literals.
fn construction_target(ctx: &IlxCtx<'_>, tok: &Token) -> Option<IlxExtension> {
    let (start, end) = content_range(ctx.full, tok);
    let inner = ctx.full.get(start..end)?;
    let mut commands = segment_commands_with_offset_and_config(
        inner,
        u32::try_from(start).unwrap_or(0),
        LexerConfig::for_dialect("f5-irules"),
    );
    if commands.len() != 1 {
        return None;
    }
    let cmd = commands.pop()?;
    let head = semantic_head(ctx.registry, resolve_head(ctx.identities, &cmd));
    let spec = ctx.registry.remote_method(&head)?.opens_handle()?;
    if spec.family != RemoteFamily::IRulesLxNode {
        return None;
    }
    let args = cmd.args();
    if args.len() != usize::from(spec.exact_argc) {
        return None;
    }
    let (plugin, _) = literal_arg_value(ctx.full, &cmd, usize::from(spec.scope_arg))?;
    let (extension, _) = literal_arg_value(ctx.full, &cmd, usize::from(spec.extension_arg))?;
    Some(IlxExtension { plugin, extension })
}

/// Record what `cmd` does to a variable that holds an ILX handle.
///
/// The layout comes from the registry's own handle-binding descriptor
/// (`set NAME [TYPE …]`, [`tcl_registry::handle_binding`]), so `::set` and a
/// proven alias or rename of it bind exactly as the bare spelling does and this
/// walk never names `set`.  A binding whose value is not a static `ILX::init`
/// **widens** rather than being ignored: `set h [ILX::init p e]; set h $other;
/// ILX::call $h m` must abstain, not resolve to the stale pair.
fn record_handle_binding(
    ctx: &IlxCtx<'_>,
    cmd: &SegmentedCommand,
    head: &str,
    args: &[&str],
    scope: &mut HandleScope,
) {
    let Some(binding) = ctx.registry.handle_binding(head) else {
        return;
    };
    // `resolve` applies the layout's own keyword gate; the index below then
    // reads the *token* the string form cannot carry.
    let Some(bound) = binding.resolve(args) else {
        return;
    };
    let HandleName::Word(name_index) = binding.name_from else {
        // An implicitly-named handle (snit's hull) names no Tcl variable an
        // ILX call could read.
        return;
    };
    let Some((var, _)) = literal_arg_value(ctx.full, cmd, usize::from(name_index)) else {
        return;
    };
    let HandleClassSource::ConstructionValue(value_index) = bound.class_source else {
        // A layout whose class word is a bare name cannot carry an
        // `[ILX::init …]` construction.
        return;
    };
    let target = cmd
        .single_token_word
        .get(usize::from(value_index) + 1)
        .copied()
        .unwrap_or(false)
        .then(|| cmd.argv.get(usize::from(value_index) + 1))
        .flatten()
        .filter(|tok| tok.kind == TokenType::Cmd)
        .and_then(|tok| construction_target(ctx, tok));
    scope.bind(&var, target);
}

// ---------------------------------------------------------------------------
// The JavaScript side
// ---------------------------------------------------------------------------

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

/// The method table `source` leaves an extension with — its `addMethod`
/// registrations, minus anything a `removeMethod` takes back out.
///
/// Supported, and nothing else (issue #1707 criterion 6):
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
/// `ilx.addMethod('m', cb); ilx.removeMethod('m');` leaves no `m` in the
/// running extension, so offering the earlier registration as `m`'s definition
/// would be a wrong answer rather than a missing one (issue #1707 review).
/// Removal is therefore *modelled*, and deliberately without order: source
/// order is not execution order — a `removeMethod` can sit in a branch, a
/// callback, or a later module — so a literal removal suppresses that name
/// outright, and a removal whose name is **not** literal
/// (`ilx.removeMethod(whatever)`) suppresses the whole table, because it could
/// take out any of it.  That is the same abstention rule the Tcl side applies
/// to a computed method word, on the other side of the boundary.
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
    use tcl_dialect::model::{Family, SurfaceLayer};
    use tcl_registry::CommandRegistry;
    use tcl_registry::remote_method::RemoteDispatch;

    fn irules_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::build_default();
        registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        registry
    }

    fn calls(source: &str) -> Vec<(String, Option<IlxExtension>, RemoteDispatch)> {
        ilx_method_calls(source, &irules_registry())
            .into_iter()
            .map(|call| (call.method, call.target, call.dispatch))
            .collect()
    }

    /// The resolved target a call is expected to carry.
    fn target(plugin: &str, extension: &str) -> IlxExtension {
        IlxExtension {
            plugin: plugin.to_owned(),
            extension: extension.to_owned(),
        }
    }

    #[test]
    fn a_literal_handle_and_method_resolve() {
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
                Some(target("my_plugin", "my_extension")),
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
            vec![(
                "real_method".to_owned(),
                Some(target("p", "e")),
                RemoteDispatch::Synchronous
            )]
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
                Some(target("p", "e")),
                RemoteDispatch::Notification
            )]
        );
    }

    #[test]
    fn an_inline_construction_resolves_without_a_variable() {
        let got = calls("when RULE_INIT {\n  ILX::call [ILX::init p e] m\n}\n");
        assert_eq!(
            got,
            vec![(
                "m".to_owned(),
                Some(target("p", "e")),
                RemoteDispatch::Synchronous
            )]
        );
    }

    #[test]
    fn dynamic_plugin_extension_or_reassignment_abstains() {
        // Every one of these keeps the method word (hover can name it) and
        // drops the target (navigation must not guess) — issue #1707 crit. 4.
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
        // false go-to-definition (issue #1707 review). Which bodies inherit
        // the caller's frame is registry data (`CommandSpec::body_kind`).
        let got = calls(concat!(
            "set h [ILX::init p e]\n",
            "proc f {} { ILX::call $h m }\n",
        ));
        assert_eq!(
            got,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );

        // …and the same body resolves normally from its own `ILX::init`.
        let own = calls("proc f {} { set h [ILX::init p e]; ILX::call $h m }\n");
        assert_eq!(
            own,
            vec![(
                "m".to_owned(),
                Some(target("p", "e")),
                RemoteDispatch::Synchronous
            )]
        );
    }

    #[test]
    fn a_control_flow_body_still_inherits_the_handle() {
        // The other half of the same registry fact: an `if` / `foreach` /
        // `catch` body *is* the caller's frame, so it must keep inheriting.
        for source in [
            "when X {\n set h [ILX::init p e]\n if {1} { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n foreach i {1 2} { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n catch { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n while {0} { ILX::call $h m }\n}\n",
        ] {
            assert_eq!(
                calls(source),
                vec![(
                    "m".to_owned(),
                    Some(target("p", "e")),
                    RemoteDispatch::Synchronous
                )],
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
            vec![(
                "api_method".to_owned(),
                Some(target("p", "e")),
                RemoteDispatch::Synchronous
            )]
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
    fn an_irule_that_mentions_no_rpc_command_is_gated_out() {
        // The cheap pre-check every navigation request pays. It must not
        // change any answer — only skip work — so the assertion is that an
        // ordinary iRule yields nothing while one spelling the command still
        // resolves.
        let registry = irules_registry();
        assert!(
            ilx_method_calls("when HTTP_REQUEST {\n  pool web_pool\n}\n", &registry).is_empty()
        );
        assert!(!super::source_can_hold_a_site(
            "when HTTP_REQUEST { pool web_pool }",
            &registry
        ));
        assert!(super::source_can_hold_a_site(
            "when X { ILX::notify $h m }",
            &registry
        ));
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
