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

//! Variable reads: the `$` and name-role reads of one command and of the substitutions and expressions inside it.

use super::super::state::Analyser;
use super::super::types::ScopeKind;
use super::{ancestor_paths, scope_at, scope_at_mut};
use tcl_lexer::{Span, Token};

impl Analyser {
    /// Walk every `Var` token in a segmented command and call
    /// [`Self::record_var_read`] for each.  This is how the
    /// analyser tracks `$x` substitutions in arg positions
    /// (`puts $x`, `string length $name`, etc.) — without it,
    /// `VarDef.references` would carry only the registry `VarRead`-role
    /// name words (`set x`, `info exists x`) collected at its end.
    ///
    /// The shared configured reference scanner verifies the
    /// original token extent and owns the full form, including literal
    /// brace-quoted names and separate array-index syntax.
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
        // Each read carries whether it used the `${…}` brace form, whose
        // content is a *literal* name: `${$n}` reads the variable called `$n`,
        // not `n`.
        let mut reads: Vec<(String, tcl_lexer::Span, bool)> = Vec::new();
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
            let Some((name, braced)) = crate::var_refs::original_variable_reference_form(
                &self.source,
                *tok,
                self.file_lexer_config(),
            ) else {
                continue;
            };
            reads.push((name.to_string(), tok.span, braced));
        }
        for (name, span, braced) in reads {
            self.record_var_read_braced_in_origin(&name, span, scope_path, braced, true);
        }

        // `$var` reads also occur inside command substitutions
        // (`[…]`) and braced `expr` / condition arguments, which the
        // main body walk treats as opaque tokens.  Collect those
        // here too so `VarDef.references` is complete for the
        // references / document-highlight providers and the
        // minifier's rename pass.  `record_var_read` no-ops on names
        // that aren't in the current scope, so liberal collection is
        // safe.
        if let Some(registry) = self.registry.as_deref() {
            let mut extra: Vec<VarReadSite> = Vec::new();
            let lexer_config = self.file_lexer_config();
            let expr_idx: std::collections::HashSet<usize> =
                crate::registry_invocation::source_structure::source_registry_words(
                    &self.source,
                    &self.result,
                    cmd,
                )
                .map_or_default(|selected| selected.written_argument_roles())
                .into_iter()
                .filter_map(|(index, role)| (role == tcl_registry::ArgRole::Expr).then_some(index))
                .collect();
            // Top-level command-substitution tokens.
            for tok in &cmd.all_tokens {
                if tok.kind == tcl_lexer::TokenType::Cmd {
                    collect_cmd_subst_reads(
                        &self.source,
                        &self.result,
                        *tok,
                        registry,
                        lexer_config,
                        &mut extra,
                    );
                }
            }
            // Braced expr arguments.
            for (i, arg) in cmd.argv.iter().enumerate().skip(1) {
                if arg.kind == tcl_lexer::TokenType::Str && expr_idx.contains(&(i - 1)) {
                    collect_expr_reads(
                        &self.source,
                        &self.result,
                        *arg,
                        registry,
                        lexer_config,
                        &mut extra,
                    );
                }
            }
            // Registry `VarRead`-role name words of this command itself
            // (`info exists m`, `array get m`) — a use site of `m` exactly as
            // `$m` is, and one the `$`-token walk above cannot see because
            // there is no `$` to see.
            collect_name_role_reads(&self.source, &self.result, cmd, &mut extra);
            for site in extra {
                self.record_var_read_braced_in_origin(
                    &site.name,
                    site.span,
                    scope_path,
                    site.braced_literal,
                    site.substitution,
                );
            }
        }
    }
}

impl Analyser {
    /// Record a variable read for go-to-definition / find-references.
    ///
    /// Looks for the variable in the scope at `scope_path`; falls
    /// back to the global scope for ``::``- and ``static::``-prefixed
    /// names, and — under a Tcl 8.x dialect (TIP 278) — for a bare
    /// name read at a namespace frame whose namespace has no such
    /// variable but the global namespace does. W210 (read-before-set)
    /// is emitted by the SSA-based pass elsewhere — this helper only
    /// records the reference span.
    pub fn record_var_read(&mut self, name: &str, read_span: Span, scope_path: &[usize]) {
        self.record_var_read_braced(name, read_span, scope_path, false);
    }

