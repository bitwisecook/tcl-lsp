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

//! The iRules object-reference walker.
//!
//! Segments an iRule body, asks [`resolve_object_ref_args`] which argument
//! positions name BIG-IP objects for each command, and resolves those positions
//! to literal object names — including names propagated through `set <var>
//! <literal>` bindings, tracked per event-handler / nested-body scope.

use std::collections::{HashMap, HashSet};

use tcl_compiler::lambda_literal::{LambdaLiteralElements, split_lambda_literal};
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::LexerConfig;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::CommandRegistry;
use tcl_registry::arg_role::ArgRole;

use crate::resolve_object_ref_args;

/// Depth cap for [`walk`]'s (and [`recurse_token`]'s) recursion over nested
/// bodies / `apply` lambdas / `[…]` command substitutions — issue #996.
///
/// This crate is reachable from a WASM host with no stack-size guarantee
/// (via `bigip-query-wasm`, transitively through `tcl-bigip`), so, like
/// `tcl_lsp_core::formatting::engine::MAX_FORMAT_DEPTH` /
/// `tcl_lsp_core::minify::MAX_MINIFY_DEPTH`, this must be safe on a small
/// ambient stack, not just a generously-sized one — same value and
/// reasoning; see those constants' doc comments.
const MAX_WALK_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(128);

/// Consumer-safe semantic category for an iRules object reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrulesObjectReferenceCategory {
    Pool,
    DataGroup,
    Other,
}

/// One iRules object reference resolved from a literal command argument
/// (mirrors `IrulesObjectReference`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrulesObjectReference {
    /// The referenced object name (a literal, or a `set`-propagated constant).
    pub name: String,
    /// Candidate registry kinds the reference can resolve to.
    pub kinds: Vec<&'static str>,
    /// The iRule command the reference came from (e.g. `"pool"`).
    pub command: String,
    /// Registry-resolved command identity used for semantic classification.
    /// Unlike `command`, this follows proven aliases, renames, and qualified
    /// spellings.
    pub effective_command: String,
    /// Typed semantic category, derived by the registry-backed walker.
    pub category: IrulesObjectReferenceCategory,
    /// 0-based argument index (after the command name).
    pub argument_index: usize,
    /// Byte span of the reference token in the source.
    pub range: Span,
}

/// Variable → constant-literal bindings carried through linear copy-propagation
/// within a scope. `None` marks an overdefined (re-assigned from non-literal)
/// binding so later lookups fail closed.
#[derive(Clone, Default)]
struct BindingScope {
    bindings: HashMap<String, Option<String>>,
}

impl BindingScope {
    fn child(&self) -> Self {
        self.clone()
    }
    fn set_const(&mut self, var: &str, value: String) {
        self.bindings.insert(var.to_owned(), Some(value));
    }
    fn widen(&mut self, var: &str) {
        self.bindings.insert(var.to_owned(), None);
    }
    fn lookup(&self, var: &str) -> Option<&String> {
        self.bindings.get(var).and_then(Option::as_ref)
    }
}

/// The sorted, de-duplicated byte spans of every referenced object name — the
/// name-range-only view the semantic-tokens layer consumes. Uses
/// `rule_module = None` so every LTM/GTM reference is recognised.
#[must_use]
pub fn object_ref_spans(source: &str, registry: &CommandRegistry) -> Vec<Span> {
    let mut spans: Vec<Span> = extract_irules_object_references(source, None, registry)
        .into_iter()
        .map(|r| r.range)
        .collect();
    spans.sort_by_key(|s| (s.start(), s.end()));
    spans.dedup();
    spans
}

