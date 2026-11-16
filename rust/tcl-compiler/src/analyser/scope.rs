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

//! Scope-graph helpers.
//!
//! Pure scope-tree mutation and traversal helpers used by the
//! analyser's command handlers. The scope tree uses a path-based
//! addressing scheme (``Vec<usize>`` indexing into
//! ``result.global_scope.children``) rather than parent
//! back-pointers, so any helper that needs to walk up the chain
//! accepts the path slice and traverses the result tree in place.
//! The cost — at most one descent from root per helper call — is
//! bounded by scope depth, which is shallow in practice.
//!
//! Helpers operate on ``&mut Analyser`` rather than free
//! functions because most need both the scope under cursor
//! (resolved from the path) and access to the analyser's
//! tracking maps (``const_strings``, ``regex_vars``,
//! ``result.regex_patterns``).

#[cfg(test)]
use tcl_core_types::DiagCode;
#[cfg(test)]
use tcl_lexer::{Span, Token};

use super::state::Analyser;
use super::types::Scope;
#[cfg(test)]
use super::types::{ScopeKind, VarDef};

mod logical_array_indices;
pub use logical_array_indices::{LogicalArrayIndexCandidate, LogicalArrayIndexKind};

mod command_namespaces;
mod constant_values;
mod invocation_resolution;
mod logical_scope_advice;
mod namespace_variables;
mod original_variable_occurrences;
mod variable_definitions;
mod variable_reads;

#[cfg(test)]
use command_namespaces::advance_command_resolution_namespace;
pub(crate) use command_namespaces::selected_jim_method_body_span_at;
pub use command_namespaces::{
    command_resolution_namespace_at, implicit_command_namespace_path_at,
    innermost_scope_is_oo_method_frame, innermost_scope_reaches_oo_helpers,
    original_namespace_scope_at,
};
pub(super) use constant_values::{LogicalConstStringProvenance, LogicalInterpreterPathProvenance};
pub use logical_scope_advice::{LogicalVariableScopeAdvice, ReadonlyDictionaryKeyVariable};
pub use namespace_variables::{
    VariableAliasLink, lookup_var_by_qualified_name, lookup_var_in_namespace, namespace_variables,
    qualified_name_for_var_decl, variable_alias_links,
};
pub(super) use variable_definitions::VariableLinkTarget;
#[cfg(test)]
use variable_reads::collect_script_command_reads;
pub(super) use variable_reads::inner_of;

/// Ancestor path enumerator: yields the active scope's path and
/// each of its proper ancestors back to the root, longest first.
///
/// `[2, 1, 0]` yields `[2, 1, 0]`, `[2, 1]`, `[2]`, `[]`. Used by
/// every helper that walks up the parent chain.
fn ancestor_paths(start: &[usize]) -> impl Iterator<Item = Vec<usize>> + '_ {
    (0..=start.len()).rev().map(|i| start[..i].to_vec())
}

/// Resolve a scope path to a `&Scope` reference inside `root`.
///
/// Walks `path` index-by-index. Returns the root for an empty
/// path; returns `None` if any index is out of bounds.
pub(super) fn scope_at<'a>(root: &'a Scope, path: &[usize]) -> Option<&'a Scope> {
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
}

impl Analyser {
    /// Walk the scope tree depth-first, yielding the scope at
    /// `scope_path` and every descendant in declaration order.
    ///
    /// Collects the paths into a `Vec` rather than returning a lazy
    /// iterator — simpler borrowing for the same iteration order.
    #[must_use]
    pub fn walk_scopes_from(&self, scope_path: &[usize]) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        let Some(start) = scope_at(&self.result.global_scope, scope_path) else {
            return out;
        };
        walk_scopes_helper(start, scope_path, &mut out, 0);
        out
    }
}

/// Depth cap for [`walk_scopes_helper`]'s recursion over nested
/// (namespace / proc / method) [`Scope`] children.
/// Transitively bounded today via `analyser::commands::MAX_BODY_DEPTH`
/// (the analyser's own recursive descent, which builds this `Scope` tree
/// in the first place, already caps its own nesting at 256), capped here
/// independently for defence-in-depth and consistency with every other
/// full-tree walker in this crate.
const MAX_SCOPE_WALK_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

fn walk_scopes_helper(scope: &Scope, path: &[usize], out: &mut Vec<Vec<usize>>, depth: u32) {
    if MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return;
    }
    out.push(path.to_vec());
    for (i, child) in scope.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(i);
        walk_scopes_helper(child, &child_path, out, depth + 1);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_positioned_reference_tests;

#[cfg(test)]
mod original_parent_purpose_tests;

#[cfg(test)]
mod original_reference_form_scope_tests;

#[cfg(test)]
mod logical_alias_occurrence_tests;

/// Source-scope relation to this analysis unit's original interpreter subject.
/// This is a readonly diagnostic boundary, not physical interpreter identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceInterpreterScopeRelation {
    /// Every retained enclosing scope stays in the analysis root's subject.
    OriginalRoot,
    /// An actual isolated child-interpreter body occurs in its ancestry.
    IsolatedBody,
}

impl super::types::AnalysisResult {
    /// Reborrow the actual source scope ancestry under its complete input.
    /// Names never encode interpreter identity here. Missing/foreign source,
    /// input or domain correspondence stays unavailable. An isolated body is
    /// a suppression boundary only, not proof that a child exists or executes.
    #[must_use]
    pub fn source_interpreter_scope_relation(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
        offset: u32,
    ) -> Option<SourceInterpreterScopeRelation> {
        let input = self.resolved_input.as_ref()?;
        if input.lexer_config() != config
            || self.body_lexer_config != Some(config)
            || !self.matches_original_source_image(image, config)
            || usize::try_from(offset).ok()? >= image.bytes().len()
            || !self
                .retained_command_realm()?
                .matches_resolved_analysis_input(input)
        {
            return None;
        }
        // Isolation is a source-body boundary even where ordinary scope
        // reporting is flattened. Inspect every actual containing body rather
        // than allowing a narrower sibling to hide an isolated parent.
        let mut pending = vec![&self.global_scope];
        while let Some(scope) = pending.pop() {
            pending.extend(&scope.children);
            if !std::ptr::eq(scope, &raw const self.global_scope)
                && !scope
                    .body_span
                    .is_some_and(|span| span.start() <= offset && offset < span.end())
            {
                continue;
            }
            if scope.is_independent_interpreter_source_body() {
                return Some(SourceInterpreterScopeRelation::IsolatedBody);
            }
            match (
                &self.global_scope.original_interpreter_source_domain,
                &scope.original_interpreter_source_domain,
            ) {
                (None, None) => {}
                (Some(root), Some(current)) if std::sync::Arc::ptr_eq(root, current) => {}
                _ => return None,
            }
        }
        Some(SourceInterpreterScopeRelation::OriginalRoot)
    }
}

#[cfg(test)]
mod original_interpreter_scope_relation_controls;
