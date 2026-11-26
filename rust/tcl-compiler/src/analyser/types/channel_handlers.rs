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

//! Channel handler prefixes: which channel protocols call a procedure, and
//! which handler word an offset falls on.

use super::AnalysisResult;
use tcl_registry::channel_protocols::ChannelCallbackProtocol;

/// One handler prefix naming a procedure, with the words written in the
/// prefix before the protocol's appended words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelHandlerUse {
    /// Protocol the host follows when it calls the prefix.
    pub protocol: ChannelCallbackProtocol,
    /// Words written in the prefix after its head.
    pub baked: usize,
}

impl AnalysisResult {
    /// Handler prefixes that name the procedure `qualified`, for advice such
    /// as the unused-parameter hint. A prefix describes the call shape the
    /// host would use; it proves no installed handler.
    #[must_use]
    pub fn channel_handler_uses(&self, qualified: &str) -> Vec<ChannelHandlerUse> {
        let mut uses = Vec::new();
        for row in &self.command_invocations {
            let Some(prefix) = row.original_callback_prefix.as_deref() else {
                continue;
            };
            let Some(protocol) = prefix.channel_protocol() else {
                continue;
            };
            let selected = row
                .original_callback_signature_lookup
                .as_deref()
                .and_then(|lookup| lookup.original().target())
                .and_then(|target| target.source_procedure(self))
                .map(|procedure| procedure.metadata().qualified_name.as_str());
            // An unresolved head is matched as the global command it names
            // when looked up from the global namespace. This only withholds
            // a hint, so a handler the host finds elsewhere costs nothing.
            let names = match (row.resolved_qualified_name.as_deref(), selected) {
                (Some(resolved), _) | (None, Some(resolved)) => resolved == qualified,
                (None, None) => tcl_syntax::naming::qualify("::", &row.name) == qualified,
            };
            let handler = ChannelHandlerUse {
                protocol,
                baked: prefix.baked_argument_count(),
            };
            if names && !uses.contains(&handler) {
                uses.push(handler);
            }
        }
        uses
    }

    /// The handler prefix whose head word covers `offset`.
    #[must_use]
    pub fn channel_handler_at(&self, offset: u32) -> Option<ChannelHandlerUse> {
        self.command_invocations.iter().find_map(|row| {
            let prefix = row.original_callback_prefix.as_deref()?;
            let protocol = prefix.channel_protocol()?;
            (row.range.start() <= offset && offset <= row.range.end()).then_some(
                ChannelHandlerUse {
                    protocol,
                    baked: prefix.baked_argument_count(),
                },
            )
        })
    }
}
