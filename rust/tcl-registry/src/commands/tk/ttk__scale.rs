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

//! `ttk::scale` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-from",
        value: OptionValue::value("value"),
        detail: "Starting value of the scale range.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-to",
        value: OptionValue::value("value"),
        detail: "Ending value of the scale range.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-value",
        value: OptionValue::value("value"),
        detail: "Current value of the scale.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::user_input_var(),
        detail: "Variable linked to the scale value.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-orient",
        value: OptionValue::value("orientation"),
        detail: "Orientation of the scale (horizontal or vertical).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-length",
        value: OptionValue::value("length"),
        detail: "Length of the long axis of the scale widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(1)),
        detail: "Command prefix invoked when the scale value changes (the new value is appended).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal or disabled).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

super::common::ttk_widget_class!(
    SUBCOMMANDS,
    CLASS,
    "ttk::scale",
    SubCommand {
        name: "coords",
        arity: Arity::new(0, 1),
        detail: "Return the coordinates corresponding to a value.",
        synopsis: "pathName coords ?value?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        traits: Traits::TAINT_SOURCE_ZERO_ARGS,
        arity: Arity::new(0, 2),
        detail: "Return the current value or the value at coordinates x y.",
        synopsis: "pathName get ?x y?",
        pure: true,
        return_type: Some(TclType::Double),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(1),
        detail: "Set the scale value.",
        synopsis: "pathName set value",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
);

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ttk::scale pathName ?options?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::scale",
        traits: Traits::TAINTS_VAR_WRITES,
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed scale (slider) widget.",
            synopsis: &["ttk::scale pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_scale.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        object_class: Some(&CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
