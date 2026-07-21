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

//! `smtp::sendmessage` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "smtp::sendmessage token ?options?",
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-servers",
        value: OptionValue::value("list"),
        detail: "List of SMTP servers to try, in order.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-ports",
        value: OptionValue::value("list"),
        detail: "List of ports matching -servers.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-username",
        value: OptionValue::value("user"),
        detail: "Username for SMTP authentication.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-password",
        value: OptionValue::value("pass"),
        detail: "Password for SMTP authentication.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-usetls",
        value: OptionValue::value("bool"),
        detail: "Whether to attempt STARTTLS negotiation.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-tlspolicy",
        // smtp.tcl 766-768: `eval $options(-tlspolicy) [list $code]
        // [list $diagnostic]` — each value is separately list-wrapped, so eval
        // appends exactly two words: the SMTP response code and the diagnostic
        // text (kept one word by its own list-wrap).
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix consulted before STARTTLS; invoked with the SMTP \
response code and diagnostic text, and must return secure / insecure.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-originator",
        value: OptionValue::value("addr"),
        detail: "Originator address override.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-recipients",
        value: OptionValue::value("list"),
        detail: "Explicit recipient list override.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-header",
        value: OptionValue::value("{key value}"),
        detail: "A single header as a {key value} pair; may be repeated.",
        ..OptionSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "smtp::sendmessage",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Send an e-mail message via SMTP.",
            synopsis: &[
                "smtp::sendmessage token ?-servers list? ?-ports list? ?-username user? ?-password pass? ?-usetls bool? ?-tlspolicy cmd? ?-originator addr? ?-recipients list? ?-header {key value} ...?",
            ],
            snippet: "",
            source: "tcllib smtp package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("smtp"),
        required_package: Some("smtp"),
        ..CommandSpec::DEFAULT
    }
}
