//! Scope-graph helpers — Rust port of
//! ``core/analysis/_analyser/_scope.py``.
//!
//! Pure scope-tree mutation and traversal helpers used by the
//! analyser's command handlers. Rust uses a path-based scope
//! addressing scheme (``Vec<usize>`` indexing into
//! ``result.global_scope.children``) instead of Python's
//! ``Scope.parent`` back-pointers, so any helper that needs to
//! walk up the chain accepts the path slice and traverses the
//! result tree in place. The cost — at most one descent from
//! root per helper call — is bounded by scope depth, which is
//! shallow in practice.
//!
//! Helpers operate on ``&mut Analyser`` rather than free
//! functions because most need both the scope under cursor
//! (resolved from the path) and access to the analyser's
//! tracking maps (``const_strings``, ``regex_vars``, ``ns_cache``,
//! ``result.regex_patterns``).

use tcl_lexer::{Span, Token};

use crate::naming::{normalise_qualified_name, normalise_var_name};

use super::state::Analyser;
use super::types::{Scope, ScopeKind, VarDef};

/// Ancestor path enumerator: yields the active scope's path and
/// each of its proper ancestors back to the root, longest first.
///
/// `[2, 1, 0]` yields `[2, 1, 0]`, `[2, 1]`, `[2]`, `[]`. Used by
/// every helper that walks up the parent chain (matches Python's
/// `while s is not None: ... s = s.parent`).
fn ancestor_paths(start: &[usize]) -> impl Iterator<Item = Vec<usize>> + '_ {
    (0..=start.len()).rev().map(|i| start[..i].to_vec())
}

/// Resolve a scope path to a `&Scope` reference inside `root`.
///
/// Walks `path` index-by-index. Returns the root for an empty
/// path; returns `None` if any index is out of bounds.
fn scope_at<'a>(root: &'a Scope, path: &[usize]) -> Option<&'a Scope> {
    let mut cursor = root;
    for &idx in path {
        cursor = cursor.children.get(idx)?;
    }
    Some(cursor)
}

/// Mutable counterpart to [`scope_at`].
pub(super) fn scope_at_mut<'a>(root: &'a mut Scope, path: &[usize]) -> Option<&'a mut Scope> {
    let mut cursor = root;
    for &idx in path {
        cursor = cursor.children.get_mut(idx)?;
    }
    Some(cursor)
}

impl Analyser {
    /// Resolve the current scope path to a borrow of the active
    /// [`Scope`] inside [`Self::result`]. Convenience wrapper
    /// around [`scope_at`] using ``self.current_scope_path`` as
    /// the path; falls back to the global scope if the path
    /// has gone stale (shouldn't happen during a healthy walk).
    #[must_use]
    pub fn current_scope(&self) -> &Scope {
        scope_at(&self.result.global_scope, &self.current_scope_path)
            .unwrap_or(&self.result.global_scope)
    }

