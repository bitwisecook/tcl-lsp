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

//! Which declared hook bodies run: the trust half of the loader–host seam,
//! which needs no host.
//!
//! Built without `programmed-packs` this is the whole of [`crate::hooks`]:
//! no body gets a registry slot, so every hook keeps the loader's abstaining
//! placeholder, as a dormant one does.

use std::path::PathBuf;

use tcl_dialect::model::Provenance;
#[cfg(not(feature = "programmed-packs"))]
use tcl_registry::spec::CommandSpec;

use crate::loader::{HookDecl, HookSource, PackCommand};
#[cfg(not(feature = "programmed-packs"))]
use crate::pack::PackSet;

/// Every hook **body** `commands` declare, beside the command it hangs off:
/// what the host would run. A `-native ID` names engine code the pack cannot
/// supply and a derivation keyword is the loader's own business, so neither
/// is one. The host adapter and [`dormant_hooks`] both read this, so the hooks
/// a trusted workspace runs are exactly the ones an untrusted one reports.
pub(super) fn bodies(commands: &[PackCommand]) -> impl Iterator<Item = (&PackCommand, &HookDecl)> {
    commands.iter().flat_map(|command| {
        command
            .hooks
            .iter()
            .filter(|hook| matches!(hook.source, HookSource::Body { .. }))
            .map(move |hook| (command, hook))
    })
}

// Execution gated on trust

/// Whether a pack hook body of `provenance` runs — the execution half of the
/// trust ruling (`docs/design/compiler/registry-consumer-contracts.md`
/// § *Ruling — trust gates execution, not authority*).
///
/// Only a workspace the editor has not marked trusted holds its bodies
/// dormant: a body runs per query on words the analysed document supplies,
/// so it is the one pack surface whose inputs an untrusted workspace would
/// choose. A Spec Studio override is untrusted for registration (it may not
/// `-override` a compiled name) but runs — it is the author's own live edit,
/// and its bodies answering is what the studio shows. Authority reads none
/// of this: a dormant pack's declarative facts reach the registry unchanged.
#[must_use]
pub fn hook_bodies_run(provenance: Provenance) -> bool {
    provenance != Provenance::WorkspaceUntrusted
}

/// A declared hook body that does not run, because its pack's provenance
/// holds it dormant ([`hook_bodies_run`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DormantHook {
    /// The pack that declares it.
    pub pack: String,
    /// The command it hangs off.
    pub command: String,
    /// The field it would fill (`const_fold`, `options.arity_hook`,
    /// `state_transitions.resolver`, `evaluate`, …).
    pub field: &'static str,
    /// The file that declares it — the merge's record on the command, empty
    /// for a pack evaluated from a source with no path.
    pub file: PathBuf,
    /// The line of the row that declares it ([`HookDecl::line`]).
    pub line: u32,
    /// The source subject of that row, when the loader issued one.
    pub row: Option<tcl_lsp_core::code_actions::SpecPackNoticeSubject>,
}

/// The hook bodies `commands` declare that stay dormant for a pack named
/// `pack` of `provenance`: every one when its bodies do not run, none
/// otherwise. What the load reports ([`crate::pack::PackNotice::dormant`])
/// and what the hook plan lists as dormant.
#[must_use]
pub fn dormant_hooks(
    pack: &str,
    commands: &[PackCommand],
    provenance: Provenance,
) -> Vec<DormantHook> {
    if hook_bodies_run(provenance) {
        return Vec::new();
    }
    bodies(commands)
        .map(|(command, hook)| DormantHook {
            pack: pack.to_owned(),
            command: command.spec.name.to_owned(),
            field: hook.field,
            file: command.file.clone(),
            line: hook.line,
            row: hook.row.clone(),
        })
        .collect()
}

/// The slot assignment a build without the host makes: none.
#[cfg(not(feature = "programmed-packs"))]
#[derive(Debug, Default)]
pub(crate) struct HookPlan;

#[cfg(not(feature = "programmed-packs"))]
pub(crate) fn plan_for(_packs: &PackSet) -> std::sync::Arc<HookPlan> {
    std::sync::Arc::new(HookPlan)
}

/// `command`'s spec as the loader built it: with no slot, nothing to bind.
#[cfg(not(feature = "programmed-packs"))]
pub(crate) fn specialise(command: &PackCommand, _plan: &HookPlan) -> &'static CommandSpec {
    command.spec
}