    /// [`Self::record_var_read`] for a read whose delimiters make its content
    /// a **literal** name — a `${…}` reference or a brace-quoted word in a
    /// variable-name role (`[set {$n}]`, `info exists {$n}`).
    ///
    /// Braces suppress every substitution, so the content is the name verbatim:
    /// `${$n}` reads the variable *called* `$n`, which Tcl keeps distinct from
    /// `n` (tclsh 9.0.4 / 8.6.14: `set {$n} v; set ${$n}` → `can't read "v"`,
    /// i.e. it read `$n` and got `v`). Stripping the `$` here would land the
    /// reference on the wrong variable and let Find-References / Rename merge
    /// the two.
    pub fn record_var_read_braced(
        &mut self,
        name: &str,
        read_span: Span,
        scope_path: &[usize],
        braced_literal: bool,
    ) {
        self.record_var_read_braced_in_origin(name, read_span, scope_path, braced_literal, false);
    }

    fn record_var_read_braced_in_origin(
        &mut self,
        name: &str,
        read_span: Span,
        scope_path: &[usize],
        braced_literal: bool,
        substitution: bool,
    ) {
        let base_name = crate::naming::normalise_var_name_braced(name, braced_literal);
        self.record_qualified_var_ref(base_name, read_span, scope_path, substitution, false);
        // An `$arr(idx)` read records the element index on the array var.
        let element = crate::naming::split_array_name_braced(name, braced_literal)
            .1
            .map(ToString::to_string);

        if self.result.allows_lexical_declaration_advice() {
            let image = tcl_lexer::SourceImage::document(&self.source);
            let selected_path = self
                .result
                .logical_variable_scope_advice(&image, self.file_lexer_config(), read_span.start())
                .and_then(|advice| {
                    let selected = advice.variable_declaration_scope(base_name)?;
                    ancestor_paths(scope_path).find(|path| {
                        scope_at(&self.result.global_scope, path)
                            .is_some_and(|scope| std::ptr::eq(scope, selected))
                    })
                });
            if let Some(path) = selected_path
                && let Some(var) = scope_at_mut(&mut self.result.global_scope, &path)
                    .and_then(|scope| scope.variables.get_mut(base_name))
            {
                var.push_reference(read_span);
                if let Some(element) = element {
                    var.array_indices.insert(element);
                }
            }
            return;
        }

        // Local scope first.
        let path = scope_path.to_vec();
        let base_owned = base_name.to_string();
        if let Some(scope) = scope_at_mut(&mut self.result.global_scope, &path)
            && let Some(var) = scope.variables.get_mut(&base_owned)
        {
            var.push_reference(read_span);
            if let Some(e) = element {
                var.array_indices.insert(e);
            }
            return;
        }

        // Cross-rule variables — fall back to global scope.
        if tcl_syntax::naming::is_absolute(&base_owned)
            || tcl_syntax::naming::authored_namespace_hint_tail(&base_owned, "static").is_some()
        {
            if let Some(var) = self.result.global_scope.variables.get_mut(&base_owned) {
                var.push_reference(read_span);
                if let Some(e) = element {
                    var.array_indices.insert(e);
                }
            } else if let Some(captured) = self.capture_global_reads.as_mut() {
                captured.push(super::super::per_item::CapturedBodyGlobalRead {
                    base_name: base_owned,
                    element,
                    span: read_span,
                });
            }
        } else if !tcl_syntax::naming::is_qualified(base_owned.as_bytes())
            && self.result.ns_var_global_fallback()
            && scope_at(&self.result.global_scope, &path)
                .is_some_and(|s| s.kind == ScopeKind::Namespace)
            && let Some(var) = self.result.global_scope.variables.get_mut(&base_owned)
        {
            // TIP 278 — at a **namespace frame**, Tcl 8.x resolves a bare
            // undefined name against the global namespace (never an
            // intermediate namespace; 9.0 dropped even the global hop), so
            // attach the read to the global cell for find-references /
            // rename.  A namespace-local declaration (`variable v`) was
            // caught by the local-scope branch above, so reaching here means
            // the namespace table genuinely lacks the name — exactly the C
            // fallback condition (8.6 `tclVar.c` `TclLookupSimpleVar`).
            var.push_reference(read_span);
            if let Some(e) = element {
                var.array_indices.insert(e);
            }
        }
    }

