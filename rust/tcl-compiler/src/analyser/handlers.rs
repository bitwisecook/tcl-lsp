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

//! Per-command handlers: the variable-mutation commands, plus `proc`,
//! `namespace eval`, the loop and branch forms, `catch`/`try`, `interp
//! alias` and `oo::objdefine`.
//!
//! The variable bindings:
//!
//! - [`Analyser::bind_value_word_assignment`] — a direct one-target write
//!   of a value word (`set var value`), read off the registry's `CellWrite`
//!   declaration
//! - [`Analyser::apply_state_transitions`] — every scope alias an
//!   invocation's state transitions state (`global`, `variable`, `upvar`,
//!   `namespace upvar`, a pack command's alias facts)
//! - [`Analyser::handle_var_binding_command`] — every loop and written
//!   variable a call's roles name (`foreach`, `dict for`, `incr`, `append`,
//!   `lassign`, …)
//!
//! Plus the remaining structural command handlers: proc-body
//! walking, `namespace eval`/`namespace ensemble`, `foreach`/`for`/
//! `switch`, `catch`/`try`, `interp alias`, `oo::objdefine`, and
//! alias resolution.

mod finite_foreach_source;

pub(super) use finite_foreach_source::FiniteForeachReportingState;
#[cfg(test)]
use tcl_core_types::DiagCode;
use tcl_lexer::{Span, Token, TokenType};

#[cfg(test)]
use crate::segmenter::SegmentedCommand;

use super::state::Analyser;
#[cfg(test)]
use super::types::{
    ClassDef, ClassFactory, FactoryMember, FactoryWord, MemberSide, MetaclassProvenance, ProcDef,
};

mod apply_lambda;
mod classes;
mod code_loading;
mod command_aliases;
mod control_flow;
mod definition_warnings;
mod ensembles;
mod interpreters;
mod namespaces;
mod opt_proc;
mod procedures;
mod regex_patterns;
mod scope_aliases;
mod symbol_definitions;
mod written_variables;

#[cfg(test)]
use classes::{
    ControlArm, ManufacturerLayout, ParameterisedCreation, ProloguePiece,
    join_parameterised_class_observations, original_class_definition_body_in_analysis,
    prologue_pieces, resolve_factory_member, substitute_bound_words,
};
pub(super) use interpreters::parse_interp_create_path;

/// Build a fully-qualified Tcl proc / class name from a namespace
/// prefix and a possibly-relative name.
///
/// `ns_prefix` is a **constructed** namespace key, rooted (`::a::b`, `::`)
/// or unrooted (`a::b`, empty = global); it is joined verbatim — only the
/// *written* `name`'s colon runs canonicalise.
pub(super) fn qualify(ns_prefix: &str, name: &str) -> String {
    crate::naming::qualify(ns_prefix, name)
}

/// The [`super::types::Scope`] name for a routine whose written name is
/// `resolved_name` (after [`Analyser::resolve_dynamic_word`]).
///
/// A `Proc` / `Method` scope's name is not decoration: it *is* the routine's
/// qualified name for
/// [`super::scope::advance_command_resolution_namespace`], which takes
/// everything before the last `::` as the namespace the body runs in.  So a
/// scope named with the raw, unresolved word invents a namespace out of the
/// substitution: the real tcllib idiom `proc [namespace current]::_x {...}`
/// inside `::pki` (tcllib 2.0 `modules/pki/pki.tcl`:316) made every definition
/// in the body home to `::pki::[namespace current]`.
///
/// When the name still carries a `$` / `[` after resolution there is no
/// trustworthy qualifier in it, so the scope keeps only the trailing segment —
/// whose holder is the *lexical* parent namespace, which is where the idiom
/// actually lands (tclsh 8.6.16 / 9.0.4: a `proc helper` inside
/// `proc [namespace current]::_inner` in `::pki` becomes `::pki::helper`).
pub(super) fn scope_name_for_routine(resolved_name: &str) -> &str {
    if crate::naming::is_dynamic_word(resolved_name) {
        crate::naming::key_tail(resolved_name)
    } else {
        resolved_name
    }
}

/// The first position `roles` assigns `role`.
fn role_position(
    roles: &[(usize, tcl_registry::ArgRole)],
    role: tcl_registry::ArgRole,
) -> Option<usize> {
    roles
        .iter()
        .find(|&&(_, assigned)| assigned == role)
        .map(|&(index, _)| index)
}

/// The words at every position `roles` assigns `role`, in position order.
fn role_texts(
    roles: &[(usize, tcl_registry::ArgRole)],
    role: tcl_registry::ArgRole,
    args: &[String],
) -> Vec<String> {
    roles
        .iter()
        .filter(|&&(_, assigned)| assigned == role)
        .filter_map(|&(index, _)| args.get(index).cloned())
        .collect()
}

impl Analyser {
    fn declaration_name_dialect(&self) -> tcl_registry::InvocationDialect {
        self.source_analysis_entry
            .as_ref()
            .and_then(|entry| entry.invocation_dialect)
            .unwrap_or_else(|| tcl_registry::InvocationDialect::of_profile(self.profile))
    }

    /// Independently selected Logical source advice. The authored declaration
    /// simulation can have a native-shaped naming policy without selecting a
    /// Native editing domain; actual entry and recipe remain separate barriers.
    pub(super) fn selected_logical_declaration_advice(&self) -> bool {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        crate::signature_scan::original_name::LogicalSourceNamePurpose::for_input(
            &self.resolved_analysis_input(),
            self.source_analysis_entry.as_deref(),
        )
        .is_some()
    }

