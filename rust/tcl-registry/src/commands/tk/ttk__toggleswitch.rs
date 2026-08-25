// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `ttk::toggleswitch` command (Tk 9.1+).
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const SWITCHSTATE_FORMS: &[SubCommandForm] = &[
    SubCommandForm {
        name: "query",
        arity: Arity::exact(0),
        traits: Some(Traits::PURE.union(Traits::TAINT_SOURCE_ZERO_ARGS)),
        mutator: Some(false),
        side_effects: Some(super::common::TTK_WIDGET_READS),
        ..SubCommandForm::DEFAULT
    },
    SubCommandForm {
        name: "set",
        arity: Arity::exact(1),
        traits: Some(Traits::EVALUATES_CODE),
        mutator: Some(true),
        side_effects: Some(super::common::TTK_CALLBACK_EFFECTS),
        ..SubCommandForm::DEFAULT
    },
];

const SIZE: &[ArgValue] = &[
    ArgValue {
        value: "1",
        detail: "Small switch.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "2",
        detail: "Medium switch.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "3",
        detail: "Large switch.",
        ..ArgValue::DEFAULT
    },
];

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
        value: OptionValue::deferred_script(),
        detail: "Script evaluated at global scope when the switch state toggles.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-offvalue",
        value: OptionValue::value("value"),
        detail: "Value written to -variable when the switch is off.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-onvalue",
        value: OptionValue::value("value"),
        detail: "Value written to -variable when the switch is on.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-size",
        value: OptionValue::enumerated(SIZE, true, "size"),
        detail: "Visual switch size: 1, 2, or 3.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::user_input_var(),
        detail: "Global variable linked to the switch state.",
        ..OptionSpec::DEFAULT
    },
];

super::common::ttk_widget_class!(
    SUBCOMMANDS,
    CLASS,
    "ttk::toggleswitch",
    style_since = "9.1",
    SubCommand {
        name: "switchstate",
        arity: Arity::new(0, 1),
        detail: "Return the current 0/1 switch state, or set it and invoke -command when it changes.",
        synopsis: "pathName switchstate ?boolean?",
        return_type: Some(TclType::String),
        subcommand_forms: SWITCHSTATE_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "toggle",
        arity: Arity::exact(0),
        detail: "Toggle the switch state and invoke its command.",
        synopsis: "pathName toggle",
        traits: Traits::EVALUATES_CODE,
        mutator: true,
        side_effects: super::common::TTK_CALLBACK_EFFECTS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        // `get` with no selector is the current user-controlled slider
        // value. `get min`, `get max`, and `get x` expose deterministic
        // internal bounds/coordinate conversion and must stay clean.
        traits: Traits::TAINT_SOURCE_ZERO_ARGS,
        arity: Arity::new(0, 1),
        detail: "Return the current, minimum, maximum, or x-coordinate-derived internal slider value.",
        synopsis: "pathName get ?min|max|x?",
        pure: true,
        return_type: Some(TclType::Double),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(1),
        detail: "Set the switch value.",
        synopsis: "pathName set value",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xcoord",
        arity: Arity::new(0, 1),
        detail: "Return the x coordinate for the current or supplied internal slider value.",
        synopsis: "pathName xcoord ?value?",
        traits: Traits::TAINT_SOURCE_ZERO_ARGS,
        pure: true,
        return_type: Some(TclType::Double),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
);

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ttk::toggleswitch pathName ?options?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::toggleswitch",
        traits: Traits::TAINTS_VAR_WRITES,
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed toggle switch widget.",
            synopsis: &["ttk::toggleswitch pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_toggleswitch.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("9.1"),
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
