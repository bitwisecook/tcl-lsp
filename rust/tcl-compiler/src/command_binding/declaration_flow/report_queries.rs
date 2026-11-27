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

//! The questions a finished declaration-flow report answers for diagnostics.

use super::{DeclarationFlowFrame, DeclarationFlowReport};
use std::collections::BTreeMap;
use tcl_names::command::CommandAllocationSite;

pub(super) fn original_procedure_entry_matches(
    frame: &super::super::declaration_layout::OriginalDiagnosticFrameEntry,
    procedure: &crate::ir::Procedure,
    image: &tcl_lexer::SourceImage,
) -> bool {
    let super::super::declaration_layout::OriginalDiagnosticFrameEntry::Body(entry) = frame else {
        return false;
    };
    entry.allocation().site.offset == procedure.span.start()
        && matches!(entry.source().origin.kind(), tcl_names::origin::SourceOriginKind::Authored(source) if source == image)
        && entry.source().base() == procedure.body_offset
        && entry.source().try_text().ok() == procedure.body_source.as_deref()
        && entry.matches_parameters(
            &procedure
                .params
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )
}

impl DeclarationFlowReport {
    /// Validate an authored declaration independently from an entered script.
    /// This retains diagnostic frame ownership and grants no body execution.
    pub(crate) fn owns_original_procedure(
        &self,
        procedure: &crate::ir::Procedure,
        image: &tcl_lexer::SourceImage,
    ) -> bool {
        match &self.frame {
            DeclarationFlowFrame::Existing(frame) => {
                original_procedure_entry_matches(frame, procedure, image)
            }
            DeclarationFlowFrame::Read(entry) => entry.owns_original_procedure(procedure, image),
        }
    }

    pub(crate) fn owns_invocation(&self, site: &CommandAllocationSite) -> bool {
        self.source == site.source && self.represented.contains(&site.offset)
    }

    #[cfg(test)]
    pub(crate) fn potential_missing_reads(&self) -> &BTreeMap<String, tcl_lexer::Span> {
        &self.reads
    }

    /// Original local references under the declared frame interpretation.
    /// Unlike physical read advice, these occurrences carry no current cell,
    /// successful read, SSA version or closed-world absence. Unknown writers
    /// remain possible; a diagnostic must describe that condition explicitly.
    #[cfg(test)]
    pub(crate) fn potential_declared_reads(&self) -> &BTreeMap<String, tcl_lexer::Span> {
        &self.authored_reads
    }

    /// An original missing-read occurrence whose authored dispatch prefix is
    /// described. Unknown handler dispatch can initialise arbitrary locals, so
    /// its successors remain in the occurrence inventory but cannot supply an
    /// absence warning. Runtime callback alternatives remain independent.
    pub(crate) fn allows_declared_absence_warning(
        &self,
        name: &str,
        span: tcl_lexer::Span,
    ) -> bool {
        self.warning_reads.get(name) == Some(&span)
    }

    /// Missing-read candidates on an independently available original prefix.
    /// Unknown later iterations and runtime alternatives remain in the other
    /// graphs; this grants only the explicitly conditional local-frame warning.
    pub(crate) fn declared_absence_warning_occurrences(
        &self,
    ) -> &BTreeMap<String, tcl_lexer::Span> {
        &self.warning_reads
    }

    /// Adjacent literal stores under an available declared local-frame prefix.
    /// These occurrences grant no actual reachability, cell or editable store.
    pub(crate) fn conditional_overwrite_occurrences(
        &self,
    ) -> impl Iterator<
        Item = (
            &CommandAllocationSite,
            &CommandAllocationSite,
            &str,
            tcl_lexer::Span,
        ),
    > {
        self.conditional_overwrites
            .iter()
            .map(|store| (&store.first, &store.next, store.name.as_str(), store.target))
    }

    /// A reference defined on the declared diagnostic paths reaching that
    /// exact original occurrence. First loop entry is distinct from backedges;
    /// this condition never removes a runtime zero-trip or callback alternative.
    pub(crate) fn declared_read_is_defined(&self, name: &str, span: tcl_lexer::Span) -> bool {
        self.declared_defined_reads
            .get(name)
            .is_some_and(|spans| spans.contains(&span))
    }

    /// Original uses in reachable declared syntax. This can suppress a warning,
    /// but cannot keep a physical version live or grant a runtime read.
    pub(crate) fn has_authored_use(&self, name: &str) -> bool {
        self.authored_uses.contains_key(name)
    }

