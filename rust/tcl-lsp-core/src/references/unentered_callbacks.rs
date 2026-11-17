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

//! Callback selectors written in member bodies the analysis never entered.

use super::member_references::collect_member_bodies_scoped;
use super::self_dispatch_scan::scan_method_sites_by_kind;
use tcl_compiler::analyser::AnalysisResult;

/// Read-only callback selectors written in member bodies the analysis never
/// entered: the declaring class's bodies and those of subclasses whose source
/// order reaches the declaration without an override. `[self] NAME` captures
/// the external object command, so it needs an exported method; `my NAME`
/// does not. These spans are reports only: rename keeps them as the
/// [`super::unproved_callback_method_selector`] hazard.
pub(crate) fn unentered_callback_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    target: &crate::method_symbol::OriginalMethodCandidate,
) -> Vec<tcl_lexer::Span> {
    use tcl_compiler::analyser::class_hierarchy::original_metadata::original_instance_metadata_order_for_inventory;
    use tcl_compiler::analyser::types::MemberSide;
    use tcl_registry::definer::DefinitionMemberNamePurpose;

    let Some(class) = target.declaring_class_in(source, analysis) else {
        return Vec::new();
    };
    let method = target.method();
    let Ok(name) = std::str::from_utf8(method.original_name_input().bytes()) else {
        return Vec::new();
    };
    if method.name_purpose() != DefinitionMemberNamePurpose::TclOoMethod {
        return Vec::new();
    }
    let classmethod = method.side() == MemberSide::ClassObject;
    let records = analysis.original_class_declarations().collect::<Vec<_>>();
    let Some(own) = records
        .iter()
        .position(|record| std::ptr::eq(*record, class))
    else {
        return Vec::new();
    };
    let occupied = analysis
        .original_procedure_declarations()
        .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name)
        .collect::<Vec<_>>();
    let declares = |index: usize| {
        records[index]
            .metadata()
            .original_members
            .declarations()
            .any(|row| {
                row.side() == method.side()
                    && row.original_name_input().bytes() == method.original_name_input().bytes()
            })
    };
    let own_exported = records[own]
        .metadata()
        .original_members
        .methods(method.side())
        .and_then(|rows| {
            rows.into_iter()
                .find(|row| {
                    row.original_name_input().policy() == method.original_name_input().policy()
                        && row.original_name_input().bytes() == method.original_name_input().bytes()
                })
                .map(|row| row.exported())
        });
    let mut spans = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let order = if index == own {
            Some(vec![own])
        } else if classmethod {
            None
        } else {
            original_instance_metadata_order_for_inventory(&records, index, &occupied)
        };
        let Some(order) = order else {
            continue;
        };
        let Some(at) = order.iter().position(|&at| at == own) else {
            continue;
        };
        if order[..at].iter().any(|&earlier| declares(earlier)) {
            continue;
        }
        // The receiver's own export/unexport decides `[self] NAME`, folded
        // from the declaration down to the receiver class.
        let exported = order[..at]
            .iter()
            .rev()
            .try_fold(own_exported, |exported, &index| {
                let exported = exported?;
                records[index]
                    .metadata()
                    .original_members
                    .inherited_visibility_for_input(
                        method.side(),
                        method.original_name_input(),
                        exported,
                    )
                    .map(Some)
            })
            .flatten();
        let bodies = collect_member_bodies_scoped(record.metadata(), classmethod);
        spans.extend(scan_method_sites_by_kind(
            source,
            analysis,
            &bodies,
            name,
            None,
            exported == Some(true),
            true,
        ));
    }
    spans
}

/// Whether a list builder whose head is at `at` may name the current object's
/// method through the literal self-dispatch `command`: the builder sits in a
/// definition body, where that command is contextual, and no procedure,
/// alias, rename or load-level deletion the analysis recorded shares its
/// tail. Deliberately over-approximate: an abstention only.
pub(super) fn contextual_dispatch_head_is_unrebound(
    analysis: &AnalysisResult,
    command: &str,
    at: u32,
) -> bool {
    let tail = tcl_syntax::naming::written_command_tail(command.as_bytes());
    analysis.offset_is_inside_any_definition_body(at)
        && !analysis
            .all_procs
            .keys()
            .chain(analysis.command_aliases.keys())
            .chain(analysis.renamed_commands.keys())
            .chain(analysis.renamed_commands.values())
            .chain(analysis.destroyed_commands.keys())
            .any(|name| tcl_syntax::naming::written_command_tail(name.as_bytes()) == tail)
}
