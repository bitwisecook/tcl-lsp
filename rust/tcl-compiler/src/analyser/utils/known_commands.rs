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

//! The known-command universe recovery consults, and whether an earlier `proc` shadows a call.

use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

/// Build the "known command" name universe consulted by unclosed-delimiter
/// recovery: the segmenter's scan-to-next heuristic
/// ([`crate::segmenter::segment_commands_with_recovery_and_config`]) and the
/// E100/E201/E202/E203 recovery diagnostics (`syntax_checks`) both use "does
/// the next line start with a known command?" as their strongest signal that
/// a missing delimiter belongs right before it.
///
/// Restricting "known" to the active dialect's [`CommandRegistry`]
/// under-recognises real code: almost every non-trivial Tcl file calls its
/// own `proc`s, so a break just before a call to one of them falls back to
/// the imprecise generic diagnostic (or, worse, the recovery point is never
/// found at all and the rest of the document goes unanalysed). This unions
/// the registry names with every proc / class / command-alias /
/// rename-target name the document itself defines — qualified and
/// unqualified tail — using the same fast, non-lowering scan
/// [`crate::signature_scan::extract_signatures`] already uses for
/// background-indexed files, so recovery recognises a document's own procs
/// as readily as a builtin.
///
/// `extra` is unioned in unconditionally alongside the registry — the same
/// user-declared / LSP-layer-resolved name set [`super::super::state::Analyser::extra_commands`]
/// already carries for the W123 unresolved-command check (`tclLsp.extraCommands`,
/// widened one layer up in `tcl-lsp-server` with workspace- and package-resolved
/// names on the recovery path — see `widen_recovery_extra_commands` there). It is
/// a plain in-memory set by the time it reaches here, so folding it in costs
/// nothing extra beyond the registry union already does.
///
/// The in-file scan runs only when configured Document completeness proves
/// the source incomplete. Both queries share the supplied whole-file grammar;
/// an unavailable parser retains only registry and caller-supplied candidates.
#[must_use]
pub fn recovery_known_commands(
    source: &str,
    registry: &CommandRegistry,
    extra: &SharedNameSet,
    config: tcl_lexer::LexerConfig,
) -> RecoveryKnownCommands {
    let mut names: HashSet<String> = registry.command_names().map(str::to_owned).collect();
    if tcl_lexer::script_image_is_complete(&tcl_lexer::SourceImage::document(source), config)
        == Ok(false)
    {
        let sig = crate::signature_scan::extract_signatures_with_config(source, config, registry);
        for declaration in &sig.procedure_declarations {
            if let Some(spelling) = declaration.source_spelling() {
                insert_qualified_and_tail(&mut names, &spelling);
                names.insert(declaration.name.clone());
            }
        }
        for declaration in &sig.class_declarations {
            if let Some(spelling) = declaration.source_spelling() {
                insert_qualified_and_tail(&mut names, &spelling);
                names.insert(declaration.name.clone());
            }
        }
        for spelling in sig
            .command_aliases
            .values()
            .filter_map(
                super::super::super::signature_scan::types::SignatureCommandAlias::source_spelling,
            )
            .chain(sig.renames.values().filter_map(
                super::super::super::signature_scan::types::SignatureRename::source_spelling,
            ))
        {
            insert_qualified_and_tail(&mut names, &spelling);
        }
    }
    RecoveryKnownCommands {
        local: names,
        extra: std::sync::Arc::clone(extra),
    }
}

/// The recovery known-command universe [`recovery_known_commands`] builds,
/// held as **two layers** rather than one merged set.
///
/// `local` is derived from the document being analysed (the active registry's
/// names plus the document's own proc / class / alias / rename names), so it is
/// rebuilt on every edit. `extra` is the caller-supplied name set
/// ([`super::super::state::Analyser::extra_commands`]), which on the LSP's
/// unclosed-delimiter recovery path is the *widened* set — every
/// workspace-indexed proc and class, plus every auto-loadable command name:
/// tens of thousands of entries on a large workspace, and unchanged between
/// keystrokes. Merging the two would deep-copy that set on every analysis of a
/// document with an open delimiter — which is the *normal* mid-typing state, so
/// it would happen on every debounced run. Sharing it behind an `Arc` makes the
/// layering free.
///
/// Every consumer only ever asks "is this name known?" or enumerates the
/// universe once, both of which the two layers answer directly.
#[derive(Debug, Default, Clone)]
pub struct RecoveryKnownCommands {
    local: HashSet<String>,
    extra: SharedNameSet,
}

