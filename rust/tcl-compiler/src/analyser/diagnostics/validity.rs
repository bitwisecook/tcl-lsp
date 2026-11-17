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

//! Command validity and arity checks emitted during the command walk.
//!
//! These diagnostics decide whether a command invocation is well-formed
//! against the registry and the active dialect: an unknown subcommand
//! (W001), a command disabled in the dialect (W002), an invalid dialect
//! option (W004) or expression operator (W003), wrong argument counts
//! (the arity diagnostics), a malformed `if` (E004), a missing `--` option
//! terminator before a value that looks like an option (W304), an `unset`
//! whose options consume every argument so nothing is unset (W217), and a stub
//! `proc` that shadows a built-in command or `expr` function (W116, W117).
//! The disabled-command, arity, and W304 emitters buffer their candidates
//! and flush them after the walk.

use rustc_hash::{FxHashMap, FxHashSet};
#[cfg(test)]
use tcl_core_types::DiagCode;

use crate::analyser::state::Analyser;

mod arity;
mod command_availability;
mod discouraged_commands;
mod expr_operators;
mod malformed_structure;
mod option_relations;
mod option_words;
mod stub_shadows;

pub(super) use arity::{arity_verdict, shift_arity, source_arity_verdict};
pub(super) use command_availability::dialect_availability_suffix;
pub(in crate::analyser) use discouraged_commands::PendingPrivateTclSourceAdvice;
#[cfg(test)]
pub(super) use expr_operators::contains_gated_word;
pub(in crate::analyser) use malformed_structure::{
    emit_invalid_formal_parameter_list_diagnostics, emit_invalid_lambda_parameter_list_diagnostics,
    emit_invalid_static_variable_list_diagnostics,
};
pub(in crate::analyser) use option_relations::{
    ScannedInvocation, SeenOption, SeenPositional, option_relation_diagnostics,
};
#[cfg(test)]
use stub_shadows::{is_builtin_expr_op, is_builtin_math_function, is_irules_only_expr_op};

/// Reporting candidates plus the **implicit `namespace path`** in force at
/// `call_off` — today only a `TclOO` member body's `::oo::Helpers`, and that
/// name comes from the registry
/// ([`tcl_registry::definer::TCLOO_MEMBER_BODY_NAMESPACE_PATH`], read through
/// [`crate::analyser::scope::implicit_command_namespace_path_at`]), never from
/// a literal here.
///
/// Without the path a bare word written in a method body only ever qualifies
/// to `::WORD`, so a document that installs its own
/// `proc ::oo::Helpers::callback` — the documented "`TclOO` Tricks" wiki
/// helper `ticklecharts` uses under 8.6 — would not count as shadowing the
/// like-named 9.0 builtin, and the call would draw a W002 "disabled in the
/// active dialect profile" the program provably never sees. tclsh 8.6.14 runs
/// that file: the helper *is* the command the method body reaches.
fn qualify_candidates_with_path(ns: &str, cmd_name: &str, path: &[&str]) -> Vec<String> {
    crate::naming::command_resolution_candidates(ns, path, cmd_name)
}

/// Deferred builtin/private-advice suppression over the merged file.
/// Actual user-call signature selection uses its retained original call owner.
/// Whole-command exclusions use their own typed original publication geometry;
/// this compatibility projection supplies no Native naming or handler proof.
///
/// Owns its data (rather than borrowing `Analyser`) so a flush loop can
/// build it once up front and then freely take/mutate other `Analyser`
/// fields (`pending_arity`, `result.diagnostics`, …) while iterating —
/// borrowing through a whole-`&Analyser` build call would otherwise pin the
/// borrow checker's view to "all of `self`" for the facts' lifetime.
pub(super) struct UserResolutionFacts {
    /// Fully-qualified names that unconditionally denote a user command once
    /// declared anywhere in the file: `TclOO`/snit/itcl classes, `interp
    /// alias` names, and `namespace ensemble create` namespaces. Not
    /// order-gated — a deliberately permissive, false-negative-avoiding
    /// convention (a genuinely order-violating call is rare enough that
    /// abstaining from a real builtin-mismatch report is the safer trade).
    non_proc_qnames: FxHashSet<String>,
    /// Qualified proc name → the offset of its *earliest* declaration —
    /// the point from which the name denotes a user command at all. A name
    /// declared twice keeps the first offset here (`all_procs` alone only
    /// remembers the second), so a call between the two still suppresses the
    /// builtin-mismatch diagnostic it shadows.
    proc_offsets: FxHashMap<String, u32>,
    /// Qualified *new* name (a static `rename OLD NEW`) → the `rename`
    /// statement's token offset. `rename` moves an existing command's
    /// identity onto `NEW`, so once in effect `NEW` is a real command
    /// exactly like a proc definition — order-gated the same way.
    rename_offsets: FxHashMap<String, u32>,
    /// Document-global `# tcl-lsp: stub` command names (unqualified).
    stub_names: std::collections::HashSet<String>,
}

