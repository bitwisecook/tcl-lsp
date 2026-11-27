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

//! Original readonly layouts for genuinely installed called statics.

use super::{CommandRegistry, ModuleCommandBindings, SharedSourceInventory, SourceCommandBindings};

impl SourceCommandBindings {
    pub(super) fn retain_called_static_declaration_layouts(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        if !self.deferred.values().any(|root| {
            root.statics.is_some()
                && self
                    .called_implementations
                    .contains(&root.implementation_id())
        }) {
            return;
        }
        let entries = self
            .deferred_entry_states(incoming)
            .into_iter()
            .filter(|(root, _)| {
                self.called_implementations
                    .contains(&root.implementation_id())
                    && !root.receiver_method
                    && root.event.is_none()
                    && root.future_frame.is_none()
                    && root.statics.is_some()
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return;
        }
        // The same genuine body/frame producer supplies only readonly layouts.
        // Its results and all entered inventories remain in this private clone.
        let mut conditional = self.clone();
        conditional.conditional_procedure_results = SharedSourceInventory::default();
        conditional.active_calls.clear();
        conditional.active_variable_observers.clear();
        for (root, entry) in entries {
            conditional.walk_deferred_entry(&root, &entry, config, registry);
        }
        self.merge_original_declaration_layouts(&conditional);
    }
}