    /// Mutable counterpart to [`Self::current_scope`].
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        let path = self.current_scope_path.clone();
        scope_at_mut(&mut self.result.global_scope, &path)
            .expect("current_scope_path must be valid")
    }

    /// Walk every `Var` token in a segmented command and call
    /// [`Self::record_var_read`] for each.  This is how the
    /// analyser tracks `$x` substitutions in arg positions
    /// (`puts $x`, `string length $name`, etc.) — without it,
    /// `VarDef.references` would only carry the explicit
    /// single-arg `set x` read sites that `handle_set_command`
    /// records.
    ///
    /// The token-text helper uses [`tcl_lexer::SourceMap`] to
    /// recover each `$name` token's textual content from the
    /// source bytes — `Var` tokens carry only their span, not
    /// the inner name.  The `$` prefix is stripped; `${name}`
    /// braced forms are decoded down to `name`.
    pub fn record_arg_var_reads(
        &mut self,
        cmd: &crate::segmenter::SegmentedCommand,
        scope_path: &[usize],
    ) {
        use tcl_lexer::TokenType;
        // Collect (name, span) tuples in a first pass so the
        // source borrow releases before we mutate `self` via
        // `record_var_read`.  Token spans that exceed the
        // source bounds (which happens in test harnesses that
        // pass synthetic tokens against an empty source) are
        // silently skipped — they aren't reading anything
        // meaningful.
        let head_span = cmd.argv.first().map(|t| t.span);
        let source_len = u32::try_from(self.source.len()).unwrap_or(u32::MAX);
        let mut reads: Vec<(String, tcl_lexer::Span)> = Vec::new();
        for tok in &cmd.all_tokens {
            if tok.kind != TokenType::Var {
                continue;
            }
            if head_span.is_some_and(|hs| tok.span == hs) {
                continue;
            }
            if tok.span.end() > source_len {
                continue;
            }
            let Some(name) = var_name_from_span(&self.source, tok.span) else {
                continue;
            };
            reads.push((name.to_string(), tok.span));
        }
        for (name, span) in reads {
            self.record_var_read(&name, span, scope_path);
        }

        // `$var` reads also occur inside command substitutions
        // (`[…]`) and braced `expr` / condition arguments, which the
        // main body walk treats as opaque tokens.  Collect those
        // here too so `VarDef.references` is complete for the
        // references / document-highlight providers and the
        // minifier's rename pass.  `record_var_read` no-ops on names
        // that aren't in the current scope, so liberal collection is
        // safe.
        if let Some(registry) = self.registry.as_ref() {
            let mut extra: Vec<(String, tcl_lexer::Span)> = Vec::new();
            let cmd_name = cmd.texts.first().map_or("", String::as_str);
            let post: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
            let expr_idx: std::collections::HashSet<usize> = registry
                .arg_indices_for_role(cmd_name, &post, tcl_registry::ArgRole::Expr)
                .into_iter()
                .collect();
            // Top-level command-substitution tokens.
            for tok in &cmd.all_tokens {
                if tok.kind == tcl_lexer::TokenType::Cmd {
                    collect_cmd_subst_reads(&self.source, *tok, registry, &mut extra);
                }
            }
            // Braced expr arguments.
            for (i, arg) in cmd.argv.iter().enumerate().skip(1) {
                if arg.kind == tcl_lexer::TokenType::Str && expr_idx.contains(&(i - 1)) {
                    collect_expr_reads(&self.source, *arg, registry, &mut extra);
                }
            }
            for (name, span) in extra {
                self.record_var_read(&name, span, scope_path);
            }
        }
    }

    /// Compute the scope-resolved qualified name for a command
    /// invocation at the current walk position.  Mirrors
    /// Python's `_resolve_command_qualified_name` logic.
    ///
    /// * Names starting with `::` are already absolute and are
    ///   returned as-is.
    /// * Names with embedded `::` but no leading `::` are
    ///   absolute-from-global by Tcl convention; the helper
    ///   prepends `::`.
    /// * Simple names are joined with the nearest enclosing
    ///   namespace scope: `cmd` inside `namespace eval ::ns`
    ///   becomes `::ns::cmd`; at the top level it becomes
    ///   `::cmd`.
    ///
    /// Returned strings are intended for matching against
    /// [`super::types::ProcDef::qualified_name`] in references
    /// / document-highlight / rename providers.  The value is
    /// not authoritative for runtime dispatch — it's a
    /// candidate that lets call-site → declaration matching
    /// resolve relative call shapes.
    #[must_use]
    pub fn resolve_command_qualified_name(&self, cmd_name: &str) -> String {
        if cmd_name.starts_with("::") {
            return cmd_name.to_string();
        }
        if cmd_name.contains("::") {
            return format!("::{cmd_name}");
        }
        // Walk the current scope path up to the nearest
        // namespace scope and join its `name` with the cmd.
        // The global scope's name is `"::"` so the simple-name
        // case at the top level becomes `"::cmd"`.
        let mut path = self.current_scope_path.clone();
        loop {
            let scope =
                scope_at(&self.result.global_scope, &path).unwrap_or(&self.result.global_scope);
            if scope.kind == super::types::ScopeKind::Namespace || path.is_empty() {
                let prefix = scope.name.as_str();
                return if prefix == "::" {
                    format!("::{cmd_name}")
                } else {
                    format!("{prefix}::{cmd_name}")
                };
            }
            if path.is_empty() {
                break;
            }
            path.pop();
        }
        format!("::{cmd_name}")
    }

    /// Record a constant string assignment for `var_name` in the
    /// scope at `scope_path`.
    ///
    /// Mirrors `_set_const_string` in
    /// `core/analysis/_analyser/_scope.py:32-39`.
    pub fn set_const_string(
        &mut self,
        var_name: &str,
        value: String,
        value_span: Span,
        scope_path: &[usize],
    ) {
        self.const_strings
            .entry(scope_path.to_vec())
            .or_default()
            .insert(var_name.to_string(), (value, value_span));
    }

    /// Remove constant-value knowledge for `var_name` (re-assigned
    /// dynamically).
    ///
    /// Mirrors `_clear_const_string` in
    /// `core/analysis/_analyser/_scope.py:41-46`.
    pub fn clear_const_string(&mut self, var_name: &str, scope_path: &[usize]) {
        if let Some(map) = self.const_strings.get_mut(scope_path) {
            map.remove(var_name);
        }
    }

    /// Look up the constant string value for `var_name`, walking
    /// the scope chain from `scope_path` outwards.
    ///
    /// Mirrors `_lookup_const_string` in
    /// `core/analysis/_analyser/_scope.py:48-56`. Returns the
    /// nearest enclosing scope's value, or `None` if the var
    /// isn't tracked anywhere on the chain.
    #[must_use]
    pub fn lookup_const_string(&self, var_name: &str, scope_path: &[usize]) -> Option<&str> {
        self.lookup_const_string_with_span(var_name, scope_path)
            .map(|(value, _)| value)
    }

    /// Look up the constant string value for `var_name` along
    /// with the source span recorded for the defining ``set``.
    /// Used by the regex-pattern emitters to also tag the
    /// defining ``set var "..."`` value as a `RegexPattern` so
    /// semantic-token highlighting fires on the literal.
    ///
    /// Mirrors the inner walk of `_record_defining_set_as_regex`
    /// in `core/analysis/_analyser/_scope.py:58-79` together
    /// with the ``Optional[span]`` field returned by
    /// `_lookup_const_string`.
    #[must_use]
    pub fn lookup_const_string_with_span(
        &self,
        var_name: &str,
        scope_path: &[usize],
    ) -> Option<(&str, Span)> {
        for ancestor in ancestor_paths(scope_path) {
            if let Some(map) = self.const_strings.get(&ancestor) {
                if let Some((value, span)) = map.get(var_name) {
                    return Some((value.as_str(), *span));
                }
            }
        }
        None
    }

    /// Compute the namespace string for a scope path, with a
    /// per-call cache.
    ///
    /// Mirrors `_namespace_from_scope` in
    /// `core/analysis/_analyser/_scope.py:126-156`. Walks the
    /// scope path collecting ``ScopeKind::Namespace`` names, then
    /// joins them via [`normalise_qualified_name`]. Caches the
    /// result on `self.ns_cache` keyed by the path.
    pub fn namespace_from_scope_path(&mut self, scope_path: &[usize]) -> String {
        if let Some(cached) = self.ns_cache.get(scope_path) {
            return cached.clone();
        }
        let mut parts: Vec<String> = Vec::new();
        let mut cursor = &self.result.global_scope;
        // The global scope itself is not a namespace component;
        // contribute its name only if we're past the root.
        for &idx in scope_path {
            let Some(child) = cursor.children.get(idx) else {
                break;
            };
            if child.kind == ScopeKind::Namespace {
                parts.push(child.name.clone());
            }
            cursor = child;
        }
        let result = if parts.is_empty() {
            "::".to_string()
        } else {
            let mut ns = "::".to_string();
            for part in parts {
                if part.starts_with("::") {
                    ns = normalise_qualified_name(&part);
                } else {
                    ns = normalise_qualified_name(&format!("{ns}::{part}"));
                }
            }
            ns
        };
        self.ns_cache.insert(scope_path.to_vec(), result.clone());
        result
    }

    /// Record a variable read for go-to-definition / find-references.
    ///
    /// Mirrors `_record_var_read` in
    /// `core/analysis/_analyser/_scope.py:158-175`. Looks for
    /// the variable in the scope at `scope_path`; falls back to
    /// the global scope for ``::``- and ``static::``-prefixed
    /// names. W210 (read-before-set) is emitted by the SSA-based
    /// pass elsewhere — this helper only records the reference
    /// span.
    pub fn record_var_read(&mut self, name: &str, read_span: Span, scope_path: &[usize]) {
        let base_name = normalise_var_name(name);
        if base_name.is_empty() {
            return;
        }

        // Local scope first.
        let path = scope_path.to_vec();
        let base_owned = base_name.to_string();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) {
            if let Some(var) = scope.variables.get_mut(&base_owned) {
                var.references.push(read_span);
                return;
            }
        }

        // Cross-rule variables — fall back to global scope.
        if base_owned.starts_with("::") || base_owned.starts_with("static::") {
            if let Some(var) = self.result.global_scope.variables.get_mut(&base_owned) {
                var.references.push(read_span);
            }
        }
    }

    /// Record a variable definition in the scope at `scope_path`.
    ///
    /// Mirrors `_define_var` in
    /// `core/analysis/_analyser/_scope.py:182-206`. New definitions
    /// are inserted into the scope's `variables` map and registered
    /// into ``result.all_variables`` keyed by ``"<scope_name>::<base_name>"``.
    /// Re-defining an existing variable does not overwrite — it
    /// only escalates the `warn_if_unused` flag.
    pub fn define_var(
        &mut self,
        name: &str,
        tok: Token,
        scope_path: &[usize],
        warn_if_unused: bool,
        definition_span: Option<Span>,
    ) {
        let base_name = normalise_var_name(name);
        if base_name.is_empty() {
            return;
        }
        let base_owned = base_name.to_string();
        let span = definition_span.unwrap_or(tok.span);

        let path = scope_path.to_vec();
        let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path) else {
            return;
        };

        if !scope.variables.contains_key(&base_owned) {
            let var = VarDef {
                name: base_owned.clone(),
                definition_span: span,
                references: Vec::new(),
                warn_if_unused,
            };
            scope.variables.insert(base_owned.clone(), var.clone());
            let key = format!("{}::{base_owned}", scope.name);
            self.result.all_variables.insert(key, var);
        } else if warn_if_unused {
            if let Some(existing) = scope.variables.get_mut(&base_owned) {
                existing.warn_if_unused = true;
            }
        }
    }

    /// Walk the scope tree depth-first, yielding the scope at
    /// `scope_path` and every descendant in declaration order.
    ///
    /// Mirrors the generator in `_walk_scopes`
    /// (`core/analysis/_analyser/_scope.py:177-180`). The Rust
    /// implementation collects into a `Vec` rather than building
    /// a generator chain — same iteration order, simpler
    /// borrowing.
    #[must_use]
    pub fn walk_scopes_from(&self, scope_path: &[usize]) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        let Some(start) = scope_at(&self.result.global_scope, scope_path) else {
            return out;
        };
        walk_scopes_helper(start, scope_path, &mut out);
        out
    }
}