/// Extract every BIG-IP object reference from iRules `source`, resolving both
/// literal arguments (`pool /Common/foo`) and constants propagated through
/// `set` (`set p /Common/foo; pool $p`). Results are sorted by source position
/// then name.
#[must_use]
pub fn extract_irules_object_references(
    source: &str,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
) -> Vec<IrulesObjectReference> {
    let mut out = Vec::new();
    let mut scope = BindingScope::default();
    // The document's statically proven command-identity facts
    // ([`tcl_compiler::head_identity`]), computed once for the whole rule: a
    // reference-bearing command reached through a proven `interp alias` /
    // `rename` is still a reference, and a spelling whose binding was provably
    // taken over is not (issue #1275).  Empty — and lookup-free — unless the
    // rule binds something.
    let identities = tcl_compiler::head_identity::command_head_identities_with_config(
        source,
        LexerConfig::for_dialect("f5-irules"),
        registry,
    );
    let ctx = WalkCtx {
        full: source,
        rule_module,
        registry,
        identities: &identities,
    };
    walk(&ctx, source, 0, &mut scope, &mut out, 0);
    out.sort_by(|a, b| {
        (a.range.start(), a.range.end(), &a.name).cmp(&(b.range.start(), b.range.end(), &b.name))
    });
    out
}

/// Everything the object-reference walk needs that does not change as it
/// recurses into nested bodies, lambdas, and `[…]` substitutions.
struct WalkCtx<'a> {
    /// The whole rule, so a nested slice's token spans stay absolute.
    full: &'a str,
    /// `"ltm"` / `"gtm"`, selecting the pool-kind family.
    rule_module: Option<&'a str>,
    /// The dialect registry the argument roles and clause-list shape come from.
    registry: &'a CommandRegistry,
    /// The rule's statically proven command-identity facts, so every head is
    /// resolved to the command it *is* rather than the one it is spelled as
    /// (issue #1275).
    identities: &'a tcl_compiler::head_identity::HeadIdentityMap,
}

/// One segmented command's *effective* head — the registry name its spelling
/// resolves to at its own byte offset, or the spelling itself when the rule
/// binds nothing.  Empty for a head whose binding was provably taken over,
/// which every table and registry lookup then answers "unknown" for.
fn resolve_head<'a>(
    identities: &'a tcl_compiler::head_identity::HeadIdentityMap,
    cmd: &'a SegmentedCommand,
) -> &'a str {
    let at = cmd.argv.first().map_or(0, |t| t.span.start());
    identities.head_words(cmd.name(), at).resolved
}

