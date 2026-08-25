// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `ttk::labelframe` command (Tk 8.5+).
use crate::prelude::*;

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
        name: "-padding",
        value: OptionValue::value("padSpec"),
        detail: "Internal padding around the labelframe content.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-borderwidth",
        value: OptionValue::value("width"),
        detail: "Width of the labelframe border.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "Border relief style for the labelframe.",
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
        name: "-height",
        value: OptionValue::value("height"),
        detail: "Requested height of the labelframe content area.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-labelanchor",
        value: OptionValue::value("anchor"),
        detail: "Position of the label relative to the labelframe border.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-labelwidget",
        value: OptionValue::value("window"),
        detail: "Window to use as the labelframe label.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-text",
        value: OptionValue::value("text"),
        detail: "Text displayed as the labelframe label.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-underline",
        value: OptionValue::value("index"),
        detail: "Index of the underlined character in the label text.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("width"),
        detail: "Requested width of the labelframe content area.",
        ..OptionSpec::DEFAULT
    },
];

super::common::ttk_widget_class!(SUBCOMMANDS, CLASS, "ttk::labelframe",);

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ttk::labelframe pathName ?options?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::labelframe",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed labelframe container widget.",
            synopsis: &["ttk::labelframe pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_labelframe.n",
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