impl UserResolutionFacts {
    pub(super) fn build(a: &Analyser) -> Self {
        let mut non_proc_qnames: FxHashSet<String> = FxHashSet::default();
        non_proc_qnames.extend(a.result.all_classes.keys().cloned());
        non_proc_qnames.extend(a.result.command_aliases.keys().cloned());
        non_proc_qnames.extend(a.ensemble_namespaces.iter().cloned());
        let proc_offsets: FxHashMap<String, u32> = a
            .result
            .all_procs
            .keys()
            .map(|qname| {
                let earliest = a
                    .result
                    .proc_declarations(qname)
                    .map(|def| def.name_span.start())
                    .min()
                    .unwrap_or(u32::MAX);
                (qname.clone(), earliest)
            })
            .collect();
        let rename_offsets: FxHashMap<String, u32> = a
            .result
            .renamed_commands
            .keys()
            .filter_map(|new_qname| {
                a.rename_offsets
                    .get(new_qname)
                    .map(|&off| (new_qname.clone(), off))
            })
            .collect();
        let stub_names = super::utils::scan_stub_command_names(&a.source);
        Self {
            non_proc_qnames,
            proc_offsets,
            rename_offsets,
            stub_names,
        }
    }

    /// Whether a call to `cmd_name` (resolved at `ns`, through the implicit
    /// `namespace path` `path`, at byte offset `call_off`) resolves to a
    /// user-declared proc, class, alias, rename target, ensemble, or stub,
    /// rather than falling through to the registry builtin that produced the
    /// candidate diagnostic.
    ///
    /// `enforce_order` gates **proc** and **rename** shadowing to a
    /// definition that lexically precedes the call: true for a top-level
    /// call site (the module body, a `namespace eval` body, or a
    /// conditional), which executes in source order during load, so a
    /// same-named definition appearing later in the file has not run yet. A
    /// proc-body call (`enforce_order == false`) runs only after the whole
    /// file has loaded, so any same-named definition anywhere in the file is
    /// already in effect. Classes / aliases / ensembles / stubs always exist
    /// by run time and are never order-gated.
    fn resolves_to_user(
        &self,
        cmd_name: &str,
        ns: &str,
        path: &[&str],
        enforce_order: bool,
        call_off: u32,
    ) -> bool {
        let bare = core::str::from_utf8(tcl_syntax::naming::written_command_tail(
            cmd_name.as_bytes(),
        ))
        .expect("tail of a Unicode command");
        let candidates = qualify_candidates_with_path(ns, cmd_name, path);
        candidates.iter().any(|c| {
            self.non_proc_qnames.contains(c.as_str())
                || self
                    .proc_offsets
                    .get(c.as_str())
                    .is_some_and(|&off| !enforce_order || off < call_off)
                || self
                    .rename_offsets
                    .get(c.as_str())
                    .is_some_and(|&off| !enforce_order || off < call_off)
        }) || self.stub_names.contains(bare)
    }
}

/// Family advice retains the Native canonical definer and explicitly Logical
/// source model as separate admission paths. A report-map default alone grants
/// neither family classification nor current object dispatch.
pub(super) fn is_tcloo_source_class(analyser: &Analyser, class: &super::types::ClassDef) -> bool {
    if class.source_name_ambiguous.is_observed() {
        return false;
    }
    let analysis = &analyser.result;
    if analysis.allows_retained_logical_declaration_advice() {
        return analyser
            .retained_logical_class_definer_grammar(&class.qualified_name)
            .is_some_and(|grammar| grammar.family == tcl_registry::definer::DefinerFamily::TclOo);
    }
    let source = &analyser.source;
    let Some(publication) = class.source_name.as_ref() else {
        return false;
    };
    let mut records = analysis.original_class_declarations().filter(|record| {
        record.name() == publication && record.name_input().span() == class.name_span
    });
    let Some(record) = records.next() else {
        return false;
    };
    if records.next().is_some() {
        return false;
    }
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    crate::command_binding::OriginalSourceClassDeclaration::from_class(source, analysis, record)
        .and_then(|declaration| declaration.grammar(&input.context_registry()))
        .is_some_and(|grammar| grammar.family == tcl_registry::definer::DefinerFamily::TclOo)
}

#[cfg(test)]
mod w117_tests;

#[cfg(test)]
mod stub_name_geometry_tests;

#[cfg(test)]
mod selected_formal_tests;

#[cfg(test)]
mod logical_class_family_tests;

#[cfg(test)]
mod original_expression_assistance_tests;

#[cfg(test)]
mod configured_proposal_controls;

#[cfg(test)]
mod original_private_tcl_policy_controls;
