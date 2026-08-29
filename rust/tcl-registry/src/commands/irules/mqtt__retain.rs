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

//! `MQTT::retain` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MQTT::retain",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set retain flag of MQTT PUBLISH message.",
            synopsis: &["MQTT::retain ('0' | '1')?"],
            snippet: "This command can be used to get or set retain flag of MQTT message.\nThis command is valid only for following MQTT message types:\n\n    PUBLISH",
            source: "https://clouddocs.f5.com/api/irules/MQTT__retain.html",
            examples: "# Convert PUBLISH for topics in a retain_datagroup to retain messages\nwhen MQTT_CLIENT_INGRESS {\n   set type [MQTT::type]\n   switch $type {\n      \"PUBLISH\" {\n          if {[MQTT::retain] eq 0} {\n              if { [class exists retain_datagroup] } {\n                  if {[class match [MQTT::topic] starts_with retain_datagroup]} {\n                     MQTT::retain 1\n                  }\n              }\n          }\n      }\n   }\n}",
            return_value: "When called without an argument, this command returns the retain flag of MQTT PUBLISH message.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MQTT"],
            also_in: &[],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "MQTT::retain ('0' | '1')?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
