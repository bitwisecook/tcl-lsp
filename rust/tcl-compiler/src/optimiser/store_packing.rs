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

//! Shared proof boundary for grouping variable stores into another invocation.
//! Completion, output address order, observers and object sharing are separate
//! obligations. Literal spellings alone never close the replacement proof.

use crate::ir::{Script, StatementResultUse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StorePackingTarget {
    Lassign,
    Foreach,
}

impl StorePackingTarget {
    fn command(self) -> &'static str {
        match self {
            Self::Lassign => "lassign",
            Self::Foreach => "foreach",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StorePackingDecline {
    EnclosingResult,
    UnknownResultUse,
    UnprovedOutputSchedule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StorePackingAssessment {
    pub target: StorePackingTarget,
    pub decline: StorePackingDecline,
}

/// Resolve prospective command slots from the original immutable interpreter
/// point. This does not infer engine identity from an editor/catalogue label.
pub(crate) fn assess_grouped_store_rewrite(
    script: &Script,
    first: usize,
    last: usize,
) -> Option<StorePackingAssessment> {
    let statement = script.statements.get(first)?;
    let tokens = script.retained_source_tokens_for_statement(statement)?;
    let source = tokens.source_binding.as_ref()?;
    let dialect = source.variable_context.invocation_dialect?;
    if dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
        return None;
    }
    let target = match dialect.tcl_version? {
        tcl_dialect::TclVersion::V8_4 => StorePackingTarget::Foreach,
        tcl_dialect::TclVersion::V8_5 | tcl_dialect::TclVersion::V8_6 => {
            StorePackingTarget::Lassign
        }
        _ => return None,
    };
    for command in [target.command()]
        .into_iter()
        .chain((target == StorePackingTarget::Foreach).then_some("break"))
    {
        let resolved = source.lookup_command_word(command);
        let [binding] = resolved.targets.as_slice() else {
            return None;
        };
        if resolved.unknown
            || resolved.may_be_absent
            || !binding.registry_backed
            || binding.command != format!("::{command}")
            || !binding.prepended.is_empty()
        {
            return None;
        }
    }
    let decline = match script.statement_result_use(last) {
        StatementResultUse::EnclosingCompletion => StorePackingDecline::EnclosingResult,
        StatementResultUse::Unknown => StorePackingDecline::UnknownResultUse,
        // A known result-discard boundary does not prove ordered variable
        // addresses, native failure text, or shared object identities. Current
        // metadata does not close that grouped-store equivalence contract.
        StatementResultUse::Discarded => StorePackingDecline::UnprovedOutputSchedule,
    };
    Some(StorePackingAssessment { target, decline })
}
