//! The iRules object-reference walker.
//!
//! Segments an iRule body, asks [`resolve_object_ref_args`] which argument
//! positions name BIG-IP objects for each command, and resolves those positions
//! to literal object names — including names propagated through `set <var>
//! <literal>` bindings, tracked per event-handler / nested-body scope.

use std::collections::{HashMap, HashSet};

use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset};
use tcl_lexer::{Span, Token, TokenType};
use tcl_registry::CommandRegistry;
use tcl_registry::arg_role::ArgRole;

use crate::resolve_object_ref_args;

/// One iRules object reference resolved from a literal command argument
/// (mirrors the Python `IrulesObjectReference`).
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
    for cmd in segment_commands_with_offset(slice, base) {
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
