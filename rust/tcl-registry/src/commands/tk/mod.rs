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

//! `tk` command specifications.

#![allow(non_snake_case)]

mod bell;
mod bind;
mod button;
mod canvas;
mod checkbutton;
mod clipboard;
mod common;
mod console;
mod destroy;
mod entry;
mod event;
mod focus;
mod font;
mod frame;
mod grab;
mod grid;
mod image;
mod label;
mod labelframe;
mod listbox;
mod lower;
mod menu;
mod menubutton;
mod message;
mod option;
mod pack;
mod panedwindow;
mod place;
mod radiobutton;
mod raise;
mod scale;
mod scrollbar;
mod selection;
mod send;
mod spinbox;
mod text;
mod tk_choosecolor;
mod tk_choosedirectory;
mod tk_cmd;
mod tk_extra_cmds;
mod tk_getopenfile;
mod tk_getsavefile;
mod tk_messagebox;
mod tk_popup;
mod tkwait;
mod toplevel;
mod ttk__button;
mod ttk__combobox;
mod ttk__entry;
mod ttk__frame;
mod ttk__label;
mod ttk__labelframe;
mod ttk__notebook;
mod ttk__progressbar;
mod ttk__scale;
mod ttk__scrollbar;
mod ttk__separator;
mod ttk__sizegrip;
mod ttk__style;
mod ttk__toggleswitch;
mod ttk__treeview;
mod ttk_extra;
mod winfo;
mod wm;

use crate::spec::CommandSpec;

/// Classic widget constructors exported in the `::tk` namespace from Tk 8.5.
///
/// Tk installs these as real command names alongside the historical global
/// constructors (see `TkCreateXEventSource`'s command table in
/// `generic/tkWindow.c`).  Keep the mapping as registry data so every consumer
/// resolves the aliases without a command-name branch of its own.
const TK_NAMESPACED_CLASSIC_ALIASES: &[(&str, &str)] = &[
    ("tk::button", "button"),
    ("tk::canvas", "canvas"),
    ("tk::checkbutton", "checkbutton"),
    ("tk::entry", "entry"),
    ("tk::frame", "frame"),
    ("tk::label", "label"),
    ("tk::labelframe", "labelframe"),
    ("tk::listbox", "listbox"),
    ("tk::menubutton", "menubutton"),
    ("tk::message", "message"),
    ("tk::panedwindow", "panedwindow"),
    ("tk::radiobutton", "radiobutton"),
    ("tk::scale", "scale"),
    ("tk::scrollbar", "scrollbar"),
    ("tk::spinbox", "spinbox"),
    ("tk::text", "text"),
    ("tk::toplevel", "toplevel"),
];

/// Return all `tk` command specifications.
#[must_use]
pub fn tk_command_specs() -> Vec<CommandSpec> {
    let mut specs = tk_command_specs_raw();
    specs.extend(ttk_extra::specs());
    specs.extend(tk_extra_cmds::specs());
    specs.extend(console::specs());
    let aliases: Vec<_> = TK_NAMESPACED_CLASSIC_ALIASES
        .iter()
        .map(|&(alias, canonical)| {
            let mut spec = specs
                .iter()
                .find(|spec| spec.name == canonical)
                .unwrap_or_else(|| panic!("Tk alias source `{canonical}` is registered"))
                .clone();
            spec.name = alias;
            spec.lifecycle.introduced = Some("8.5");
            spec
        })
        .collect();
    specs.extend(aliases);
    // The themed-widget set (`ttk::*`) was introduced with Tk 8.5.  Stamp the
    // package lifecycle here rather than in every ttk spec file so the gate
    // stays in one place.
    for spec in &mut specs {
        if spec.lifecycle.introduced.is_none() && spec.name.starts_with("ttk::") {
            spec.lifecycle.introduced = Some("8.5");
        }
    }
    specs
}

fn tk_command_specs_raw() -> Vec<CommandSpec> {
    vec![
        bell::spec(),
        bind::spec(),
        button::spec(),
        canvas::spec(),
        checkbutton::spec(),
        clipboard::spec(),
        destroy::spec(),
        entry::spec(),
        event::spec(),
        focus::spec(),
        font::spec(),
        frame::spec(),
        grab::spec(),
        grid::spec(),
        image::spec(),
        label::spec(),
        labelframe::spec(),
        listbox::spec(),
        lower::spec(),
        menu::spec(),
        menubutton::spec(),
        message::spec(),
        option::spec(),
        pack::spec(),
        panedwindow::spec(),
        place::spec(),
        radiobutton::spec(),
        raise::spec(),
        scale::spec(),
        scrollbar::spec(),
        selection::spec(),
        send::spec(),
        spinbox::spec(),
        text::spec(),
        tkwait::spec(),
        tk_cmd::spec(),
        tk_choosecolor::spec(),
        tk_choosedirectory::spec(),
        tk_getopenfile::spec(),
        tk_getsavefile::spec(),
        tk_messagebox::spec(),
        tk_popup::spec(),
        toplevel::spec(),
        ttk__button::spec(),
        ttk__combobox::spec(),
        ttk__entry::spec(),
        ttk__frame::spec(),
        ttk__label::spec(),
        ttk__labelframe::spec(),
        ttk__notebook::spec(),
        ttk__progressbar::spec(),
        ttk__scale::spec(),
        ttk__separator::spec(),
        ttk__scrollbar::spec(),
        ttk__sizegrip::spec(),
        ttk__style::spec(),
        ttk__treeview::spec(),
        ttk__toggleswitch::spec(),
        winfo::spec(),
        wm::spec(),
    ]
}

#[cfg(test)]
mod tests {
    use tcl_dialect::model::{SurfaceQuery, Family};
    
    use super::tk_command_specs;

