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

use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::LexerConfig;
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::CommandRegistry;
use tcl_registry::arg_role::ArgRole;

use crate::resolve_object_ref_args;

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
    walk(
        source,
        source,
        0,
        rule_module,
        registry,
        &mut scope,
        &mut out,
    );
    out.sort_by(|a, b| {
        (a.range.start(), a.range.end(), &a.name).cmp(&(b.range.start(), b.range.end(), &b.name))
    });
    out
}

/// Segment `slice` (a substring of `full` starting at byte `base`) and collect
/// references, recursing into body / expr / command-substitution arguments with
/// child scopes. Token spans are absolute into `full`.
fn walk(
    full: &str,
    slice: &str,
    base: u32,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
    scope: &mut BindingScope,
    out: &mut Vec<IrulesObjectReference>,
) {
    // Always the iRules dialect: segment with the f5-irules preset so an iRule's
    // `if {expr}{body}` (`}{` valid in TMM) splits into distinct words and its
    // pool/node references are attributed to the right command, not swallowed by
    // the stock-Tcl "extra characters after close-brace" mis-segmentation.
    for cmd in
        segment_commands_with_offset_and_config(slice, base, LexerConfig::for_dialect("f5-irules"))
    {
        let args: Vec<&str> = cmd.args().iter().map(String::as_str).collect();

        // Resolve declared references *before* mutating the binding table, so a
        // same-command `set` re-bind doesn't leak into this call's refs.
        for (argument_index, kinds) in resolve_object_ref_args(cmd.name(), &args, rule_module) {
            if let Some((name, range)) = resolve_arg_value(full, &cmd, argument_index, scope) {
                out.push(IrulesObjectReference {
                    name,
                    kinds,
                    command: cmd.name().to_owned(),
                    argument_index,
                    range,
                });
            }
        }
        record_set_binding(full, &cmd, scope);

        let body_indices = registry.arg_indices_for_role(cmd.name(), &args, ArgRole::Body);
        let mut recursed: HashSet<(u32, u32)> = HashSet::new();
        for body_idx in body_indices {
            let word_index = body_idx + 1;
            if let Some(tok) = cmd.argv.get(word_index)
                && matches!(tok.kind, TokenType::Str | TokenType::Cmd)
                && !inner_is_empty(full, tok)
            {
                let mut child = scope.child();
                recurse_token(full, tok, rule_module, registry, &mut child, out);
                if matches!(tok.kind, TokenType::Cmd) {
                    recursed.insert((tok.span.start(), tok.span.end()));
                }
            }
        }

        // EXPR-role words (`if`/`while`/`expr` conditions) are braced/quoted
        // words whose `[…]` substitutions aren't separate `Cmd` tokens; recurse
        // into the expression text so refs nested in a condition are found.
        let expr_indices = registry.arg_indices_for_role(cmd.name(), &args, ArgRole::Expr);
        for expr_idx in expr_indices {
            let word_index = expr_idx + 1;
            if let Some(tok) = cmd.argv.get(word_index)
                && matches!(tok.kind, TokenType::Str | TokenType::Esc)
                && !inner_is_empty(full, tok)
            {
                let mut child = scope.child();
                recurse_token(full, tok, rule_module, registry, &mut child, out);
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
        if let Some(spec) = registry.get(cmd.name()).and_then(|s| s.case_list)
            && let Some(tok) = case_list_word(&cmd, &args, spec)
            && !inner_is_empty(full, &tok)
        {
            for body in case_list_body_tokens(full, &tok, spec) {
                let mut child = scope.child();
                recurse_token(full, &body, rule_module, registry, &mut child, out);
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
            let mut child = scope.child();
            recurse_token(full, tok, rule_module, registry, &mut child, out);
        }
    }
}

/// The clause-list word of `cmd` (`switch … {pat body …}`), per the registry's
/// [`CaseListSpec`]: skip the command's options (and any that take a value),
/// then its subject words; the clause list is the final braced word.
fn case_list_word(
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
    args: &[&str],
    spec: &tcl_registry::CaseListSpec,
) -> Option<Token> {
    let mut i = 0usize;
    while i < args.len() && args[i].starts_with('-') {
        if args[i] == "--" {
            i += 1;
            break;
        }
        if spec.value_options.contains(&args[i]) {
            i += 1;
        }
        i += 1;
    }
    let case_idx = i + usize::from(spec.subject_args);
    if case_idx != args.len().checked_sub(1)? {
        return None;
    }
    let tok = *cmd.argv.get(case_idx + 1)?;
    matches!(tok.kind, TokenType::Str).then_some(tok)
}

/// The **body** elements of a clause list — the scripts to recurse.
///
/// Walks clause by clause (leading flags, pattern, body) rather than assuming
/// strict alternation, because Expect lets a clause carry `-re` / `-timeout 5`
/// flags that would otherwise shift every following element by one.  The list
/// grammar (`find_element`) is shared with the token walker; only this
/// mechanical split is restated here, since the two consumers want different
/// things out of it — tokens there, referenced objects here.
fn case_list_body_tokens(full: &str, tok: &Token, spec: &tcl_registry::CaseListSpec) -> Vec<Token> {
    let (cstart, cend) = content_range(full, tok);
    let Some(inner) = full.get(cstart..cend) else {
        return Vec::new();
    };
    // Split the list into (element-span, text) pairs.
    let mut elems: Vec<(usize, usize, &str)> = Vec::new();
    let mut scan = 0usize;
    while let Ok(Some(el)) = tcl_syntax::list::find_element(inner, scan) {
        let braced = el.value.start > 0 && inner.as_bytes()[el.value.start - 1] == b'{';
        let start = if braced {
            el.value.start - 1
        } else {
            el.value.start
        };
        elems.push((
            start,
            el.value.end,
            inner.get(el.value.clone()).unwrap_or_default(),
        ));
        if el.next <= scan {
            break;
        }
        scan = el.next;
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    while i < elems.len() {
        // Leading clause flags (Expect only; `switch` declares none).
        while i < elems.len() && spec.clause_flags.contains(&elems[i].2) {
            let takes_value = spec.clause_value_flags.contains(&elems[i].2);
            i += 1;
            if takes_value {
                i += 1;
            }
        }
        // The pattern, then the body.
        if i >= elems.len() {
            break;
        }
        i += 1;
        let Some(&(bstart, bend, _)) = elems.get(i) else {
            break;
        };
        // Only a braced body is a script.
        if inner.as_bytes().get(bstart) == Some(&b'{') {
            out.push(Token::with_content_offset(
                TokenType::Str,
                tcl_lexer::Span::new(
                    u32::try_from(cstart + bstart).unwrap_or(0),
                    u32::try_from(cstart + bend).unwrap_or(0),
                ),
                1,
            ));
        }
        i += 1;
    }
    out
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
    full: &str,
    tok: &Token,
    rule_module: Option<&str>,
    registry: &CommandRegistry,
    scope: &mut BindingScope,
    out: &mut Vec<IrulesObjectReference>,
) {
    let (start, end) = content_range(full, tok);
    if start >= end {
        return;
    }
    let inner = &full[start..end];
    if inner.trim().is_empty() {
        return;
    }
    walk(
        full,
        inner,
        u32::try_from(start).unwrap_or(0),
        rule_module,
        registry,
        scope,
        out,
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
fn record_set_binding(full: &str, cmd: &SegmentedCommand, scope: &mut BindingScope) {
    if cmd.name() != "set" || cmd.args().len() < 2 {
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

    /// Extract object references from `source` against an iRules-aware
    /// registry (`pool` / `snatpool` / `class` are dialect commands).
    fn refs(source: &str) -> Vec<IrulesObjectReference> {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        extract_irules_object_references(source, None, &registry)
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
    use tcl_registry::CommandRegistry;
    use tcl_registry::dialects::DialectSet;

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