/// Segment `slice` (a substring of the rule starting at byte `base`) and
/// collect references, recursing into body / expr / command-substitution
/// arguments with child scopes. Token spans are absolute into `ctx.full`.
/// `depth` is this call's nesting level — see [`MAX_WALK_DEPTH`].
#[allow(clippy::too_many_lines)] // recursive registry-body walk is intentionally local
fn walk(
    ctx: &WalkCtx<'_>,
    slice: &str,
    base: u32,
    scope: &mut BindingScope,
    out: &mut Vec<IrulesObjectReference>,
    depth: u32,
) {
    let WalkCtx {
        full,
        rule_module,
        registry,
        identities,
    } = *ctx;
    // Native-stack safety net — see `MAX_WALK_DEPTH`'s doc comment (issue
    // #996). Past the cap, stop descending — the references collected up
    // to this nesting level still stand.
    if MAX_WALK_DEPTH.exceeded(depth) {
        return;
    }
    // Always the iRules dialect: segment with the f5-irules preset so an iRule's
    // `if {expr}{body}` (`}{` valid in TMM) splits into distinct words and its
    // pool/node references are attributed to the right command, not swallowed by
    // the stock-Tcl "extra characters after close-brace" mis-segmentation.
    for cmd in
        segment_commands_with_offset_and_config(slice, base, LexerConfig::for_dialect("f5-irules"))
    {
        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();
        // The head's *effective command identity*, resolved exactly as the
        // semantic-token walk resolves it (issue #1275).  Every table and
        // registry lookup below reads it, so `::pool`, a proven
        // `interp alias {} p {} pool`, and a `rename pool p` all find the same
        // object-reference argument, while a `pool` whose binding was provably
        // taken over by a user `proc` finds none.
        let head = resolve_head(identities, &cmd);
        // Registry command specs are canonical unqualified iRules names;
        // Tcl's leading `::` is an absolute-namespace marker, not a distinct
        // command identity.
        // Only canonicalise the iRules global commands this walker owns.
        // `::tcl::dict::*` is a distinct qualified core command whose BODY
        // declarations must remain visible to the registry.
        let canonical = tcl_syntax::naming::canonical_written_command(head);
        let semantic_head = if registry.get_exact(&canonical).is_some() {
            canonical
        } else if canonical.starts_with("::") {
            let rooted_name = canonical.trim_start_matches("::");
            if registry.get_exact(rooted_name).is_some() {
                rooted_name.to_owned()
            } else {
                canonical
            }
        } else {
            canonical
        };

        // Resolve declared references *before* mutating the binding table, so a
        // same-command `set` re-bind doesn't leak into this call's refs.
        for (argument_index, kinds) in resolve_object_ref_args(&semantic_head, &args, rule_module) {
            if let Some((name, range)) = resolve_arg_value(full, &cmd, argument_index, scope) {
                out.push(IrulesObjectReference {
                    name,
                    kinds,
                    command: cmd.name().to_owned(),
                    effective_command: semantic_head.clone(),
                    category: match semantic_head.as_str() {
                        "pool" => IrulesObjectReferenceCategory::Pool,
                        "class" => IrulesObjectReferenceCategory::DataGroup,
                        _ => IrulesObjectReferenceCategory::Other,
                    },
                    argument_index,
                    range,
                });
            }
        }
        record_set_binding(full, &semantic_head, &cmd, scope);

        let body_indices = registry.arg_indices_for_role(&semantic_head, &args, ArgRole::Body);
        let mut recursed: HashSet<(u32, u32)> = HashSet::new();
        for body_idx in body_indices {
            let word_index = body_idx + 1;
            if let Some(tok) = cmd.argv.get(word_index)
                && matches!(tok.kind, TokenType::Str | TokenType::Cmd)
                && !inner_is_empty(full, tok)
            {
                if tok.kind == TokenType::Cmd {
                    // A command substitution supplying a body runs now, in
                    // the caller's frame; only the script value it returns is
                    // later evaluated as the body.  Running the substitution
                    // in a child scope loses Tcl-visible writes such as
                    // `catch [set p new]`.
                    recurse_token(ctx, tok, scope, out, depth + 1);
                    recursed.insert((tok.span.start(), tok.span.end()));
                } else {
                    let mut child = scope.child();
                    recurse_token(ctx, tok, &mut child, out, depth + 1);
                }
            }
        }

        // EXPR-role words (`if`/`while`/`expr` conditions) are braced/quoted
        // words whose `[…]` substitutions aren't separate `Cmd` tokens; recurse
        // into the expression text so refs nested in a condition are found.
        let expr_indices = registry.arg_indices_for_role(&semantic_head, &args, ArgRole::Expr);
        for expr_idx in expr_indices {
            let word_index = expr_idx + 1;
            if let Some(tok) = cmd.argv.get(word_index)
                && matches!(tok.kind, TokenType::Str | TokenType::Esc)
                && !inner_is_empty(full, tok)
            {
                // Expr command substitutions are evaluated once, before the
                // command runs, and in the caller's live frame.  Record any
                // same-span lexer tokens so the generic substitution pass
                // below cannot visit them a second time.
                recurse_token(ctx, tok, scope, out, depth + 1);
                for nested in cmd.all_tokens.iter().filter(|nested| {
                    nested.kind == TokenType::Cmd
                        && tok.span.start() <= nested.span.start()
                        && nested.span.end() <= tok.span.end()
                }) {
                    recursed.insert((nested.span.start(), nested.span.end()));
                }
            }
        }

        // `apply {argList body ?ns?} …` (and any future command sharing the
        // shape) — recurse into the real body *element*, not the whole
        // lambda literal (issue #954): re-segmenting the whole `{argList}
        // {body}` blob as a script previously misread the parameter word as
        // a command name, so an object referenced only inside an apply
        // lambda body embedded in an iRule event handler was invisible to
        // this walker — and thus, per `bigip-cleanup`, looked unreferenced.
        //
        // Unlike an `if`/`foreach`/`switch` body (which shares the enclosing
        // frame — `scope.child()` is correct there), an `apply` body runs in
        // a *fresh* call frame: it does not inherit the caller's `set`-bound
        // constants, only whatever its own actual arguments bind to its own
        // params (`lambda_frame_scope` builds exactly that).
        for lambda_idx in
            registry.arg_indices_for_role(&semantic_head, &args, ArgRole::LambdaLiteral)
        {
            let word_index = lambda_idx + 1;
            if let Some(tok) = cmd.argv.get(word_index)
                && matches!(tok.kind, TokenType::Str)
                && let Some(elems) = split_lambda_literal(full, *tok)
                && let Some(body_span) = elems.body
            {
                let (bstart, bend) = (body_span.start() as usize, body_span.end() as usize);
                if let Some(inner) = full.get(bstart..bend)
                    && !inner.trim().is_empty()
                {
                    let mut child = lambda_frame_scope(full, &cmd, lambda_idx, elems, scope);
                    walk(ctx, inner, body_span.start(), &mut child, out, depth + 1);
                }
            }
        }

        // A clause-list word (`switch … {pat body …}`) is **not** an
        // `ArgRole::Body`, so the recursion above never reached inside it: an
        // object referenced only from a `switch` arm — `switch [HTTP::uri] {
        // "/api/*" { pool /Common/api_pool } }`, an entirely ordinary iRule —
        // was invisible to this walker.  For highlighting that meant the pool
        // name read as a plain string; for the *reference graph* it meant a live
        // pool looked unreferenced, which is what `bigip-cleanup` decides
        // deletions from.  The clause-list shape is registry data
        // (`CommandSpec::case_list`), the same entry the token walker reads.
        if let Some((tok, spec)) = case_list_word(&cmd, &semantic_head, &args, registry)
            && !inner_is_empty(full, &tok)
        {
            for body in case_list_body_tokens(full, &tok, &spec) {
                let mut child = scope.child();
                recurse_token(ctx, &body, &mut child, out, depth + 1);
            }
        }

        // `[…]` command substitutions anywhere else in the command.
        for tok in &cmd.all_tokens {
            if !matches!(tok.kind, TokenType::Cmd) {
                continue;
            }
            let key = (tok.span.start(), tok.span.end());
            if recursed.contains(&key) || inner_is_empty(full, tok) {
                continue;
            }
            recursed.insert(key);
            // Tcl evaluates substitutions left-to-right in the caller's
            // frame. A set inside a command substitution therefore changes
            // the value a following command (or sibling substitution) sees.
            recurse_token(ctx, tok, scope, out, depth + 1);
        }
    }
}