    /// The variable names defined so far in the scope at `scope_path` —
    /// the walk-time in-scope candidate set for the W212 / W215
    /// "did you mean…?" suggestions.  Owned strings so callers can keep
    /// the list across later scope mutations.
    #[must_use]
    pub(in crate::analyser) fn scope_variable_names(&self, scope_path: &[usize]) -> Vec<String> {
        scope_at(&self.result.global_scope, scope_path)
            .map_or_default(|s| s.variables.keys().cloned().collect())
    }
}

// Substituted / expr `$var` read collection

/// Inner content text + base offset of a wrapper token (`[…]` /
/// `{…}`): the delimiter is normally a single leading byte, skipped via
/// `content_offset` — not hardcoded, since a synthetic recovery token
/// (e.g. `Analyser::recover_stray_close_bracket`'s virtual `Cmd` token)
/// has no real opener in the source and sets `content_offset` to `0`.
/// The span excludes the closing delimiter.
pub(in super::super) fn inner_of(source: &str, tok: Token) -> Option<(&str, u32)> {
    let off = u32::from(tok.content_offset);
    let (s, e) = ((tok.span.start() + off) as usize, tok.span.end() as usize);
    if s > e || e > source.len() {
        return None;
    }
    Some((&source[s..e], tok.span.start() + off))
}

/// One collected read site: the variable word as written, where it is, and
/// whether its braces make the content a literal name (`[set {$n}]` reads the
/// cell called `$n`, not `n`).
pub(in super::super) struct VarReadSite {
    pub name: String,
    pub span: Span,
    pub braced_literal: bool,
    substitution: bool,
}

impl VarReadSite {
    /// A read written as an ordinary `$`-substitution or expression term,
    /// whose text is already the bare name.
    pub(super) fn substituted(name: &str, span: Span, braced_literal: bool) -> Self {
        Self {
            name: name.to_owned(),
            span,
            braced_literal,
            substitution: true,
        }
    }
}

/// Record the variable-name words of `cmd` that the registry marks as
/// **reads** — the `m` of `info exists m` / `array get m` / `[set m]`, and
/// the literal `$n` of `[set {$n}]`.
///
/// Original source roles from [`crate::var_refs::original_variable_name_role_words`], so no
/// command is named here: any spec declaring an [`ArgRole::VarRead`] position
/// contributes its use sites to `VarDef::references`, which is what
/// Find References / document-highlight / the minifier's rename pass consume.
/// Write roles are excluded — a write is a *declaration* site, recorded by
/// the declaring handler, not a reference.
///
/// [`ArgRole::VarRead`]: tcl_registry::ArgRole::VarRead
fn collect_name_role_reads(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    cmd: &crate::segmenter::SegmentedCommand,
    out: &mut Vec<VarReadSite>,
) {
    for w in crate::var_refs::original_variable_name_role_words(source, analysis, cmd) {
        if w.role != tcl_registry::ArgRole::VarRead {
            continue;
        }
        out.push(VarReadSite {
            name: w.word,
            span: w.span,
            braced_literal: w.braced_literal,
            substitution: false,
        });
    }
}

/// Collect `$var` reads inside a command-substitution token,
/// recursing into nested substitutions, expr arguments, and
/// same-scope (plain) body arguments.  Structural-body arguments
/// (proc / namespace / oo) introduce a new scope and are skipped.
fn collect_cmd_subst_reads(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    cmd_tok: Token,
    registry: &tcl_registry::CommandRegistry,
    lexer_config: tcl_lexer::LexerConfig,
    out: &mut Vec<VarReadSite>,
) {
    let Some((inner, base)) = inner_of(source, cmd_tok) else {
        return;
    };
    for cmd in crate::segmenter::segment_commands_with_offset_and_config(
        inner,
        base,
        lexer_config.nested(),
    ) {
        collect_script_command_reads(source, analysis, &cmd, registry, lexer_config, out);
    }
}

