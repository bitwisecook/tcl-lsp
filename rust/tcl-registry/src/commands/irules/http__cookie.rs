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

//! `HTTP::cookie` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return list of cookie names.",
        synopsis: "HTTP::cookie names",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "count",
        arity: Arity::exact(0),
        detail: "Return the number of cookies.",
        synopsis: "HTTP::cookie count",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "value",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie value.",
        synopsis: "HTTP::cookie value <name> ?string?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "version",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie version.",
        synopsis: "HTTP::cookie version <name> ?value?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "path",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie path.",
        synopsis: "HTTP::cookie path <name> ?value?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "domain",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie domain.",
        synopsis: "HTTP::cookie domain <name> ?value?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ports",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie ports.",
        synopsis: "HTTP::cookie ports <name> ?value?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::new(4, 10),
        detail: "Insert a new cookie.",
        synopsis: "HTTP::cookie insert name <name> value <value> ?path <path>? ?domain <domain>? ?version <0 | 1 | 2>?",
        mutator: true,
        credential_arg: Some(2),
        sensitive_headers: &[
            "authorization",
            "proxy-authorization",
            "x-api-key",
            "x-auth-token",
            "x-secret",
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::exact(1),
        detail: "Remove a cookie by name.",
        synopsis: "HTTP::cookie remove <name>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sanitize",
        arity: Arity::at_least(1),
        detail: "Remove all cookies except named ones.",
        synopsis: "HTTP::cookie sanitize <name-list>",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Check if a cookie exists.",
        synopsis: "HTTP::cookie exists <name>",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "maxage",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie max-age.",
        synopsis: "HTTP::cookie maxage <name> ?value?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "expires",
        arity: Arity::new(1, 3),
        detail: "Get/set cookie expires.",
        synopsis: "HTTP::cookie expires <name> ?seconds? ?absolute | relative?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "comment",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie comment.",
        synopsis: "HTTP::cookie comment <name> ?value?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "secure",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie secure flag.",
        synopsis: "HTTP::cookie secure <name> ?enable|disable?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "commenturl",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie comment URL.",
        synopsis: "HTTP::cookie commenturl <name> ?value?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encrypt",
        arity: Arity::new(2, 3),
        detail: "Encrypt a cookie value.",
        synopsis: "HTTP::cookie encrypt <name> <passphrase> ?128 | 192 | 256?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "decrypt",
        arity: Arity::new(2, 3),
        detail: "Decrypt a cookie value.",
        synopsis: "HTTP::cookie decrypt <name> <passphrase> ?128 | 192 | 256?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "httponly",
        arity: Arity::new(1, 2),
        detail: "Get/set cookie httponly flag.",
        synopsis: "HTTP::cookie httponly <name> ?enable|disable?",
        subcommand_forms: &[
            SubCommandForm {
                name: "getter",
                ..SubCommandForm::DEFAULT
            },
            SubCommandForm {
                name: "setter",
                ..SubCommandForm::DEFAULT
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "attribute",
        arity: Arity::new(1, 4),
        detail: "Get/set arbitrary cookie attribute.",
        synopsis: "HTTP::cookie attribute <name> ?insert <attr> ?value? | exists <attr> | value <attr> | remove <attr> | names | count?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::at_least(2),
        detail: "Replace cookie value.",
        synopsis: "HTTP::cookie replace <name> <value>",
        mutator: true,
        credential_arg: Some(2),
        sensitive_headers: &[
            "authorization",
            "proxy-authorization",
            "x-api-key",
            "x-auth-token",
            "x-secret",
        ],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::cookie",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::DIAGRAM_ACTION),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Queries for or manipulates cookies in HTTP requests and responses.",
            synopsis: &["HTTP::cookie <subcommand> ?arg ...?"],
            snippet: "Queries for or manipulates cookies in HTTP requests and responses. This command replaces the BIG-IP 4.X variable http_cookie.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__cookie.html",
            examples: "# Or just for one statically defined cookie:\nwhen HTTP_RESPONSE {\n   HTTP::cookie version myCookie 1\n   HTTP::cookie httponly myCookie enable\n}",
            return_value: "",
        }),
        // `HTTP::cookie insert|replace` with tainted data →
        // header injection (IRULE3002).
        // (`credential_arg` + `sensitive_headers` now live on the
        // `insert` / `replace` SubCommand specs above.)
        taint_output_sink: Some("IRULE3002"),
        taint_output_sink_subcommands: &["insert", "replace"],
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            synopsis: "HTTP::cookie <subcommand> ?arg ...?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpCookie,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