/// Build the fresh binding scope an `apply` lambda body runs in: empty
/// (unlike `scope.child()`'s full clone, this inherits none of the
/// enclosing frame's `set`-bound constants), with the lambda's own params
/// bound to whatever their corresponding actual argument resolves to.
///
/// Inheriting the full scope would let an unrelated enclosing binding that
/// happens to share a lambda-local variable's name (or, in the case of a
/// zero-param lambda, *any* enclosing binding at all) misattribute an
/// object reference to the wrong constant — a lambda body does not close
/// over the caller's locals. `lambda_idx` is the lambda-literal argument's
/// own 0-based (head-excluded) index in `cmd`, so its actual arguments run
/// from `lambda_idx + 1`; each is resolved via the same literal /
/// `$var`-propagation [`resolve_arg_value`] already uses for ordinary
/// references, against the *enclosing* `scope`.
fn lambda_frame_scope(
    full: &str,
    cmd: &SegmentedCommand,
    lambda_idx: usize,
    elems: LambdaLiteralElements,
    scope: &BindingScope,
) -> BindingScope {
    let mut child = BindingScope::default();
    let Some(params_text) = full.get(elems.params.start() as usize..elems.params.end() as usize)
    else {
        return child;
    };
    let lambda_params = tcl_compiler::signature_scan::params::parse_param_list(params_text);
    for (k, param) in lambda_params.iter().enumerate() {
        if let Some((value, _span)) = resolve_arg_value(full, cmd, lambda_idx + 1 + k, scope) {
            child.set_const(&param.name, value);
        }
    }
    child
}

