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

//! `XLAT::listen` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::listen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates a related ephemeral listener.",
            synopsis: &[
                "XLAT::listen (-hairpin)? (-inherit-main-rules)? (-single-connection)? (-translation-loose)? (XLAT_LISTEN_SUBCMDS)+",
            ],
            snippet: "Creates a related ephemeral listener and returns the TCL handle for the listener. bind address and port can be omitted. It is recommend that users don't set this, so the command can choose an IP:port based on the server address specified and also conforms to source translation config. If the server address is on the clientside, then bind IP::port will be a valid endpoint on the clientside and conforms to the source translation config on the clientside.",
            source: "https://clouddocs.f5.com/api/irules/XLAT__listen.html",
            examples: "when SERVER_CONNECTED {\n    set listen [XLAT::listen -inherit-main-rules 30 {\n        proto [IP::protocol]\n        bind -allow [LINK::vlan_id],/Common/public1 -ip [serverside {IP::local_addr}]\n        server [IP::client_addr] 7000\n        allow [LB::server addr] 0\n        inherit-vs [virtual]\n    }]\n    log local0. \"LISTEN: $listen\"\n\n    # hairpin\n    set listen_hairpin [XLAT::listen -hairpin 30 {\n        proto [IP::protocol]\n        bind -allow [clientside {LINK::vlan_id}]",
            return_value: "Return the TCL handle to the created listener. String representaion of the handle: \"<local addr>%<local route domain id>,<local port>,<remote addr>%<remote route domain id>,<remote port>,<server addr>%<server route domain id>,<server port>,<vlan id>,<protocol number>\".",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_DATA", "SERVER_CONNECTED", "SERVER_DATA"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "XLAT::listen (-hairpin)? (-inherit-main-rules)? (-single-connection)? (-translation-loose)? (XLAT_LISTEN_SUBCMDS)+",
            dialects: None,
        }],
        options: const {
            &[
                OptionSpec {
                    name: "-hairpin",
                    value: OptionValue::flag(),
                    detail: "Enable hairpin mode for the listener.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-inherit-main-rules",
                    value: OptionValue::flag(),
                    detail: "Execute main rules attached to parent virtual.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-single-connection",
                    value: OptionValue::flag(),
                    detail: "Listener expires after one connection.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-translation-loose",
                    value: OptionValue::flag(),
                    detail: "Use hint data as suggestion; don't fail if unusable.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::LsnState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
