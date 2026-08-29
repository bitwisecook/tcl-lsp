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

//! `ttk::notebook` command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-width",
        value: OptionValue::value("width"),
        detail: "Desired width of the notebook.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("height"),
        detail: "Desired height of the notebook.",
        surface: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-padding",
        value: OptionValue::value("padSpec"),
        detail: "Internal padding around the notebook content.",
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

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "ttk::notebook pathName ?options?",
    ..FormSpec::DEFAULT
}];

const TAB_STATES: &[ArgValue] = &[
    ArgValue {
        value: "normal",
        detail: "The tab is selectable and displayed normally.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "disabled",
        detail: "The tab is displayed but cannot be selected.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "hidden",
        detail: "The tab is not displayed but remains managed.",
        ..ArgValue::DEFAULT
    },
];

const TAB_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-state",
        value: OptionValue::enumerated(TAB_STATES, true, "state"),
        detail: "The tab's display and selection state.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-sticky",
        value: OptionValue::value("stickySpec"),
        detail: "How the child window is positioned within the pane.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-padding",
        value: OptionValue::value("padding"),
        detail: "Extra space around the child window.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-text",
        value: OptionValue::value("text"),
        detail: "Text displayed in the tab label.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-image",
        value: OptionValue::value("imageName"),
        detail: "Image displayed in the tab label.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-compound",
        value: OptionValue::value("compound"),
        detail: "How the tab image and text are combined.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-underline",
        value: OptionValue::value("index"),
        detail: "Character index underlined in the tab label.",
        ..OptionSpec::DEFAULT
    },
];

/// The command's subcommands.
static SUBCOMMANDS: [SubCommand; 14] = [
    SubCommand {
        name: "cget",
        arity: Arity::exact(1),
        detail: "Return the current value of a widget option.",
        synopsis: "pathName cget option",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(0),
        detail: "Query or change widget options.",
        synopsis: "pathName configure ?option? ?value option value ...?",
        return_type: Some(TclType::String),
        subcommand_forms: super::common::CONFIGURE_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "add",
        arity: Arity::at_least(1),
        detail: "Add a new tab displaying the given window as a pane.",
        synopsis: "pathName add window ?options?",
        options: TAB_OPTIONS,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "Remove the tab specified by tabid and unmanage its window.",
        synopsis: "pathName forget tabid",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hide",
        arity: Arity::exact(1),
        detail: "Hide the tab specified by tabid without removing it.",
        synopsis: "pathName hide tabid",
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "identify",
        arity: Arity::exact(3),
        detail: "Identify the element or tab at the given coordinates.",
        synopsis: "pathName identify component x y",
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the numeric index of the tab specified by tabid.",
        synopsis: "pathName index tabid",
        pure: true,
        return_type: Some(TclType::Int),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Insert a tab at the specified position, adding or moving its window.",
        synopsis: "pathName insert pos window ?options?",
        options: TAB_OPTIONS,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "select",
        arity: Arity::new(0, 1),
        detail: "Select the given tab, or return the currently selected tab.",
        synopsis: "pathName select ?tabid?",
        return_type: Some(TclType::String),
        subcommand_forms: super::common::TAINTED_QUERY_OR_SET_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tab",
        arity: Arity::at_least(1),
        detail: "Query or modify the options of the tab specified by tabid.",
        synopsis: "pathName tab tabid ?option? ?value ...?",
        options: TAB_OPTIONS,
        mutator: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS_WRITES,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tabs",
        arity: Arity::exact(0),
        detail: "Return the list of windows managed by the notebook.",
        synopsis: "pathName tabs",
        pure: true,
        return_type: Some(TclType::List),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "instate",
        arity: Arity::new(1, 2),
        detail: "Test whether the widget state matches statespec, optionally running a script.",
        synopsis: "pathName instate statespec ?script?",
        // `script`, when given, runs as `if {[pathName instate statespec]} script`
        // — a real Tcl body.
        arg_roles: &[(1, ArgRole::Body)],
        traits: Traits::EVALUATES_CODE,
        side_effects: super::common::TTK_INSTATE_EFFECTS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "state",
        arity: Arity::new(0, 1),
        detail: "Modify or query the widget state.",
        synopsis: "pathName state ?stateSpec?",
        return_type: Some(TclType::List),
        subcommand_forms: super::common::QUERY_OR_SET_FORMS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "style",
        arity: Arity::exact(0),
        detail: "Return the widget's current style.",
        synopsis: "pathName style",
        lifecycle: Lifecycle::introduced_in("8.7"),
        pure: true,
        return_type: Some(TclType::String),
        side_effects: super::common::TTK_WIDGET_READS,
        ..SubCommand::DEFAULT
    },
];

/// `ttk::notebook`'s instance command dispatches through the same
/// subcommand table as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static TTK_NOTEBOOK_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "ttk::notebook",
    instance_methods: &SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
    method_prefix_matching: PrefixMatching::Enabled,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::notebook",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed tabbed notebook widget.",
            synopsis: &["ttk::notebook pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_notebook.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        lifecycle: Lifecycle::introduced_in("8.5"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: &SUBCOMMANDS,
        object_class: Some(&TTK_NOTEBOOK_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(name: &str) -> &'static SubCommand {
        SUBCOMMANDS
            .iter()
            .find(|sub| sub.name == name)
            .expect("subcommand exists")
    }

    #[test]
    fn notebook_cget_is_a_plain_widget_read() {
        let cget = sub("cget");
        assert!(cget.pure);
        assert_eq!(cget.side_effects, super::super::common::TTK_WIDGET_READS);
        assert!(
            !cget
                .side_effects
                .iter()
                .any(|effect| effect.target == SideEffectTarget::Unknown)
        );
    }

    #[test]
    fn notebook_configure_declares_instance_option_dispatch() {
        let configure = sub("configure");
        assert!(configure.traits.is_empty());
        assert!(!configure.mutator);
        assert!(configure.side_effects.is_empty());
        let setter = configure
            .subcommand_forms
            .iter()
            .find(|form| form.name == "set")
            .unwrap();
        assert!(
            setter
                .traits
                .is_some_and(|traits| traits.contains(Traits::CONFIGURES_INSTANCE_OPTIONS))
        );
        assert_eq!(setter.mutator, Some(true));
        assert!(
            setter
                .side_effects
                .is_some_and(|effects| effects.iter().any(|effect| effect.writes))
        );
    }
}