/// The clause-list word of `cmd` (`switch … {pat body …}`), per the registry's
/// [`CaseListSpec`]: skip the command's options (and any that take a value),
/// then its subject words; the clause list is the final braced word.
fn case_list_word(
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
    name: &str,
    args: &[&str],
    registry: &CommandRegistry,
) -> Option<(Token, tcl_registry::CaseListSpec)> {
    let dialect = registry
        .profile()
        .map_or_else(tcl_dialect::DialectSet::empty, |profile| {
            profile.availability_mask
        });
    let (spec, invocation) = registry.case_invocation(name, args, dialect)?;
    let case_idx = invocation.clause_list_index?;
    let tok = *cmd.argv.get(case_idx + 1)?;
    matches!(tok.kind, TokenType::Str).then_some((tok, spec))
}

/// The **body** elements of a clause list — the scripts to recurse.
///
/// The clause split is `tcl-syntax`'s, shared with the semantic-token walker: if
/// the two disagreed about where a clause body is, they would disagree about
/// what the code says.
fn case_list_body_tokens(full: &str, tok: &Token, spec: &tcl_registry::CaseListSpec) -> Vec<Token> {
    let (cstart, cend) = content_range(full, tok);
    let Some(inner) = full.get(cstart..cend) else {
        return Vec::new();
    };
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: spec.clause_flags,
        clause_value_flags: spec.clause_value_flags,
    };
    tcl_syntax::case_list::split_case_list(inner, &shape)
        .into_iter()
        .filter_map(|c| c.body)
        // Only a braced body is a script.
        .filter(|b| b.braced)
        .map(|b| {
            Token::with_content_offset(
                TokenType::Str,
                tcl_lexer::Span::new(
                    u32::try_from(cstart + b.start).unwrap_or(0),
                    u32::try_from(cstart + b.end).unwrap_or(0),
                ),
                1,
            )
        })
        .collect()
}

/// The content byte range of a token (offset past its opening delimiter).
fn content_range(full: &str, tok: &Token) -> (usize, usize) {
    let start = tok.span.start() as usize + tok.content_offset as usize;
    let end = (tok.span.end() as usize).min(full.len());
    (start, end)
}

fn inner_is_empty(full: &str, tok: &Token) -> bool {
    let (start, end) = content_range(full, tok);
    start >= end || full[start..end].trim().is_empty()
}