    pub(super) fn declaration_name_policy(&self) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
        if self.vendor_source_name_policy().is_some()
            && self.source_analysis_entry.as_ref().is_none_or(|entry| {
                entry.native_entry.is_none()
                    && !matches!(
                        entry.execution_name_policy,
                        Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(_))
                    )
            })
        {
            // A hosted document supplied without an entry is a fresh rule
            // load (user decision 2026-10-21T00:10Z, "realm-aware command
            // world"): its loader defines each `proc` at load time under the
            // authored naming recipe of that entry.
            if self.source_analysis_entry.is_some() {
                return None;
            }
            return crate::command_binding::SourceAnalysisEntry::for_fresh_hosted_source(
                &self.resolved_analysis_input(),
            )
            .filter(|entry| !entry.unknown_entry)
            .and_then(|entry| match entry.execution_name_policy {
                Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(policy)) => Some(policy),
                _ => None,
            });
        }
        match self.source_analysis_entry.as_deref() {
            Some(entry) => match entry.native_entry.as_deref() {
                Some(native) => native.command_name_policy(),
                None => match entry.execution_name_policy {
                    Some(tcl_syntax::naming::ExecutionNamePolicy::ObservedBigIp(_)) => {
                        entry.invocation_dialect?.authored_name_policy()
                    }
                    Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(policy)) => {
                        Some(policy)
                    }
                    None => self.declaration_name_dialect().authored_name_policy(),
                },
            },
            None => self.declaration_name_dialect().authored_name_policy(),
        }
    }

    pub(super) fn declaration_namespace_scope(
        &self,
        path: &[usize],
    ) -> Option<crate::signature_scan::scope::SignatureNamespaceScope> {
        use crate::signature_scan::scope::SignatureNamespaceScope;
        let policy = self.declaration_name_policy()?;
        let mut current = match self
            .source_analysis_entry
            .as_deref()
            .and_then(|entry| entry.native_entry.as_deref())
        {
            Some(entry) => {
                let row = entry.namespace_context(entry.current_namespace).ok()?;
                match policy.recipe() {
                    tcl_syntax::naming::NativeNameProtocol::C(_) => {
                        SignatureNamespaceScope::C(row.path.clone())
                    }
                    tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                        SignatureNamespaceScope::Jim(row.jim_namespace_object.clone()?)
                    }
                }
            }
            None => SignatureNamespaceScope::root(Some(policy)),
        };
        let mut scope = &self.result.global_scope;
        for &index in path {
            scope = scope.children.get(index)?;
        }
        // Each receipt owns a complete namespace. Synthetic reporting wrappers
        // need not have their own recipe when the final lambda/declaration
        // scope independently retains one. An unknown final scope stays unknown.
        if !path.is_empty() {
            current = scope.naming_scope.clone()?;
        }
        Some(current)
    }

    /// Static words and dominating interpolations supply constant argument values.
    /// Command substitutions and unavailable variable values stay unresolved.
    /// Shared by any command whose argument names something else purely
    /// as data rather than computing it — [`Self::handle_rename`]'s
    /// `OLD`/`NEW` words, and [`Self::handle_source_command`]'s path word
    /// (`set p "e.tcl"; source $p`, the same shape `rename $old new`
    /// already resolves), the `oo::define`
    /// target word, and the command head itself
    /// ([`Self::resolve_dynamic_command_head`]).
    pub(super) fn resolve_dynamic_word(
        &self,
        text: &str,
        tok: Option<Token>,
        is_single: bool,
        _scope_path: &[usize],
    ) -> Option<String> {
        if !crate::naming::is_dynamic_word(text) {
            return Some(text.to_string());
        }
        let tok = tok?;
        if is_single && matches!(tok.kind, TokenType::Str | TokenType::Esc) {
            return Some(text.to_owned());
        }
        self.resolve_logical_value_word(tok)
            .or_else(|| self.frozen_value_word(tok))
    }

    /// Resolve a command name to the `ProcDef` that implements it, following
    /// Tcl's real bareword command resolution via
    /// [`crate::naming::bareword_resolution_candidates`] — the current
    /// namespace (computed by [`Self::command_resolution_namespace`]) first,
    /// then global, exactly two levels, never every enclosing ancestor
    /// namespace on the scope chain (Tcl's own command lookup does not walk
    /// intermediate namespaces absent an explicit `namespace path`).
    ///
    /// Shared with the optimiser's identical same-file resolution
    /// (`resolve_proc_qname`) and the disabled-command / arity suppression
    /// checks' resolution (`UserResolutionFacts::resolves_to_user`) so the
    /// three can't diverge on the same rule.
    ///
    /// Returns the first matching ``ProcDef`` (by reference into
    /// ``result.all_procs``), or `None` if no candidate is known.
    #[must_use]
    pub fn resolve_proc_call(
        &self,
        cmd_name: &str,
        scope_path: &[usize],
    ) -> Option<&super::types::ProcDef> {
        if cmd_name.is_empty() {
            return None;
        }
        let ns = self.command_resolution_namespace(scope_path);
        crate::naming::bareword_resolution_candidates(&ns, cmd_name)
            .into_iter()
            .find_map(|qname| self.result.all_procs.get(&qname))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_native_namespace_path_tests;

#[cfg(test)]
mod original_static_declaration_tests;

#[cfg(test)]
mod original_optlist_declaration_tests;

#[cfg(test)]
mod original_registry_symbol_tests;

#[cfg(test)]
mod original_parent_purpose_tests;

#[cfg(test)]
mod retained_static_context_tests;

#[cfg(test)]
mod constructed_projection_tests;

#[cfg(test)]
mod original_proc_source_layout_tests;

#[cfg(test)]
mod direct_original_class_body_tests;