    pub(crate) fn authored_use_spans(&self, name: &str) -> &[tcl_lexer::Span] {
        self.authored_uses.get(name).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn has_named_use(&self, name: &str) -> bool {
        self.named_uses.contains(name)
    }

    /// Literal data mentions only suppress unused-store diagnostics. They are
    /// not reads, variable versions, liveness facts or executable source.
    pub(crate) fn has_quoted_use(&self, name: &str) -> bool {
        self.quoted_use_unavailable || self.quoted_uses.contains(name)
    }

    pub(crate) fn has_declared_alias(&self, name: &str) -> bool {
        self.aliases.contains(name)
    }

    /// Original declaration-local value immediately before the selected
    /// handler, after its original operands. Unknown writers and aliases
    /// withdraw this advice; no physical contents or SSA value is supplied.
    pub(crate) fn conditional_handler_literal(
        &self,
        site: &CommandAllocationSite,
        name: &str,
    ) -> Option<&str> {
        (site.source == self.source)
            .then(|| {
                self.conditional_handler_literals
                    .get(&site.offset)?
                    .get(name)
                    .map(String::as_str)
            })
            .flatten()
    }

    /// An original formal still carries its incoming value at this declared
    /// argument entry, before this command's operands and dispatch. The caller
    /// must independently close intervening operand effects. Unknown earlier
    /// writers and joins withdraw this source-only provenance; this command's
    /// unavailable handler cannot erase its own available argument prefix.
    /// This grants neither an actual read nor a caller value.
    pub(crate) fn conditional_argument_keeps_incoming(
        &self,
        site: &CommandAllocationSite,
        name: &str,
    ) -> bool {
        site.source == self.source
            && self
                .conditional_argument_incoming
                .get(&site.offset)
                .is_some_and(|names| names.contains(name))
    }

    /// Every original alias in this available declaration prefix selects the
    /// immediate caller's frame. This conditional name-layout fact does not
    /// establish a successful alias, actual caller address or receiver entry.
    pub(crate) fn conditional_alias_prefix_targets_caller(
        &self,
        site: &CommandAllocationSite,
    ) -> bool {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        self.source == site.source
            && self
                .conditional_caller_alias_prefixes
                .contains(&site.offset)
    }

    /// Original argument entry under the accepted declared-frame layouts.
    /// Unknown runtime dispatch and writers remain possible. This purpose
    /// supplies candidate diagnostics only, never successful entry or SSA facts.
    pub(crate) fn declared_argument_entry(&self, site: &CommandAllocationSite) -> bool {
        self.source == site.source && self.declared_entries.contains(&site.offset)
    }

    /// Conditional argument-entry reachability at this original invocation.
    /// An unavailable predecessor returns `None`. This command's subsequent
    /// dispatch does not withdraw its own entry, but does withdraw successors.
    /// Absence never means proved dead runtime code.
    pub(crate) fn invocation_may_be_reached(&self, site: &CommandAllocationSite) -> Option<bool> {
        if self.source != site.source || !self.represented.contains(&site.offset) {
            return None;
        }
        if self.reached.contains(&site.offset) {
            return (!self.unavailable_before.contains(&site.offset)).then_some(true);
        }
        (!self.unavailable).then_some(false)
    }

    /// Conditional absence of an original local read in the complete native
    /// declaration layout. Runtime callbacks and observation remain open.
    pub(crate) fn local_store_may_be_unread(
        &self,
        site: &CommandAllocationSite,
        name: &str,
    ) -> bool {
        !self.unavailable
            && !self.unknown_reads
            && self.invocation_may_be_reached(site) == Some(true)
            && self.stores.contains(&(site.offset, name.to_owned()))
            && !self.all_reads.contains(name)
            && !self.has_authored_use(name)
            && !self.aliases.contains(name)
    }

    #[cfg(test)]
    pub(crate) fn unread_gate_summary(&self, site: &CommandAllocationSite, name: &str) -> String {
        format!(
            "unknown_paths={} unknown_reads={} entry={:?} store={} read={} alias={}",
            self.unavailable,
            self.unknown_reads,
            self.invocation_may_be_reached(site),
            self.stores.contains(&(site.offset, name.to_owned())),
            self.all_reads.contains(name),
            self.aliases.contains(name)
        )
    }
}