/// A command-name set shared between the analyser and its caller — see
/// [`RecoveryKnownCommands`] and [`super::super::state::Analyser::extra_commands`].
pub type SharedNameSet = std::sync::Arc<HashSet<String>>;

impl RecoveryKnownCommands {
    /// Whether `name` is a known command in either layer.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.local.contains(name) || self.extra.contains(name)
    }

    /// Every known name, `local` first. A name present in both layers is
    /// yielded twice; every consumer collects into a set, so that is harmless
    /// and avoids a de-duplication pass over the (large) shared layer.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.local
            .iter()
            .chain(self.extra.iter())
            .map(String::as_str)
    }

    /// Drop the per-document layer and release the shared one — the
    /// between-runs reset ([`super::super::state::Analyser::clear_run_state`]).
    pub fn clear(&mut self) {
        self.local.clear();
        self.extra = std::sync::Arc::default();
    }
}

impl FromIterator<String> for RecoveryKnownCommands {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Self {
            local: iter.into_iter().collect(),
            extra: std::sync::Arc::default(),
        }
    }
}

/// True when an earlier *unconditional* user `proc` named `qualified_name`
/// lexically precedes `call_start` — Tcl resolves a proc over a same-named
/// builtin/keyword at that call site, so the diagnostic that would fire
/// against the builtin/keyword must be suppressed. Shared by W002's inline
/// disabled-command shadow check and the EXPR-role emitters'
/// (W100/W110/W003/W114) control-flow-keyword shadow check — both are the
/// exact same position comparison against [`ProcDef::name_span`], just
/// triggered from different call sites. W002's *per-item* flush path needs
/// the fuller `UserResolutionFacts::resolves_to_user` instead (it also
/// covers aliases/renames/classes/ensembles and namespace-qualified procs),
/// so this simpler check isn't a fit there.
#[must_use]
pub fn proc_shadows_call<S: std::hash::BuildHasher>(
    all_procs: &HashMap<String, super::super::types::ProcDef, S>,
    qualified_name: &str,
    call_start: u32,
) -> bool {
    all_procs
        .get(qualified_name)
        .is_some_and(|def| def.name_span.start() < call_start)
}

/// Insert `qname` (e.g. `::ns::foo`) — as-is, with the leading `::`
/// stripped, and as its unqualified tail (`foo`) — into `names`. Mirrors the
/// tail-matching the W123 unresolved-command pass already uses for the same
/// proc/class/alias collections (`build_w123_known_names`), plus the
/// original fully-qualified form: `signature_scan::qualify` always prepends
/// `::`, and a recovery-point call can be written either way
/// (`::ns::foo` or `ns::foo`), so both must be recognised.
///
/// `pub` (not just `pub(in super::super)`) because `tcl-lsp-server` reuses this exact
/// three-form insertion when widening the recovery known-command set with
/// workspace-indexed and package-resolved names one layer up — those names
/// carry the same `::`-qualified shape and need the same recognition rules,
/// so this is the single place that logic lives rather than a second,
/// independently-maintained copy.
pub fn insert_qualified_and_tail<S: std::hash::BuildHasher>(
    names: &mut HashSet<String, S>,
    qname: &str,
) {
    if !qname.is_empty() {
        names.insert(qname.to_string());
    }
    let absolute = crate::naming::unroot_rooted_key(qname).unwrap_or(qname);
    if !absolute.is_empty() {
        names.insert(absolute.to_string());
    }
    let tail = crate::naming::key_tail(qname);
    if !tail.is_empty() {
        names.insert(tail.to_string());
    }
}