    fn assert_complete_prose(owner: &str, prose: &str) {
        let prose = prose.trim();
        assert!(!prose.is_empty(), "{owner} has empty documentation");
        assert_ne!(prose, ".", "{owner} has placeholder documentation");
        assert!(
            !prose.contains(",."),
            "{owner} has malformed punctuation: {prose}"
        );

        let lower = prose.to_ascii_lowercase();
        for ending in [
            " a.",
            " an.",
            " an empty.",
            " and.",
            " extra.",
            " if.",
            " may.",
            " of.",
            " or.",
            " ought to.",
            " should be.",
            " that.",
            " the.",
            " this.",
            ". used.",
        ] {
            assert!(
                !lower.ends_with(ending),
                "{owner} ends with an obvious sentence fragment: {prose}"
            );
        }
    }

    fn assert_options_are_documented(owner: &str, options: &[crate::prelude::OptionSpec]) {
        for option in options {
            assert_complete_prose(&format!("{owner} {}", option.name), option.detail);
        }
    }

    fn assert_subcommands_are_documented(owner: &str, subcommands: &[crate::SubCommand]) {
        for subcommand in subcommands {
            let sub_owner = format!("{owner} {}", subcommand.name);
            assert_complete_prose(&sub_owner, subcommand.detail);
            assert!(
                !subcommand.synopsis.trim().is_empty(),
                "{sub_owner} has an empty synopsis"
            );
            assert_options_are_documented(&sub_owner, subcommand.options);
            for nested in subcommand.sub_subcommands {
                let nested_owner = format!("{sub_owner} {}", nested.name);
                assert_complete_prose(&nested_owner, nested.detail);
                assert!(
                    !nested.synopsis.trim().is_empty(),
                    "{nested_owner} has an empty synopsis"
                );
                if let Some(options) = nested.options {
                    assert_options_are_documented(&nested_owner, options);
                }
            }
        }
    }

    #[test]
    fn tk_completion_and_hover_text_has_no_obvious_truncation() {
        for spec in tk_command_specs() {
            assert_options_are_documented(spec.name, spec.options);
            assert_subcommands_are_documented(spec.name, spec.subcommands);
            if let Some(class) = spec.object_class {
                assert_subcommands_are_documented(spec.name, class.instance_methods);
            }
            if let Some(hover) = spec.hover {
                assert_complete_prose(spec.name, hover.summary);
                for synopsis in hover.synopsis {
                    assert!(
                        !synopsis.trim().is_empty(),
                        "{} has an empty hover synopsis",
                        spec.name
                    );
                }
            }
        }
    }

    #[test]
    fn tk_command_specs_are_well_formed_and_cover_core_widgets() {
        let specs = tk_command_specs();
        assert!(!specs.is_empty(), "tk specs are registered");
        // Every spec carries a non-empty command name.
        assert!(specs.iter().all(|s| !s.name.is_empty()));
        // No duplicate command names slipped into the table.
        let mut names: Vec<&str> = specs.iter().map(|s| s.name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "duplicate tk command name in the table");
        // The core widgets, geometry managers, and window utilities are present.
        let has = |n: &str| specs.iter().any(|s| s.name == n);
        for cmd in [
            "button", "label", "frame", "entry", "canvas", "menu", "listbox", "text", "toplevel",
            "pack", "grid", "place", "bind", "focus", "wm", "winfo",
        ] {
            assert!(has(cmd), "tk command `{cmd}` is registered");
        }
    }

    #[test]
    fn newly_added_tk_commands_are_present() {
        let specs = tk_command_specs();
        let has = |n: &str| specs.iter().any(|s| s.name == n);
        // The remaining themed widgets.
        for w in [
            "ttk::checkbutton",
            "ttk::labelframe",
            "ttk::menubutton",
            "ttk::panedwindow",
            "ttk::radiobutton",
            "ttk::scrollbar",
            "ttk::spinbox",
            "ttk::toggleswitch",
        ] {
            assert!(has(w), "themed widget `{w}` is registered");
        }
        // Additional standalone commands.
        for c in [
            "bindtags",
            "tk_optionMenu",
            "tk_dialog",
            "tk_setPalette",
            "tk_focusNext",
            "tk_focusPrev",
            "console",
            "consoleinterp",
        ] {
            assert!(has(c), "tk command `{c}` is registered");
        }
        // ttk::spinbox is gated to 8.5 like the rest of the themed set.
        let spin = specs.iter().find(|s| s.name == "ttk::spinbox").unwrap();
        assert_eq!(spin.lifecycle.introduced, Some("8.5"));
        let send = specs.iter().find(|s| s.name == "send").unwrap();
        assert!(send.lifecycle.is_unspecified());
    }

    #[test]
    fn console_and_consoleinterp_eval_bodies_are_registered_correctly() {
        use crate::{ArgRole, Arity};
        // Issue #925: `console eval` / `consoleinterp eval` / `consoleinterp
        // record` each take exactly one script argument that must resolve as
        // `ArgRole::Body` (so the LSP recurses into it) and must be listed as
        // a cross-interpreter eval sink (T105) — same shape as `interp eval`.
        // Stable across Tk 8.4-9.0 (no lifecycle gate on any subcommand).
        let specs = tk_command_specs();

        let console = specs.iter().find(|s| s.name == "console").unwrap();
        assert!(console.lifecycle.is_unspecified());
        assert_eq!(console.taint_interp_eval_subcommands, &["eval"]);
        let console_eval = console.resolve_subcommand("eval").unwrap();
        assert_eq!(console_eval.arity, Arity::exact(1));
        assert_eq!(console_eval.arg_role_at(0), Some(ArgRole::Body));
        for name in ["hide", "show", "title"] {
            let sub = console.resolve_subcommand(name).unwrap();
            assert_eq!(sub.arg_role_at(0), None, "`console {name}` has no body arg");
        }

        let consoleinterp = specs.iter().find(|s| s.name == "consoleinterp").unwrap();
        assert!(consoleinterp.lifecycle.is_unspecified());
        assert_eq!(
            consoleinterp.taint_interp_eval_subcommands,
            &["eval", "record"]
        );
        for name in ["eval", "record"] {
            let sub = consoleinterp.resolve_subcommand(name).unwrap();
            assert_eq!(sub.arity, Arity::exact(1));
            assert_eq!(sub.arg_role_at(0), Some(ArgRole::Body));
        }
    }

