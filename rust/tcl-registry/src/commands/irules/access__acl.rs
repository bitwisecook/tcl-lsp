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

//! `ACCESS::acl` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "result",
        arity: Arity::exact(0),
        detail: "Get ACL match result (Allow/Reject).",
        synopsis: "ACCESS::acl result",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "matched",
        arity: Arity::exact(0),
        detail: "Get matched ACL.",
        synopsis: "ACCESS::acl matched",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lookup",
        arity: Arity::exact(0),
        detail: "List assigned ACLs for the session.",
        synopsis: "ACCESS::acl lookup",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eval",
        arity: Arity::exact(1),
        detail: "Enforce an ACL by name.",
        synopsis: "ACCESS::acl eval <acl_name>",
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::acl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Poll or enforce ACLs in your connections.",
            synopsis: &["ACCESS::acl (result | matched | lookup | (eval ACL_NAME))"],
            snippet: "The ACCESS::acl commands allow you to poll, query or enforce ACLs for a\ngiven connection.\n\nACCESS::acl result\n\n     * Returns the result of ACL match for a particular URI in\n       ACCESS_ACL_ALLOWED and ACCESS_ACL_DENIED events.\n     * This result can have one of the following values\n     * - Allow\n     * - Reject\n\nACCESS::acl lookup\n\n     * Returns the name of all the assigned ACLs for a particular session.\n\nACCESS::acl eval $acl_name\n\n     * Allows admin to enforce an ACL to a user request from iRule.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__acl.html",
            examples: "when ACCESS_ACL_ALLOWED {\n      ACCESS::acl eval \"additional_acl\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::acl <subcommand> ?args?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