/// Record reads for one command appearing inside a substitution:
/// top-level `$var` tokens, plus recursion into nested
/// substitutions / expr args / plain bodies.
pub(super) fn collect_script_command_reads(
    source: &str,
    analysis: &crate::analyser::AnalysisResult,
    cmd: &crate::segmenter::SegmentedCommand,
    registry: &tcl_registry::CommandRegistry,
    lexer_config: tcl_lexer::LexerConfig,
    out: &mut Vec<VarReadSite>,
) {
    let head_span = cmd.argv.first().map(|t| t.span);
    for tok in &cmd.all_tokens {
        if tok.kind == tcl_lexer::TokenType::Var && Some(tok.span) != head_span {
            if let Some((name, braced)) =
                crate::var_refs::original_variable_reference_form(source, *tok, lexer_config)
            {
                out.push(VarReadSite::substituted(name, tok.span, braced));
            }
        } else if tok.kind == tcl_lexer::TokenType::Cmd {
            collect_cmd_subst_reads(source, analysis, *tok, registry, lexer_config, out);
        }
    }
    // The substituted command's own `VarRead`-role name words — `puts [set
    // m]` and `puts [info exists m]` read `m`, which nothing else here sees.
    collect_name_role_reads(source, analysis, cmd, out);
    let selected =
        crate::registry_invocation::source_structure::source_registry_words(source, analysis, cmd);
    let roles = selected.as_ref().map_or_default(
        crate::registry_invocation::source_structure::OriginalRegistryWords::written_argument_roles,
    );
    let expr_idx: std::collections::HashSet<_> = roles
        .iter()
        .filter_map(|(index, role)| (*role == tcl_registry::ArgRole::Expr).then_some(*index))
        .collect();
    let body_idx: std::collections::HashSet<_> = roles
        .iter()
        .filter_map(|(index, role)| (*role == tcl_registry::ArgRole::Body).then_some(*index))
        .collect();
    let plain_body = analysis.resolved_input.as_ref().and_then(|input| {
        selected
            .as_ref()?
            .with_source_schema(&input.context_registry(), |invocation| {
                invocation.semantics.body_kind != tcl_registry::BodyKind::Structural
            })
    }) == Some(true);
    for (i, arg) in cmd.argv.iter().enumerate().skip(1) {
        if arg.kind != tcl_lexer::TokenType::Str {
            continue;
        }
        let pidx = i - 1;
        if expr_idx.contains(&pidx) {
            collect_expr_reads(source, analysis, *arg, registry, lexer_config, out);
        } else if body_idx.contains(&pidx) && plain_body {
            // Plain body inside a substitution — same scope, and the
            // main walk never reached it.
            if let Some((inner, base)) = inner_of(source, *arg) {
                for sub in crate::segmenter::segment_commands_with_offset_and_config(
                    inner,
                    base,
                    lexer_config.nested(),
                ) {
                    collect_script_command_reads(
                        source,
                        analysis,
                        &sub,
                        registry,
                        lexer_config,
                        out,
                    );
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
    analysis: &crate::analyser::AnalysisResult,
    expr_tok: Token,
    registry: &tcl_registry::CommandRegistry,
    lexer_config: tcl_lexer::LexerConfig,
    out: &mut Vec<VarReadSite>,
) {
    let Some((inner, base)) = inner_of(source, expr_tok) else {
        return;
    };
    let Ok(tokens) = tcl_lexer::Lexer::with_config(inner, lexer_config.nested()).tokenise_all()
    else {
        return;
    };
    for tok in tokens {
        let abs = Span::new(tok.span.start() + base, tok.span.end() + base);
        match tok.kind {
            tcl_lexer::TokenType::Var => {
                if let Some((name, braced)) = crate::var_refs::original_variable_reference_form(
                    source,
                    Token { span: abs, ..tok },
                    lexer_config,
                ) {
                    out.push(VarReadSite::substituted(name, abs, braced));
                }
            }
            tcl_lexer::TokenType::Cmd => {
                collect_cmd_subst_reads(
                    source,
                    analysis,
                    Token::new(tcl_lexer::TokenType::Cmd, abs),
                    registry,
                    lexer_config,
                    out,
                );
            }
            _ => {}
        }
    }
}
