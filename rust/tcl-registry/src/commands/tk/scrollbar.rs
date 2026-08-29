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

//! `scrollbar` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-orient",
        value: OptionValue::value(""),
        detail: "Orientation of the scrollbar: horizontal or vertical.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_command_prefix_n(
            "prefix",
            AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&[2, 3])),
        ),
        detail: "Command prefix to invoke when the scrollbar is moved (`moveto frac` appends 2, `scroll n units|pages` appends 3).",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value(""),
        detail: "Desired narrow dimension of the scrollbar in screen units.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-bg",
        value: OptionValue::value(""),
        detail: "Shorthand for -background.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-background",
        value: OptionValue::value(""),
        detail: "Background colour of the scrollbar.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-activebackground",
        value: OptionValue::value(""),
        detail: "Background colour when the mouse is over the scrollbar elements.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-troughcolor",
        value: OptionValue::value(""),
        detail: "Colour of the trough area behind the slider.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-borderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around the scrollbar.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-elementborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the borders around the internal elements of the scrollbar.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-jump",
        value: OptionValue::value(""),
        detail: "Whether to delay updates until the mouse button is released.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-activerelief",
        value: OptionValue::value(""),
        detail: "Relief to use for the active element of the scrollbar.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the scrollbar.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the scrollbar accepts focus during keyboard traversal.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the scrollbar does not have focus.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the scrollbar has focus.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the scrollbar.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-repeatdelay",
        value: OptionValue::value(""),
        detail: "Milliseconds before auto-repeat begins when an arrow is held.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-repeatinterval",
        value: OptionValue::value(""),
        detail: "Milliseconds between auto-repeat invocations.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "scrollbar pathName ?option value ...?",
    ..FormSpec::DEFAULT
}];

static SCROLLBAR_METHODS: &[SubCommand] = &[
    SubCommand {
        name: "activate",
        arity: Arity::new(0, 1),
        detail: "Return the active element, or mark the named element active.",
        synopsis: "pathName activate ?element?",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    super::common::CLASSIC_WIDGET_CGET,
    super::common::CLASSIC_WIDGET_CONFIGURE,
    SubCommand {
        name: "delta",
        arity: Arity::exact(2),
        detail: "Return the fraction corresponding to a pixel displacement.",
        synopsis: "pathName delta deltaX deltaY",
        pure: true,
        return_type: Some(TclType::Double),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "fraction",
        arity: Arity::exact(2),
        detail: "Return the fraction corresponding to a point in the trough.",
        synopsis: "pathName fraction x y",
        pure: true,
        return_type: Some(TclType::Double),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::exact(0),
        detail: "Return the scrollbar's current view fractions.",
        synopsis: "pathName get",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "identify",
        arity: Arity::exact(2),
        detail: "Return the scrollbar element at the given coordinates.",
        synopsis: "pathName identify x y",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(2),
        detail: "Set the scrollbar's first and last view fractions.",
        synopsis: "pathName set first last",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
];

static SCROLLBAR_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "scrollbar",
    instance_methods: SCROLLBAR_METHODS,
    superclasses: &[],
    allow_unknown_methods: false,
    method_prefix_matching: PrefixMatching::Enabled,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "scrollbar",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a scrollbar widget.",
            synopsis: &["scrollbar pathName ?option value ...?"],
            snippet: "Displays a scrollbar and allows the user to control the viewing area of an associated widget.",
            source: "Tk man page scrollbar.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        object_class: Some(&SCROLLBAR_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