    #[test]
    fn ttk_instate_script_arg_is_a_body_with_tight_arity() {
        // Found while auditing for issue #925 siblings: `pathName instate
        // statespec ?script?` runs `script` as `if {[pathName instate
        // statespec]} script` per the ttk::widget manual page — a real body,
        // same shape as `console eval`, but was declared with an unbounded
        // `Arity::at_least(1)` and no `ArgRole::Body`.
        use crate::{ArgRole, Arity, Traits};
        let specs = tk_command_specs();
        for widget in ["ttk::treeview", "ttk::notebook"] {
            let spec = specs.iter().find(|s| s.name == widget).unwrap();
            let instate = spec.resolve_subcommand("instate").unwrap();
            assert_eq!(instate.arity, Arity::new(1, 2), "{widget} instate arity");
            assert_eq!(
                instate.arg_role_at(1),
                Some(ArgRole::Body),
                "{widget} instate script arg"
            );
            assert!(
                instate.traits.contains(Traits::EVALUATES_CODE),
                "{widget} instate invokes its optional script"
            );
        }
    }

    #[test]
    fn ttk_widgets_are_gated_to_tk_85() {
        let specs = tk_command_specs();
        let ttk_button = specs.iter().find(|s| s.name == "ttk::button").unwrap();
        assert_eq!(ttk_button.lifecycle.introduced, Some("8.5"));
        // Available floor >= 8.5 -> present; older -> absent.
        assert!(ttk_button.available_for_version(Some("8.5")));
        assert!(ttk_button.available_for_version(Some("9.0")));
        assert!(!ttk_button.available_for_version(Some("8.4")));
        // A plain widget carries no package lifecycle gate.
        let button = specs.iter().find(|s| s.name == "button").unwrap();
        assert!(button.lifecycle.is_unspecified());
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn added_ttk_widgets_have_lifecycle_instances_and_callback_shapes() {
        use crate::side_effects::SideEffectTarget;
        use crate::{AppendedArity, AppendedAritySet, ArgRole, Traits};

        let specs = tk_command_specs();
        for name in ["ttk::labelframe", "ttk::scrollbar"] {
            let spec = specs.iter().find(|s| s.name == name).unwrap();
            assert_eq!(spec.lifecycle.introduced, Some("8.5"), "{name} lifecycle");
            assert_eq!(spec.creates_instance_at, Some(0), "{name} factory position");
            assert!(spec.object_class.is_some(), "{name} has instance typing");
            assert!(
                spec.resolve_subcommand("instate").is_some(),
                "{name} has instate"
            );
            if name == "ttk::labelframe" {
                assert!(spec.find_option("-borderwidth", None, None).is_some());
                assert!(spec.find_option("-relief", None, None).is_some());
            }
            assert_eq!(
                spec.resolve_subcommand("identify").unwrap().arity,
                crate::Arity::exact(3),
                "{name} identify coordinates"
            );
            assert_eq!(
                spec.resolve_subcommand("style")
                    .unwrap()
                    .lifecycle
                    .introduced,
                Some("8.7"),
                "{name} style lifecycle"
            );
            assert_eq!(
                spec.resolve_subcommand("style").unwrap().arity,
                crate::Arity::exact(0),
                "{name} style is query-only"
            );
        }

        for scrollbar in ["scrollbar", "ttk::scrollbar"] {
            let spec = specs.iter().find(|spec| spec.name == scrollbar).unwrap();
            let command = spec.find_option("-command", None, None).unwrap();
            assert_eq!(command.value_role(), Some(ArgRole::CommandPrefix));
            assert_eq!(
                command.value_appended_arity(),
                AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&[2, 3])),
                "{scrollbar} calls -command with either moveto/fraction or scroll/count/unit"
            );
        }

        let toggleswitch = specs
            .iter()
            .find(|s| s.name == "ttk::toggleswitch")
            .unwrap();
        assert_eq!(toggleswitch.lifecycle.introduced, Some("9.1"));
        assert!(!toggleswitch.available_for_version(Some("9.0")));
        assert!(toggleswitch.available_for_version(Some("9.1")));
        assert!(toggleswitch.object_class.is_some());
        assert_eq!(
            toggleswitch
                .find_option("-command", None, None)
                .unwrap()
                .value_role(),
            Some(ArgRole::Body)
        );
        assert_eq!(
            toggleswitch
                .find_option("-variable", None, None)
                .unwrap()
                .value_role(),
            Some(ArgRole::VarWrite)
        );
        let switchstate = toggleswitch.resolve_subcommand("switchstate").unwrap();
        assert!(switchstate.traits.is_empty());
        assert!(!switchstate.mutator);
        assert!(switchstate.side_effects.is_empty());
        assert_eq!(switchstate.subcommand_forms.len(), 2);
        let get = toggleswitch.resolve_subcommand("get").unwrap();
        assert_eq!(get.return_type, Some(crate::TclType::Double));
        assert!(get.traits.contains(Traits::TAINT_SOURCE_ZERO_ARGS));
        assert!(!get.traits.contains(Traits::TAINT_SOURCE));
        let xcoord = toggleswitch.resolve_subcommand("xcoord").unwrap();
        assert!(xcoord.pure);
        assert!(xcoord.traits.contains(Traits::TAINT_SOURCE_ZERO_ARGS));
        assert!(!xcoord.mutator);
        for supported in [
            "-class",
            "-command",
            "-cursor",
            "-offvalue",
            "-onvalue",
            "-size",
            "-style",
            "-takefocus",
            "-variable",
        ] {
            assert!(
                toggleswitch.find_option(supported, None, None).is_some(),
                "ttk::toggleswitch must expose its documented option: {supported}"
            );
        }
        for unsupported in ["-text", "-textvariable", "-underline", "-width"] {
            assert!(
                toggleswitch.find_option(unsupported, None, None).is_none(),
                "ttk::toggleswitch must not inherit an undocumented text option: {unsupported}"
            );
        }
        let treeview = specs
            .iter()
            .find(|spec| spec.name == "ttk::treeview")
            .unwrap();
        assert_eq!(
            treeview
                .resolve_subcommand("style")
                .unwrap()
                .lifecycle
                .introduced,
            Some("9.0")
        );

