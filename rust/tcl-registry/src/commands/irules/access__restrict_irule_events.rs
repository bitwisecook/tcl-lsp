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

//! `ACCESS::restrict_irule_events` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::restrict_irule_events",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable or disable HTTP and higher layer iRule events for the internal APM access control URIs.",
            synopsis: &["ACCESS::restrict_irule_events (enable | disable)"],
            snippet: "During access policy execution, ACCESS creates requests to various URIs\nrelated to various access policy processing. These includes /my.policy\nand other pages (logon, message box etc.) shown to the end user. By\ndefault from 11.0.0 onward, HTTP and higher layer iRule events are not\nraised for the internal access control URIs. All events except\nACCESS_SESSION_STARTED, ACCESS_SESSION_CLOSED,\nACCESS_POLICY_AGENT_EVENT, ACCESS_POLICY_COMPLETED are blocked (not\nraised) for internal access control URI.\nThis command allows admin to overwrite the default behavior.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__restrict_irule_events.html",
            examples: "when CLIENT_ACCEPTED {\n    ACCESS::restrict_irule_events disable\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::restrict_irule_events (enable | disable)",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