/// Recurse into a token's inner content (offset-adjusted to absolute spans).
fn recurse_token(
    ctx: &WalkCtx<'_>,
    tok: &Token,
    scope: &mut BindingScope,
    out: &mut Vec<IrulesObjectReference>,
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

/// The trimmed literal `(name, span)` of argument `arg_index`, or `None` when it
/// isn't a usable single-token literal (`$var` / `[cmd]` / multi-token /
/// whitespace). + `_normalise_literal_name`.
fn literal_arg_value(
    full: &str,
    cmd: &SegmentedCommand,
    arg_index: usize,
) -> Option<(String, Span)> {
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
    if !matches!(tok.kind, TokenType::Esc | TokenType::Str) {
        return None;
    }
    let (start, end) = content_range(full, tok);
    let raw = full.get(start..end)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with('$') || trimmed.starts_with('[') {
        return None;
    }
    if trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    let lead = raw.len() - raw.trim_start().len();
    let abs_start = start + lead;
    let abs_end = abs_start + trimmed.len();
    let span = Span::new(
        u32::try_from(abs_start).unwrap_or(0),
        u32::try_from(abs_end).unwrap_or(0),
    );
    Some((trimmed.to_owned(), span))
}

/// The variable name of a single `$var` substitution token (mirrors
/// `_var_token_name`): `None` for array elements (`foo(bar)`).
fn var_token_name(full: &str, tok: &Token) -> Option<String> {
    if !matches!(tok.kind, TokenType::Var) {
        return None;
    }
    let (start, end) = content_range(full, tok);
    let raw = full.get(start..end)?.trim();
    let mut name = raw;
    if name.starts_with('{') && name.ends_with('}') {
        name = &name[1..name.len() - 1];
    }
    if name.is_empty() || name.contains('(') {
        return None;
    }
    Some(name.to_owned())
}

/// Resolve argument `arg_index` to `(name, span)`: the literal value first, then
/// a `$var` substitution's most recent constant binding. Mirrors
/// `_resolve_arg_value`.
fn resolve_arg_value(
    full: &str,
    cmd: &SegmentedCommand,
    arg_index: usize,
    scope: &BindingScope,
) -> Option<(String, Span)> {
    if let Some(literal) = literal_arg_value(full, cmd, arg_index) {
        return Some(literal);
    }
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
    let var = var_token_name(full, tok)?;
    let value = scope.lookup(&var)?.clone();
    // Anchor the resolved reference to the use site (the `$var` token).
    let (start, end) = content_range(full, tok);
    let span = Span::new(
        u32::try_from(start).unwrap_or(0),
        u32::try_from(end).unwrap_or(0),
    );
    Some((value, span))
}

/// Record a `set <var> <literal>` constant binding, or widen on a non-literal
/// RHS.
fn record_set_binding(
    full: &str,
    resolved_head: &str,
    cmd: &SegmentedCommand,
    scope: &mut BindingScope,
) {
    // The head is the *resolved* one, so `::set` and a proven alias / rename of
    // it propagate constants exactly as the bare spelling does, and a `set`
    // whose binding was provably taken over propagates none (issue #1275).
    if resolved_head != "set" || cmd.args().len() < 2 {
        return;
    }
    let var = &cmd.args()[0];
    if var.is_empty()
        || var.chars().any(char::is_whitespace)
        || var.starts_with('$')
        || var.contains('(')
    {
        return;
    }
    if let Some((value, _span)) = literal_arg_value(full, cmd, 1) {
        scope.set_const(var, value);
    } else {
        scope.widen(var);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extract object references from `source` against the profile-stamped
    /// iRules registry (`pool` / `snatpool` / `class` are dialect commands).
    fn refs(source: &str) -> Vec<IrulesObjectReference> {
        let registry = tcl_registry::registry_for_profile(tcl_dialect::DialectProfile::irules());
        extract_irules_object_references(source, None, registry)
    }

    #[test]
    fn extracts_pool_snatpool_and_datagroup_refs() {
        let source = "\n\
            when HTTP_REQUEST {\n\
            \x20   if {[class match -- [HTTP::host] equals /Common/host_dg]} {\n\
            \x20       snatpool /Common/sp1\n\
            \x20       pool /Common/web_pool\n\
            \x20   }\n\
            }\n";
        let by_name: Vec<(String, String)> = refs(source)
            .iter()
            .map(|r| (r.command.clone(), r.name.clone()))
            .collect();
        assert!(by_name.contains(&("class".to_owned(), "/Common/host_dg".to_owned())));
        assert!(by_name.contains(&("snatpool".to_owned(), "/Common/sp1".to_owned())));
        assert!(by_name.contains(&("pool".to_owned(), "/Common/web_pool".to_owned())));
    }

    /// Issue #1275 — the object-reference walk must resolve a command head's
    /// *effective identity*, not its written spelling.
    ///
    /// This is not cosmetic for iRules: the reference graph is what
    /// `bigip-cleanup` decides deletions from, so a pool reached only through
    /// an aliased or renamed `pool` command must still count as referenced.
    ///
    /// tclsh oracle (8.6.16 and 9.0.4, byte-identical): `interp alias {} p {}
    /// pool` makes `p` run `pool`; `rename pool p` moves it and leaves `pool`
    /// gone; a top-level `proc pool …` takes the name over.
    fn ref_names(source: &str) -> Vec<String> {
        refs(source).into_iter().map(|r| r.name).collect()
    }

    /// A `when` body invoking `caller /Common/web_pool`, after `prelude`.
    fn pool_rule(prelude: &str, caller: &str) -> String {
        format!("{prelude}when HTTP_REQUEST {{\n    {caller} /Common/web_pool\n}}\n")
    }

    #[test]
    fn object_refs_follow_an_aliased_command() {
        assert_eq!(
            ref_names(&pool_rule("interp alias {} p {} pool\n", "p")),
            vec!["/Common/web_pool".to_owned()],
        );
        // The `::`-qualified spelling of the alias classifies alike.
        assert_eq!(
            ref_names(&pool_rule("interp alias {} p {} pool\n", "::p")),
            vec!["/Common/web_pool".to_owned()],
        );
        // Guard: an unbound `p` names no BIG-IP object.
        assert!(ref_names(&pool_rule("set y 1\n", "p")).is_empty());
    }

    #[test]
    fn object_refs_follow_a_renamed_command() {
        assert_eq!(
            ref_names(&pool_rule("rename pool p\n", "p")),
            vec!["/Common/web_pool".to_owned()],
        );
        // The old spelling is gone from the rename onwards.
        assert!(
            ref_names(&pool_rule("rename pool p\n", "pool")).is_empty(),
            "a renamed-away `pool` must not keep the object-reference grammar"
        );
    }

    #[test]
    fn object_refs_abstain_for_a_command_shadowed_by_a_user_proc() {
        assert!(
            ref_names(&pool_rule("proc pool {args} { return 1 }\n", "pool")).is_empty(),
            "a user `proc pool` takes the name over; its argument is not a pool name"
        );
        // Guard: the unshadowed command still resolves the reference.
        assert_eq!(
            ref_names(&pool_rule("set y 1\n", "pool")),
            vec!["/Common/web_pool".to_owned()],
        );
    }

    #[test]
    fn object_refs_abstain_for_a_dynamic_binding() {
        assert!(
            ref_names(&pool_rule("rename $old p\n", "p")).is_empty(),
            "a dynamic rename must not make `p` an object-reference command"
        );
        assert_eq!(
            ref_names(&pool_rule("rename $old p\n", "pool")),
            vec!["/Common/web_pool".to_owned()],
            "a dynamic rename must not take `pool`'s grammar away either"
        );
    }

    /// The `set`-constant propagation reads the resolved head too, so a pool
    /// name bound through an aliased `set` still resolves.
    #[test]
    fn constant_propagation_follows_an_aliased_set() {
        let source = "interp alias {} assign {} set\n\
                      when HTTP_REQUEST {\n\
                      assign p /Common/web_pool\n\
                      pool $p\n\
                      }\n";
        assert_eq!(ref_names(source), vec!["/Common/web_pool".to_owned()]);
        // Guard: without the alias, `assign` binds nothing and `$p` is unknown.
        let source = "set y 1\n\
                      when HTTP_REQUEST {\n\
                      assign p /Common/web_pool\n\
                      pool $p\n\
                      }\n";
        assert!(ref_names(source).is_empty());
    }

    #[test]
    fn command_substitution_set_effects_stay_in_the_live_scope() {
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      puts [set p /Common/new]\n\
                      pool $p\n\
                      }\n";
        assert_eq!(ref_names(source), vec!["/Common/new".to_owned()]);
    }

    #[test]
    fn ordered_sibling_command_substitutions_share_effects() {
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      puts [set p /Common/first] [set p /Common/second]\n\
                      pool $p\n\
                      }\n";
        assert_eq!(ref_names(source), vec!["/Common/second".to_owned()]);
    }

    #[test]
    fn body_role_command_substitution_runs_once_in_the_live_scope() {
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      catch [set p /Common/new]\n\
                      pool $p\n\
                      }\n";
        assert_eq!(ref_names(source), vec!["/Common/new".to_owned()]);
    }

    #[test]
    fn expr_role_substitutions_run_live_without_duplicate_references() {
        let source = "when HTTP_REQUEST {\n\
                      set p /Common/old\n\
                      if {[set p /Common/new] ne {}} { set seen 1 }\n\
                      if {[active_members /Common/expr_pool] > 0} { set seen 1 }\n\
                      pool $p\n\
                      }\n";
        assert_eq!(
            ref_names(source),
            vec!["/Common/expr_pool".to_owned(), "/Common/new".to_owned()]
        );
    }

    /// Regression coverage for issue #996: `walk`/`recurse_token`'s mutual
    /// recursion over nested command-substitution bodies is now capped at
    /// `MAX_WALK_DEPTH` (128). 300 nested `[…]` command substitutions is
    /// comfortably past the cap; the assertion is that extraction returns
    /// at all, not what it returns.
    #[test]
    fn deeply_nested_command_substitution_does_not_crash() {
        const DEPTH: usize = 300;
        let mut source = "when HTTP_REQUEST {\n    set x ".to_owned();
        for _ in 0..DEPTH {
            source.push('[');
        }
        source.push_str("pool /Common/p");
        for _ in 0..DEPTH {
            source.push(']');
        }
        source.push_str("\n}\n");
        let _ = refs(&source);
    }

    #[test]
    fn extracts_refs_nested_in_body_and_command_substitution() {
        let source = "\n\
            when HTTP_REQUEST {\n\
            \x20   set count [active_members /Common/app_pool]\n\
            \x20   LB::reselect pool /Common/fallback_pool\n\
            }\n";
        let names: Vec<String> = refs(source).iter().map(|r| r.name.clone()).collect();
        assert!(names.contains(&"/Common/app_pool".to_owned()));
        assert!(names.contains(&"/Common/fallback_pool".to_owned()));
    }
}

