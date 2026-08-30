// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `ttk::scrollbar` command (Tk 8.5+).
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::deferred_command_prefix_n(
            "prefix",
            AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&[2, 3])),
        ),
        detail: "Command prefix invoked with moveto fraction or scroll number units|pages when the scrollbar is moved.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-orient",
        value: OptionValue::enumerated(ORIENT, true, "orientation"),
        detail: "Orientation of the scrollbar: horizontal or vertical.",
        ..OptionSpec::DEFAULT
    },
];

const ORIENT: &[ArgValue] = &[
    ArgValue {
        value: "horizontal",
        detail: "Horizontal scrollbar.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "vertical",
        detail: "Vertical scrollbar.",
        ..ArgValue::DEFAULT
    },
];

super::common::ttk_widget_class!(
    SUBCOMMANDS,
    CLASS,
    "ttk::scrollbar",
    SubCommand {
        name: "get",
        arity: Arity::exact(0),
        detail: "Return the scrollbar's current first and last fractions.",
        synopsis: "pathName get",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(2),
        detail: "Set the scrollbar's first and last fractions.",
        synopsis: "pathName set first last",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
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
);

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ttk::scrollbar pathName ?options?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::scrollbar",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed scrollbar widget.",
            synopsis: &["ttk::scrollbar pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_scrollbar.n",
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