        let send = specs.iter().find(|s| s.name == "send").unwrap();
        assert_eq!(send.arity, crate::Arity::at_least(2));
        assert!(send.find_option("-async", None, None).is_some());
        assert!(send.find_option("-displayof", None, None).is_some());
        assert_eq!(
            super::send::send_arg_roles(&["app", "cmd", "arg"]),
            vec![(0, ArgRole::Name)]
        );
        assert_eq!(
            super::send::send_arg_roles(&["-displayof", ".", "app", "cmd"]),
            vec![(2, ArgRole::Name), (3, ArgRole::Body)]
        );
        assert_eq!(
            super::send::send_arg_roles(&["-async", "app", "cmd", "arg"]),
            vec![(1, ArgRole::Name)]
        );
        assert!(
            send.side_effects
                .iter()
                .any(|effect| { effect.target == SideEffectTarget::NetworkIo && effect.writes })
        );
    }

    #[test]
    fn ttk_callback_and_editing_methods_declare_effects() {
        use crate::side_effects::SideEffectTarget;
        use crate::{TclType, Traits};

        let specs = tk_command_specs();
        for widget in ["ttk::button", "ttk::checkbutton", "ttk::radiobutton"] {
            let invoke = specs
                .iter()
                .find(|spec| spec.name == widget)
                .unwrap()
                .resolve_subcommand("invoke")
                .unwrap();
            assert!(invoke.traits.contains(Traits::EVALUATES_CODE), "{widget}");
            assert!(invoke.mutator, "{widget}");
            assert!(
                invoke.side_effects.iter().any(|effect| {
                    effect.target == SideEffectTarget::InterpState && effect.writes
                })
            );
        }

        let progress = specs
            .iter()
            .find(|spec| spec.name == "ttk::progressbar")
            .unwrap();
        for method in ["start", "step", "stop"] {
            let method = progress.resolve_subcommand(method).unwrap();
            assert!(method.mutator);
            assert_eq!(method.return_type, Some(TclType::String));
            assert!(method.side_effects.iter().any(|effect| effect.writes));
        }

        for widget in ["ttk::entry", "ttk::spinbox"] {
            let spec = specs.iter().find(|spec| spec.name == widget).unwrap();
            for method in ["delete", "insert", "validate"] {
                let method = spec.resolve_subcommand(method).unwrap();
                assert!(method.mutator, "{widget} {method:?}");
                assert!(
                    method.traits.contains(Traits::EVALUATES_CODE),
                    "{widget} {method:?} must model validation callbacks"
                );
                assert!(method.side_effects.iter().any(|effect| {
                    effect.target == SideEffectTarget::Unknown && effect.reads && effect.writes
                }));
            }
            assert_eq!(
                spec.resolve_subcommand("validate").unwrap().return_type,
                Some(TclType::Boolean)
            );
        }
        let combo = specs
            .iter()
            .find(|spec| spec.name == "ttk::combobox")
            .unwrap();
        for option in [
            "-validate",
            "-validatecommand",
            "-invalidcommand",
            "-locale",
        ] {
            assert!(combo.find_option(option, None, None).is_some(), "{option}");
        }
        assert!(combo.resolve_subcommand("set").unwrap().mutator);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn user_editable_widget_values_are_complete_taint_sources() {
        use crate::{ArgRole, Traits, VariableScope, prelude::SideEffectTarget};

        let specs = tk_command_specs();
        for command in [
            "entry",
            "spinbox",
            "text",
            "scale",
            "ttk::entry",
            "ttk::combobox",
            "ttk::spinbox",
            "ttk::scale",
            "ttk::toggleswitch",
        ] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            let class = spec
                .object_class
                .unwrap_or_else(|| panic!("{command} must expose its runtime instance API"));
            let get = class
                .instance_methods
                .iter()
                .find(|method| method.name == "get")
                .unwrap_or_else(|| panic!("{command} must expose its value getter"));
            assert!(
                get.traits
                    .intersects(Traits::TAINT_SOURCE | Traits::TAINT_SOURCE_ZERO_ARGS),
                "{command} get must classify user-controlled data"
            );
            assert!(get.pure, "{command} get must be a read, not a mutation");
            assert!(get.return_type.is_some(), "{command} get return type");
            assert!(!get.detail.is_empty(), "{command} get hover detail");
            assert!(!get.synopsis.is_empty(), "{command} get synopsis");
            assert!(
                get.side_effects
                    .iter()
                    .any(|effect| effect.target == SideEffectTarget::InterpState && effect.reads),
                "{command} get must declare its widget-state read"
            );
        }

        for (command, option) in [
            ("entry", "-textvariable"),
            ("spinbox", "-textvariable"),
            ("scale", "-variable"),
            ("checkbutton", "-variable"),
            ("radiobutton", "-variable"),
            ("ttk::entry", "-textvariable"),
            ("ttk::combobox", "-textvariable"),
            ("ttk::spinbox", "-textvariable"),
            ("ttk::scale", "-variable"),
            ("ttk::checkbutton", "-variable"),
            ("ttk::radiobutton", "-variable"),
            ("ttk::toggleswitch", "-variable"),
        ] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            assert!(
                spec.traits.contains(Traits::TAINTS_VAR_WRITES),
                "{command} must taint user-editable linked state"
            );
            let linked = spec.find_option(option, None, None).unwrap();
            assert_eq!(linked.value_role(), Some(ArgRole::VarWrite));
            assert_eq!(linked.value_also_role(), Some(ArgRole::VarRead));
            assert_eq!(
                linked.value_variable_scope(),
                Some(VariableScope::Global),
                "{command} {option} is documented as a global Tk variable link"
            );
            assert!(
                linked.taints_var_write(),
                "{command} {option} must identify its specific external-input link"
            );
        }

        // A display-only textvariable reflects program state into the widget;
        // the user cannot edit it, so it is not itself an input boundary.
        for command in ["label", "button", "ttk::label", "ttk::button"] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            assert!(!spec.traits.contains(Traits::TAINTS_VAR_WRITES));
            assert!(
                !spec
                    .find_option("-textvariable", None, None)
                    .unwrap()
                    .taints_var_write()
            );
            assert_eq!(
                spec.find_option("-textvariable", None, None)
                    .unwrap()
                    .value_variable_scope(),
                Some(VariableScope::Global)
            );
        }
        for command in [
            "checkbutton",
            "radiobutton",
            "ttk::checkbutton",
            "ttk::radiobutton",
        ] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            assert!(
                !spec
                    .find_option("-textvariable", None, None)
                    .unwrap()
                    .taints_var_write()
            );
        }

        for command in ["clipboard", "selection"] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            let get = spec.resolve_subcommand("get").unwrap();
            assert!(get.traits.contains(Traits::TAINT_SOURCE));
            assert!(get.return_type.is_some());
            assert!(
                !get.pure,
                "{command} get reads externally mutable selection state"
            );
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn zero_argument_treeview_and_notebook_queries_are_sources_only_in_query_form() {
        use crate::{CommandRegistry, Traits};

        let registry = CommandRegistry::build_default();
        let cases = [
            (
                "ttk::combobox",
                "current",
                &["current"][..],
                &["current", "0"] as &[&str],
            ),
            (
                "ttk::treeview",
                "selection",
                &["selection"][..],
                &["selection", "set"] as &[&str],
            ),
            (
                "ttk::treeview",
                "cellselection",
                &["cellselection"][..],
                &["cellselection", "set"] as &[&str],
            ),
            (
                "ttk::treeview",
                "focus",
                &["focus"][..],
                &["focus", "item"] as &[&str],
            ),
            (
                "ttk::treeview",
                "cellfocus",
                &["cellfocus"][..],
                &["cellfocus", "0,0"] as &[&str],
            ),
            (
                "ttk::notebook",
                "select",
                &["select"][..],
                &["select", ".tab"] as &[&str],
            ),
        ];
        for (command, method, query, setter) in cases {
            let spec = registry.get(command).expect("Tk command");
            let sub = spec.resolve_subcommand(method).expect("Tk method");
            assert!(
                sub.traits.is_empty(),
                "neutral parent row: {command} {method}"
            );
            assert!(!sub.mutator, "neutral parent row: {command} {method}");
            assert!(
                sub.side_effects.is_empty(),
                "neutral parent row: {command} {method}"
            );
            let query = registry
                .resolve_instance_invocation(command, ".w", query, None)
                .expect("query form resolves");
            let setter = registry
                .resolve_instance_invocation(command, ".w", setter, None)
                .expect("setter form resolves");
            assert!(
                query
                    .semantics
                    .traits
                    .contains(Traits::TAINT_SOURCE_ZERO_ARGS | Traits::PURE),
                "{command} {method}"
            );
            assert!(
                !query.semantics.mutator
                    && query
                        .semantics
                        .side_effects
                        .iter()
                        .all(|effect| !effect.writes),
                "query form must not write: {command} {method}"
            );
            assert!(
                setter.semantics.mutator
                    && setter
                        .semantics
                        .side_effects
                        .iter()
                        .any(|effect| effect.writes),
                "setter form must write: {command} {method}"
            );
            assert!(
                !setter
                    .semantics
                    .traits
                    .contains(Traits::TAINT_SOURCE_ZERO_ARGS | Traits::PURE),
                "setter form must not inherit query traits: {command} {method}"
            );
        }

        for args in [
            &["pointerx", "."][..],
            &["pointery", "."][..],
            &["pointerxy", "."][..],
            &["containing", "10", "20"][..],
            &["containing", "-displayof", ".", "10", "20"][..],
        ] {
            assert!(crate::taint::is_taint_source(
                &registry,
                "winfo",
                args,
                Some(SurfaceQuery::core(Family::Tcl, "9.0"))
            ));
        }
    }

    #[test]
    fn instance_method_forms_resolve_query_and_mutation_effects_exactly() {
        use crate::{CommandRegistry, Traits, prelude::SideEffectTarget};

        let registry = CommandRegistry::build_default();
        let cases: &[(&str, &[&str], &[&str])] = &[
            (
                "button",
                &["configure", "-text"],
                &["configure", "-text", "Save"],
            ),
            ("entry", &["configure"], &["configure", "-width", "20"]),
            (
                "text",
                &["configure", "-wrap"],
                &["configure", "-wrap", "word"],
            ),
            (
                "ttk::button",
                &["configure"],
                &["configure", "-text", "Save"],
            ),
            ("ttk::frame", &["state"], &["state", "disabled"]),
            ("ttk::combobox", &["current"], &["current", "2"]),
            ("ttk::notebook", &["select"], &["select", ".n.page"]),
            ("ttk::treeview", &["focus"], &["focus", "item"]),
            (
                "ttk::treeview",
                &["selection"],
                &["selection", "set", "item"],
            ),
            ("ttk::toggleswitch", &["switchstate"], &["switchstate", "1"]),
        ];
        for (class, query_args, mutation_args) in cases {
            let query = registry
                .resolve_instance_invocation(class, ".w", query_args, None)
                .unwrap_or_else(|| panic!("query form resolves: {class} {query_args:?}"));
            assert_eq!(query.form.map(|form| form.name), Some("query"));
            assert!(query.semantics.traits.contains(Traits::PURE));
            assert!(!query.semantics.mutator);
            assert!(
                query
                    .semantics
                    .side_effects
                    .iter()
                    .all(|effect| !effect.writes)
            );

            let mutation = registry
                .resolve_instance_invocation(class, ".w", mutation_args, None)
                .unwrap_or_else(|| panic!("mutation form resolves: {class} {mutation_args:?}"));
            assert!(matches!(
                mutation.form.map(|form| form.name),
                Some("set" | "modify")
            ));
            assert!(!mutation.semantics.traits.contains(Traits::PURE));
            assert!(mutation.semantics.mutator);
            assert!(
                mutation
                    .semantics
                    .side_effects
                    .iter()
                    .any(|effect| effect.writes)
            );
        }

        let configure = registry
            .resolve_instance_invocation(
                "button",
                ".w",
                &["configure", "-text", "Save"],
                None,
            )
            .unwrap();
        assert!(
            configure
                .semantics
                .traits
                .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
        );

        let switchstate = registry
            .resolve_instance_invocation(
                "ttk::toggleswitch",
                ".w",
                &["switchstate", "1"],
                None,
            )
            .unwrap();
        assert!(
            switchstate
                .semantics
                .traits
                .contains(Traits::EVALUATES_CODE)
        );
        assert!(
            switchstate
                .semantics
                .side_effects
                .iter()
                .any(|effect| effect.target == SideEffectTarget::Unknown && effect.writes)
        );
    }

    #[test]
    fn literal_operation_forms_cover_tk_nested_method_tables() {
        use crate::{CommandRegistry};

        let registry = CommandRegistry::build_default();
        let cases: &[(&str, &[&str], &[&str])] = &[
            ("entry", &["selection", "present"], &["selection", "clear"]),
            (
                "spinbox",
                &["selection", "element"],
                &["selection", "element", "buttonup"],
            ),
            (
                "ttk::entry",
                &["selection", "present"],
                &["selection", "range", "0", "end"],
            ),
            (
                "ttk::combobox",
                &["selection", "present"],
                &["selection", "clear"],
            ),
            (
                "ttk::spinbox",
                &["selection", "present"],
                &["selection", "clear"],
            ),
            (
                "listbox",
                &["selection", "includes", "0"],
                &["selection", "set", "0"],
            ),
            ("canvas", &["select", "item"], &["select", "clear"]),
            ("panedwindow", &["proxy", "coord"], &["proxy", "forget"]),
            (
                "panedwindow",
                &["sash", "coord", "0"],
                &["sash", "place", "0", "10", "20"],
            ),
            ("text", &["edit", "canundo"], &["edit", "undo"]),
            ("text", &["edit", "modified"], &["edit", "modified", "1"]),
            ("text", &["image", "names"], &["image", "create", "1.0"]),
            (
                "text",
                &["mark", "gravity", "insert"],
                &["mark", "gravity", "insert", "left"],
            ),
            ("text", &["peer", "names"], &["peer", "create", ".peer"]),
            ("text", &["tag", "ranges", "hot"], &["tag", "raise", "hot"]),
            (
                "text",
                &["window", "configure", "1.0"],
                &["window", "configure", "1.0", "-padx", "2"],
            ),
            (
                "ttk::treeview",
                &["tag", "has", "hot", "item"],
                &["tag", "add", "hot", "item"],
            ),
            (
                "ttk::treeview",
                &["tag", "cell", "has", "hot"],
                &["tag", "cell", "remove", "hot"],
            ),
        ];

        for (class, query_args, mutation_args) in cases {
            let query = registry
                .resolve_instance_invocation(class, ".w", query_args, None)
                .unwrap_or_else(|| panic!("query resolves: {class} {query_args:?}"));
            assert!(query.form.is_some(), "query form: {class} {query_args:?}");
            assert!(!query.semantics.mutator, "query: {class} {query_args:?}");
            assert!(
                query
                    .semantics
                    .side_effects
                    .iter()
                    .all(|effect| !effect.writes),
                "query effects: {class} {query_args:?}"
            );

            let mutation = registry
                .resolve_instance_invocation(class, ".w", mutation_args, None)
                .unwrap_or_else(|| panic!("mutation resolves: {class} {mutation_args:?}"));
            assert!(
                mutation.semantics.mutator
                    && mutation
                        .semantics
                        .side_effects
                        .iter()
                        .any(|effect| effect.writes),
                "mutation: {class} {mutation_args:?}"
            );
        }
    }

    #[test]
    fn literal_operation_selection_abstains_for_dynamic_unknown_and_ambiguous_words() {
        use crate::{CommandRegistry, InvocationWord, InvocationWords};

        let registry = CommandRegistry::build_default();
        let dynamic = [
            InvocationWord::Literal("selection"),
            InvocationWord::Dynamic,
        ];
        let invocation = registry
            .resolve_structured_instance_invocation(
                "entry",
                InvocationWords::structured(InvocationWord::Literal(".e"), &dynamic),
                None,
            )
            .expect("the method row still resolves");
        assert!(invocation.form.is_none());
        assert!(invocation.semantics.mutator);
        assert!(
            invocation
                .semantics
                .side_effects
                .iter()
                .any(|effect| effect.writes)
        );

        for (class, args) in [
            ("entry", &["selection", "unknown"] as &[&str]),
            ("text", &["tag", "ra", "hot"] as &[&str]),
            ("ttk::treeview", &["tag", "c", "hot"] as &[&str]),
        ] {
            let invocation = registry
                .resolve_instance_invocation(class, ".w", args, None)
                .unwrap_or_else(|| panic!("parent method resolves: {class} {args:?}"));
            assert!(invocation.form.is_none(), "must abstain: {class} {args:?}");
            assert!(invocation.semantics.mutator, "parent fallback: {class}");
        }

        let abbreviated = registry
            .resolve_instance_invocation("entry", ".e", &["selection", "pres"], None)
            .expect("unique literal prefix resolves");
        assert_eq!(abbreviated.form.map(|form| form.name), Some("present"));
        assert!(!abbreviated.semantics.mutator);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn classic_widget_instance_apis_model_configuration_and_callbacks() {
        use crate::{CommandRegistry, Traits, prelude::SideEffectTarget};

        let specs = tk_command_specs();
        let registry = CommandRegistry::build_default();
        for command in [
            "button",
            "entry",
            "frame",
            "label",
            "labelframe",
            "menubutton",
            "message",
            "scrollbar",
            "spinbox",
            "scale",
            "checkbutton",
            "radiobutton",
            "text",
            "listbox",
            "toplevel",
        ] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            let class = spec
                .object_class
                .unwrap_or_else(|| panic!("{command} must expose an instance class"));
            for method in ["cget", "configure"] {
                let sub = class
                    .instance_method(method)
                    .unwrap_or_else(|| panic!("{command} must expose {method}"));
                assert!(!sub.detail.is_empty());
                assert!(!sub.synopsis.is_empty());
                assert!(sub.return_type.is_some());
                if method == "cget" {
                    assert!(sub.side_effects.iter().any(|effect| {
                        effect.target == SideEffectTarget::InterpState && effect.reads
                    }));
                } else {
                    assert_eq!(sub.subcommand_forms.len(), 2);
                }
            }
        }

        for command in [
            "button",
            "frame",
            "label",
            "labelframe",
            "menubutton",
            "message",
            "scrollbar",
            "toplevel",
        ] {
            let class = specs
                .iter()
                .find(|spec| spec.name == command)
                .and_then(|spec| spec.object_class)
                .unwrap();
            let cget = class.instance_method("cget").unwrap();
            assert!(cget.pure && !cget.mutator, "{command} cget");
            assert!(cget.side_effects.iter().all(|effect| !effect.writes));

            let configure = class.instance_method("configure").unwrap();
            assert!(
                !configure.mutator && !configure.pure,
                "neutral {command} configure"
            );
            assert!(configure.traits.is_empty(), "neutral {command} configure");
            assert!(
                configure.side_effects.is_empty(),
                "neutral {command} configure"
            );
            let query = registry
                .resolve_instance_invocation(command, ".w", &["configure"], None)
                .unwrap();
            assert!(query.semantics.traits.contains(Traits::PURE));
            assert!(!query.semantics.mutator);
            assert!(
                query
                    .semantics
                    .side_effects
                    .iter()
                    .all(|effect| !effect.writes)
            );
            let setter = registry
                .resolve_instance_invocation(
                    command,
                    ".w",
                    &["configure", "-text", "value"],
                    None,
                )
                .unwrap();
            assert!(setter.semantics.mutator);
            assert!(
                setter
                    .semantics
                    .traits
                    .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
            );
            assert!(setter.semantics.side_effects.iter().any(|effect| {
                effect.target == SideEffectTarget::InterpState && effect.reads && effect.writes
            }));
            assert!(class.instance_method("conf").is_some(), "{command} prefix");
        }

        let button = specs.iter().find(|spec| spec.name == "button").unwrap();
        let button_class = button.object_class.unwrap();
        assert!(!button_class.allow_unknown_methods);
        assert_eq!(
            button_class
                .instance_methods
                .iter()
                .map(|method| method.name)
                .collect::<Vec<_>>(),
            ["cget", "configure", "flash", "invoke"]
        );
        let invoke = button_class.instance_method("invoke").unwrap();
        assert!(invoke.traits.contains(Traits::EVALUATES_CODE));
        assert!(invoke.side_effects.iter().any(|effect| {
            effect.target == SideEffectTarget::Unknown && effect.reads && effect.writes
        }));
        assert_eq!(
            button
                .find_option("-command", None, None)
                .unwrap()
                .value_script_timing(),
            Some(crate::ScriptTiming::Deferred)
        );

        let scrollbar = specs.iter().find(|spec| spec.name == "scrollbar").unwrap();
        let scrollbar_class = scrollbar.object_class.unwrap();
        assert!(!scrollbar_class.allow_unknown_methods);
        assert_eq!(
            scrollbar_class
                .instance_methods
                .iter()
                .map(|method| method.name)
                .collect::<Vec<_>>(),
            [
                "activate",
                "cget",
                "configure",
                "delta",
                "fraction",
                "get",
                "identify",
                "set",
            ]
        );
        for method in ["delta", "fraction", "get", "identify"] {
            let method = scrollbar_class.instance_method(method).unwrap();
            assert!(method.pure && !method.mutator, "{method:?}");
            assert!(method.return_type.is_some(), "{method:?}");
            assert!(method.side_effects.iter().all(|effect| !effect.writes));
        }
        for method in ["activate", "set"] {
            let method = scrollbar_class.instance_method(method).unwrap();
            assert!(method.mutator && !method.pure, "{method:?}");
            assert!(method.side_effects.iter().any(|effect| effect.writes));
        }

        for (command, method) in [
            ("entry", "validate"),
            ("spinbox", "validate"),
            ("spinbox", "invoke"),
            ("scale", "set"),
            ("checkbutton", "invoke"),
            ("radiobutton", "invoke"),
        ] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            let sub = spec.object_class.unwrap().instance_method(method).unwrap();
            assert!(sub.traits.contains(Traits::EVALUATES_CODE));
            assert!(sub.side_effects.iter().any(|effect| {
                effect.target == SideEffectTarget::Unknown && effect.reads && effect.writes
            }));
        }

        for (command, method) in [
            ("entry", "delete"),
            ("entry", "insert"),
            ("spinbox", "delete"),
            ("spinbox", "insert"),
        ] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            assert!(
                spec.object_class
                    .unwrap()
                    .instance_method(method)
                    .unwrap()
                    .traits
                    .contains(Traits::EVALUATES_CODE),
                "{command} {method} must model validation callbacks"
            );
            assert!(
                spec.object_class
                    .unwrap()
                    .instance_method(method)
                    .unwrap()
                    .side_effects
                    .iter()
                    .any(|effect| effect.target == SideEffectTarget::Unknown
                        && effect.reads
                        && effect.writes),
                "{command} {method} must retain unknown callback effects"
            );
        }
    }

    #[test]
    fn text_dump_models_its_synchronous_callback_form() {
        use crate::{
            AppendedArity, ArgRole, Traits,
            prelude::{OptionValue, SideEffectTarget},
        };

        let specs = tk_command_specs();
        let dump = specs
            .iter()
            .find(|spec| spec.name == "text")
            .unwrap()
            .resolve_subcommand("dump")
            .unwrap();
        assert!(!dump.pure);
        assert!(dump.traits.contains(Traits::TAINT_SOURCE));
        assert!(dump.traits.contains(Traits::EVALUATES_CODE));
        let command = dump
            .options
            .iter()
            .find(|option| option.name == "-command")
            .expect("dump -command option");
        let OptionValue::Takes(command) = command.value else {
            panic!("dump -command takes a value")
        };
        assert_eq!(command.role, ArgRole::CommandPrefix);
        assert_eq!(command.appended_arity, AppendedArity::Exactly(3));
        assert!(dump.side_effects.iter().any(|effect| {
            effect.target == SideEffectTarget::Unknown && effect.reads && effect.writes
        }));
    }

    #[test]
    fn classic_tk_namespace_constructors_follow_the_85_lifecycle() {
        let specs = tk_command_specs();
        for &(alias, canonical) in super::TK_NAMESPACED_CLASSIC_ALIASES {
            let alias_spec = specs.iter().find(|spec| spec.name == alias).unwrap();
            let canonical_spec = specs.iter().find(|spec| spec.name == canonical).unwrap();
            assert_eq!(alias_spec.lifecycle.introduced, Some("8.5"), "{alias}");
            assert!(!alias_spec.available_for_version(Some("8.4")), "{alias}");
            assert!(alias_spec.available_for_version(Some("8.5")), "{alias}");
            assert_eq!(
                alias_spec.creates_instance_at,
                canonical_spec.creates_instance_at
            );
            assert_eq!(
                alias_spec
                    .options
                    .iter()
                    .map(|option| option.name)
                    .collect::<Vec<_>>(),
                canonical_spec
                    .options
                    .iter()
                    .map(|option| option.name)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                alias_spec.object_class.map(|class| class.class_name),
                canonical_spec.object_class.map(|class| class.class_name)
            );
            assert_eq!(alias_spec.traits, canonical_spec.traits);
        }
    }

    #[test]
    fn entry_placeholder_is_gated_to_tk_87() {
        let specs = tk_command_specs();
        let entry = specs.iter().find(|s| s.name == "entry").unwrap();
        // Floor derived from `package require Tk <req>`.
        let floor = crate::version::requirement_lower_bound;
        // Tk 8.7+ -> -placeholder available; 8.6 -> not; unversioned -> permissive.
        assert!(
            entry
                .find_option("-placeholder", None, Some(floor("8.7")))
                .is_some()
        );
        assert!(
            entry
                .find_option("-placeholder", None, Some(floor("8.6")))
                .is_none()
        );
        assert!(entry.find_option("-placeholder", None, None).is_some());
        // Completion (canonical, version-gated) hides -placeholder under 8.6.
        let names_86 = entry.switch_names_ext(None, false, Some(floor("8.6")));
        assert!(!names_86.contains(&"-placeholder"));
        let names_87 = entry.switch_names_ext(None, false, Some(floor("8.7")));
        assert!(names_87.contains(&"-placeholder"));
    }

    #[test]
    fn tk_91_text_locale_rotation_and_inactive_selection_are_version_gated() {
        use crate::Arity;

        let specs = tk_command_specs();
        let floor = crate::version::requirement_lower_bound;

        for (command, option) in [
            ("entry", "-locale"),
            ("spinbox", "-locale"),
            ("text", "-locale"),
            ("ttk::entry", "-locale"),
            ("ttk::combobox", "-locale"),
            ("ttk::spinbox", "-locale"),
            ("label", "-textangle"),
            ("ttk::label", "-textangle"),
            ("listbox", "-inactiveselectbackground"),
            ("listbox", "-inactiveselectforeground"),
        ] {
            let spec = specs.iter().find(|spec| spec.name == command).unwrap();
            assert!(
                spec.find_option(option, None, Some(floor("9.0"))).is_none(),
                "{command} {option} must not leak into Tk 9.0"
            );
            assert!(
                spec.find_option(option, None, Some(floor("9.1"))).is_some(),
                "{command} {option} must exist in Tk 9.1"
            );
        }

        let text = specs.iter().find(|spec| spec.name == "text").unwrap();
        let locale = text.resolve_subcommand("locale").unwrap();
        assert_eq!(locale.lifecycle.introduced, Some("9.1"));
        assert_eq!(locale.arity, Arity::exact(1));
        assert!(locale.pure);

        let toggle = specs
            .iter()
            .find(|spec| spec.name == "ttk::toggleswitch")
            .unwrap();
        for invalid in ["-text", "-textvariable", "-underline", "-width"] {
            assert!(
                toggle
                    .find_option(invalid, None, Some(floor("9.1")))
                    .is_none(),
                "ttk::toggleswitch does not inherit label options: {invalid}"
            );
        }
    }

    #[test]
    fn tk_event_binding_bodies_are_explicitly_deferred() {
        use crate::ScriptTiming;

        let specs = tk_command_specs();
        let bind = specs.iter().find(|spec| spec.name == "bind").unwrap();
        assert_eq!(
            bind.script_timing_resolver.unwrap()(&[".entry", "<Key>", "puts %A"]),
            vec![(2, ScriptTiming::Deferred)]
        );

        for (widget, method, args, expected_index) in [
            ("canvas", "bind", &["item", "<Key>", "puts %A"][..], 2),
            ("text", "tag", &["b", "warning", "<Key>", "puts %A"][..], 3),
            (
                "ttk::treeview",
                "tag",
                &["bind", "warning", "<Key>", "puts %A"][..],
                3,
            ),
        ] {
            let method = specs
                .iter()
                .find(|spec| spec.name == widget)
                .unwrap()
                .resolve_subcommand(method)
                .unwrap();
            assert_eq!(
                method.script_timing_resolver.unwrap()(args),
                vec![(expected_index, ScriptTiming::Deferred)],
                "{widget} {method:?}"
            );
            assert!(
                method
                    .callback_taint_inputs
                    .iter()
                    .any(|(index, inputs)| *index == expected_index && !inputs.is_empty()),
                "{widget} must attach event input taint to the callback"
            );
        }
    }
}
