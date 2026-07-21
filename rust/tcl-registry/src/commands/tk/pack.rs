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

//! `pack` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Set or query the packing options for one or more slaves.",
        synopsis: "pack configure slave ?slave ...? ?option value ...?",
        options: OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "content",
        arity: Arity::exact(1),
        detail: "Return a list of all slaves in the packing order for the master (9.0+ name for `slaves`).",
        synopsis: "pack content master",
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::at_least(1),
        detail: "Remove each slave from the packing order for its master.",
        synopsis: "pack forget slave ?slave ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(1),
        detail: "Return a list of the current configuration state of the slave.",
        synopsis: "pack info slave",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "propagate",
        arity: Arity::new(1, 2),
        detail: "Control whether the master computes its geometry from slaves.",
        synopsis: "pack propagate master ?boolean?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "slaves",
        arity: Arity::exact(1),
        detail: "Return a list of all slaves in the packing order for the master.",
        synopsis: "pack slaves master",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-side",
        value: OptionValue::value("top|bottom|left|right"),
        detail: "Specifies which side of the master the slave will be packed against.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-fill",
        value: OptionValue::value("none|x|y|both"),
        detail: "If a slave's parcel is larger than its requested dimensions, this option may be used to stretch the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-expand",
        value: OptionValue::value("boolean"),
        detail: "Specifies whether the slave should be expanded to consume extra space in its master.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-anchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "Anchor must be a valid anchor position: n, ne, e, se, s, sw, w, nw, or center.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-padx",
        value: OptionValue::value("amount"),
        detail: "Specifies how much external horizontal padding to leave on each side of the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-pady",
        value: OptionValue::value("amount"),
        detail: "Specifies how much external vertical padding to leave on each side of the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ipadx",
        value: OptionValue::value("amount"),
        detail: "Specifies how much internal horizontal padding to leave on each side of the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ipady",
        value: OptionValue::value("amount"),
        detail: "Specifies how much internal vertical padding to leave on each side of the slave.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-in",
        value: OptionValue::value("master"),
        detail: "Insert the slave at the end of the packing order for the master window.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-before",
        value: OptionValue::value("other"),
        detail: "Insert the slave before the window given by other in the packing order.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-after",
        value: OptionValue::value("other"),
        detail: "Insert the slave after the window given by other in the packing order.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "pack option arg ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pack",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Geometry manager that packs slaves around the edges of a cavity.",
            synopsis: &[
                "pack slave ?slave ...? ?option value ...?",
                "pack configure slave ?slave ...? ?option value ...?",
                "pack forget slave ?slave ...?",
                "pack info slave",
                "pack propagate master ?boolean?",
                "pack slaves master",
            ],
            snippet: "",
            source: "Tk man page pack.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
