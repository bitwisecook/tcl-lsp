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

//! Readonly channel callback protocols selected from original operand roles.

use super::ResolvedInvocation;
use crate::channel_protocols::ChannelCallbackProtocol;

impl ResolvedInvocation<'_, '_> {
    /// Protocol of one original command-prefix operand in effective post-head
    /// coordinates. Unknown layouts and unclassified/reference-only positions
    /// abstain. This supplies no handler, native entry or future dispatch proof.
    #[must_use]
    pub fn authored_source_callback_protocol(
        &self,
        argument: usize,
    ) -> Option<ChannelCallbackProtocol> {
        if !self
            .authored_source_command_prefix_arguments()?
            .iter()
            .any(|&(index, _)| index == argument)
        {
            return None;
        }
        let descriptors = self.authored_source_descriptors();
        let protocols = descriptors
            .subcommand
            .map_or(descriptors.command.callback_protocols, |subcommand| {
                subcommand.callback_protocols
            });
        if let Some(protocol) = protocols.iter().find_map(|&(index, protocol)| {
            (usize::from(index) + self.semantics.argument_offset == argument).then_some(protocol)
        }) {
            return Some(protocol);
        }
        self.authored_source_diagnostic_options()?
            .options
            .into_iter()
            .find_map(|option| {
                if !option.available || !option.values?.contains(&argument) {
                    return None;
                }
                match option.option.value {
                    crate::hover::OptionValue::Takes(value) => value.callback_protocol,
                    crate::hover::OptionValue::Flag => None,
                }
            })
    }
}