fn walk_scopes_helper(scope: &Scope, path: &[usize], out: &mut Vec<Vec<usize>>) {
    out.push(path.to_vec());
    for (i, child) in scope.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(i);
        walk_scopes_helper(child, &child_path, out);
    }
}

// ---------------------------------------------------------------------------
// Substituted / expr `$var` read collection
// ---------------------------------------------------------------------------

/// Extract the base variable name from a `Var` token's source span:
/// `$name` / `${name}` → `name`, and `$arr(idx)` → `arr` (the array
/// index is dropped so element reads attribute to the base array
/// variable).
///
/// Handles the lexer's `Var`-token span convention, which excludes
/// the closing `}` of a braced reference (`${x}` spans `${x`), so the
/// trailing brace is stripped only when present.  The array-index
/// suffix is dropped for the unbraced form only — inside `${…}` the
/// parentheses are part of a literal name.  The recorded base name is
/// re-normalised by [`normalise_var_name`] in `record_var_read`; this
/// helper keeps that contract so reads attribute to the right slot.
fn var_name_from_span(source: &str, span: Span) -> Option<&str> {
    let (s, e) = (span.start() as usize, span.end() as usize);
    if s > e || e > source.len() {
        return None;
    }
    let text = &source[s..e];
    let rest = text.strip_prefix('$')?;
    let name = if let Some(inner) = rest.strip_prefix('{') {
        // Braced `${name}` — the span may omit the closing brace.
        inner.strip_suffix('}').unwrap_or(inner)
    } else {
        // Unbraced `$arr(idx)` — keep everything before the index.
        match rest.find('(') {
            Some(i) => &rest[..i],
            None => rest,
        }
    };
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Inner content text + base offset of a wrapper token (`[…]` /
/// `{…}`): the delimiter is a single leading byte, the span
/// excludes the closing delimiter.
fn inner_of(source: &str, span: Span) -> Option<(&str, u32)> {
    let (s, e) = (span.start() as usize, span.end() as usize);
    if s + 1 > e || e > source.len() {
        return None;
    }
    Some((&source[s + 1..e], span.start() + 1))
}

/// Collect `$var` reads inside a command-substitution token,
/// recursing into nested substitutions, expr arguments, and
/// same-scope (plain) body arguments.  Structural-body arguments
/// (proc / namespace / oo) introduce a new scope and are skipped.
fn collect_cmd_subst_reads(
    source: &str,
    cmd_tok: Token,
    registry: &tcl_registry::CommandRegistry,
    out: &mut Vec<(String, Span)>,
) {
    let Some((inner, base)) = inner_of(source, cmd_tok.span) else {
        return;
    };
    for cmd in crate::segmenter::segment_commands_with_offset(inner, base) {
        collect_script_command_reads(source, &cmd, registry, out);
    }
}

/// Record reads for one command appearing inside a substitution:
/// top-level `$var` tokens, plus recursion into nested
/// substitutions / expr args / plain bodies.
fn collect_script_command_reads(
    source: &str,
    cmd: &crate::segmenter::SegmentedCommand,
    registry: &tcl_registry::CommandRegistry,
    out: &mut Vec<(String, Span)>,
) {
    let head_span = cmd.argv.first().map(|t| t.span);
    for tok in &cmd.all_tokens {
        if tok.kind == tcl_lexer::TokenType::Var && Some(tok.span) != head_span {
            if let Some(name) = var_name_from_span(source, tok.span) {
                out.push((name.to_owned(), tok.span));
            }
        } else if tok.kind == tcl_lexer::TokenType::Cmd {
            collect_cmd_subst_reads(source, *tok, registry, out);
        }
    }
    let cmd_name = cmd.texts.first().map_or("", String::as_str);
    let post: Vec<&str> = cmd.texts.iter().skip(1).map(String::as_str).collect();
    let expr_idx: std::collections::HashSet<usize> = registry
        .arg_indices_for_role(cmd_name, &post, tcl_registry::ArgRole::Expr)
        .into_iter()
        .collect();
    let body_idx: std::collections::HashSet<usize> = registry
        .arg_indices_for_role(cmd_name, &post, tcl_registry::ArgRole::Body)
        .into_iter()
        .collect();
    let structural = registry
        .get(cmd_name)
        .is_some_and(|s| s.body_kind == tcl_registry::BodyKind::Structural);
    for (i, arg) in cmd.argv.iter().enumerate().skip(1) {
        if arg.kind != tcl_lexer::TokenType::Str {
            continue;
        }
        let pidx = i - 1;
        if expr_idx.contains(&pidx) {
            collect_expr_reads(source, *arg, registry, out);
        } else if body_idx.contains(&pidx) && !structural {
            // Plain body inside a substitution — same scope, and the
            // main walk never reached it.
            if let Some((inner, base)) = inner_of(source, arg.span) {
                for sub in crate::segmenter::segment_commands_with_offset(inner, base) {
                    collect_script_command_reads(source, &sub, registry, out);
                }
            }
        }
    }
}

/// Collect `$var` reads inside a braced `expr` argument: every
/// `Var` token plus any command substitution nested in the
/// expression.
fn collect_expr_reads(
    source: &str,
    expr_tok: Token,
    registry: &tcl_registry::CommandRegistry,
    out: &mut Vec<(String, Span)>,
) {
    let Some((inner, base)) = inner_of(source, expr_tok.span) else {
        return;
    };
    let Ok(tokens) = tcl_lexer::Lexer::new(inner).tokenise_all() else {
        return;
    };
    for tok in tokens {
        let abs = Span::new(tok.span.start() + base, tok.span.end() + base);
        match tok.kind {
            tcl_lexer::TokenType::Var => {
                if let Some(name) = var_name_from_span(source, abs) {
                    out.push((name.to_owned(), abs));
                }
            }
            tcl_lexer::TokenType::Cmd => {
                collect_cmd_subst_reads(
                    source,
                    Token::new(tcl_lexer::TokenType::Cmd, abs),
                    registry,
                    out,
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::TokenType;

    fn span(start: u32, end: u32) -> Span {
        Span::new(start, end)
    }

    fn make_analyser_with_namespace(ns_name: &str) -> Analyser {
        let mut a = Analyser::new();
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, ns_name));
        a
    }

    #[test]
    fn current_scope_defaults_to_global() {
        let a = Analyser::new();
        assert_eq!(a.current_scope().name, "::");
        assert_eq!(a.current_scope().kind, ScopeKind::Global);
    }

    #[test]
    fn current_scope_follows_path() {
        let mut a = make_analyser_with_namespace("ns1");
        a.current_scope_path = vec![0];
        assert_eq!(a.current_scope().name, "ns1");
        assert_eq!(a.current_scope().kind, ScopeKind::Namespace);
    }

    #[test]
    fn set_then_lookup_const_string() {
        let mut a = Analyser::new();
        a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
        assert_eq!(a.lookup_const_string("x", &[]), Some("hello"));
    }

    #[test]
    fn lookup_const_string_walks_chain() {
        // x defined in global; lookup from a child scope finds it.
        let mut a = make_analyser_with_namespace("ns1");
        a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
        assert_eq!(a.lookup_const_string("x", &[0]), Some("hello"));
    }

    #[test]
    fn lookup_const_string_inner_shadows_outer() {
        let mut a = make_analyser_with_namespace("ns1");
        a.set_const_string("x", "outer".to_string(), span(0, 5), &[]);
        a.set_const_string("x", "inner".to_string(), span(10, 15), &[0]);
        assert_eq!(a.lookup_const_string("x", &[0]), Some("inner"));
        assert_eq!(a.lookup_const_string("x", &[]), Some("outer"));
    }

    #[test]
    fn clear_const_string_removes_entry() {
        let mut a = Analyser::new();
        a.set_const_string("x", "hello".to_string(), span(0, 5), &[]);
        a.clear_const_string("x", &[]);
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn lookup_const_string_returns_none_for_unknown() {
        let a = Analyser::new();
        assert_eq!(a.lookup_const_string("x", &[]), None);
    }

    #[test]
    fn namespace_from_scope_path_global_returns_root() {
        let mut a = Analyser::new();
        assert_eq!(a.namespace_from_scope_path(&[]), "::");
    }

    #[test]
    fn namespace_from_scope_path_single_relative() {
        let mut a = make_analyser_with_namespace("ns1");
        assert_eq!(a.namespace_from_scope_path(&[0]), "::ns1");
    }

    #[test]
    fn namespace_from_scope_path_absolute_rebases() {
        let mut a = make_analyser_with_namespace("outer");
        // Add an absolute child namespace.
        a.result.global_scope.children[0]
            .children
            .push(Scope::new(ScopeKind::Namespace, "::abs"));
        // Path [0, 0] should rebase at ::abs, not nest under outer.
        assert_eq!(a.namespace_from_scope_path(&[0, 0]), "::abs");
    }

    #[test]
    fn namespace_from_scope_path_caches_result() {
        let mut a = make_analyser_with_namespace("ns1");
        let _ = a.namespace_from_scope_path(&[0]);
        assert!(a.ns_cache.contains_key(&vec![0_usize]));
    }

    #[test]
    fn namespace_from_scope_path_skips_proc_scopes() {
        // proc scopes don't contribute to the namespace path.
        let mut a = Analyser::new();
        let mut ns = Scope::new(ScopeKind::Namespace, "ns1");
        ns.children
            .push(Scope::new(ScopeKind::Proc, "::ns1::myproc"));
        a.result.global_scope.children.push(ns);
        assert_eq!(a.namespace_from_scope_path(&[0, 0]), "::ns1");
    }

    #[test]
    fn define_var_inserts_into_scope_and_all_variables() {
        let mut a = Analyser::new();
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("x", tok, &[], true, None);
        assert!(a.result.global_scope.variables.contains_key("x"));
        assert!(a.result.all_variables.contains_key("::::x"));
        let v = &a.result.global_scope.variables["x"];
        assert!(v.warn_if_unused);
        assert_eq!(v.definition_span, span(0, 4));
    }

    #[test]
    fn define_var_existing_only_escalates_warn_flag() {
        let mut a = Analyser::new();
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("x", tok, &[], false, None);
        assert!(!a.result.global_scope.variables["x"].warn_if_unused);
        a.define_var("x", tok, &[], true, None);
        assert!(a.result.global_scope.variables["x"].warn_if_unused);
        // Definition span unchanged — first definition wins.
        assert_eq!(
            a.result.global_scope.variables["x"].definition_span,
            span(0, 4)
        );
    }

    #[test]
    fn define_var_uses_explicit_definition_span() {
        let mut a = Analyser::new();
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("x", tok, &[], true, Some(span(10, 15)));
        assert_eq!(
            a.result.global_scope.variables["x"].definition_span,
            span(10, 15)
        );
    }

    #[test]
    fn record_var_read_local_scope() {
        let mut a = Analyser::new();
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("x", tok, &[], true, None);
        a.record_var_read("x", span(20, 21), &[]);
        assert_eq!(
            a.result.global_scope.variables["x"].references,
            vec![span(20, 21)]
        );
    }

    #[test]
    fn record_var_read_global_fallback_for_qualified_name() {
        // Var defined in global scope, read from a child namespace
        // via qualified name — should fall back to global.
        let mut a = make_analyser_with_namespace("ns1");
        let tok = Token::new(TokenType::Esc, span(0, 4));
        a.define_var("::x", tok, &[], true, None);
        a.record_var_read("::x", span(20, 21), &[0]);
        assert_eq!(
            a.result.global_scope.variables["::x"].references,
            vec![span(20, 21)]
        );
    }

    #[test]
    fn record_var_read_no_match_silent() {
        let mut a = Analyser::new();
        // No var defined; read does nothing.
        a.record_var_read("x", span(20, 21), &[]);
        // No panic, no references recorded anywhere.
        assert!(a.result.global_scope.variables.is_empty());
    }

    #[test]
    fn walk_scopes_from_root_visits_all() {
        let mut a = Analyser::new();
        let mut ns = Scope::new(ScopeKind::Namespace, "ns1");
        ns.children.push(Scope::new(ScopeKind::Proc, "::ns1::p"));
        a.result.global_scope.children.push(ns);
        a.result
            .global_scope
            .children
            .push(Scope::new(ScopeKind::Namespace, "ns2"));
        let paths = a.walk_scopes_from(&[]);
        assert_eq!(paths, vec![vec![], vec![0], vec![0, 0], vec![1]]);
    }
}