#[cfg(test)]
mod case_list_tests {
    use super::extract_irules_object_references;
    use tcl_dialect::DialectSet;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_dialect(DialectSet::parse("f5-irules").expect("the iRules dialect"));
        r
    }

    /// An object referenced only from a `switch` arm must still be found.
    ///
    /// A `switch` case list is not an `ArgRole::Body`, so the walker never
    /// descended into it: a pool used only inside a `switch` arm — an entirely
    /// ordinary iRule — looked **unreferenced**.  For highlighting that meant the
    /// name read as a plain string; for the reference graph it meant
    /// `bigip-cleanup` would have seen a live pool as an orphan.
    #[test]
    fn objects_referenced_from_a_switch_arm_are_found() {
        let registry = reg();
        let src = "when HTTP_REQUEST {\n\
                   \x20   pool /Common/top\n\
                   \x20   switch -glob [HTTP::uri] {\n\
                   \x20       \"/api/*\" { pool /Common/in_switch }\n\
                   \x20       default  { pool /Common/fallback }\n\
                   \x20   }\n\
                   }\n";
        let names: Vec<String> = extract_irules_object_references(src, None, &registry)
            .into_iter()
            .map(|r| r.name)
            .collect();
        for want in ["/Common/top", "/Common/in_switch", "/Common/fallback"] {
            assert!(
                names.iter().any(|n| n == want),
                "`{want}` must be a resolved reference; got {names:?}"
            );
        }
    }
}
