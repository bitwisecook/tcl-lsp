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
mod ttk__notebook;
mod ttk__progressbar;
mod ttk__scale;
mod ttk__separator;
mod ttk__sizegrip;
mod ttk__style;
mod ttk__treeview;
mod ttk_extra;
mod winfo;
mod wm;

use crate::spec::CommandSpec;

/// Return all `tk` command specifications.
#[must_use]
pub fn tk_command_specs() -> Vec<CommandSpec> {
    let mut specs = tk_command_specs_raw();
    specs.extend(ttk_extra::specs());
    specs.extend(tk_extra_cmds::specs());
    specs.extend(console::specs());
    // The themed-widget set (`ttk::*`) was introduced with Tk 8.5.  Stamp the
    // package `min_version` here rather than in every ttk spec file so the
    // gate stays in one place.
    for spec in &mut specs {
        if spec.min_version.is_none() && spec.name.starts_with("ttk::") {
            spec.min_version = Some("8.5");
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
        ttk__notebook::spec(),
        ttk__progressbar::spec(),
        ttk__scale::spec(),
        ttk__separator::spec(),
        ttk__sizegrip::spec(),
        ttk__style::spec(),
        ttk__treeview::spec(),
        winfo::spec(),
        wm::spec(),
    ]
}

#[cfg(test)]
mod tests {
    use super::tk_command_specs;

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
            "ttk::menubutton",
            "ttk::panedwindow",
            "ttk::radiobutton",
            "ttk::spinbox",
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
        assert_eq!(spin.min_version, Some("8.5"));
    }

    #[test]
    fn console_and_consoleinterp_eval_bodies_are_registered_correctly() {
        use crate::{ArgRole, Arity};
        // Issue #925: `console eval` / `consoleinterp eval` / `consoleinterp
        // record` each take exactly one script argument that must resolve as
        // `ArgRole::Body` (so the LSP recurses into it) and must be listed as
        // a cross-interpreter eval sink (T105) — same shape as `interp eval`.
        // Stable across Tk 8.4-9.0 (no `min_version` gate on any subcommand).
        let specs = tk_command_specs();

        let console = specs.iter().find(|s| s.name == "console").unwrap();
        assert_eq!(console.min_version, None);
        assert_eq!(console.taint_interp_eval_subcommands, &["eval"]);
        let console_eval = console.resolve_subcommand("eval").unwrap();
        assert_eq!(console_eval.arity, Arity::exact(1));
        assert_eq!(console_eval.arg_role_at(0), Some(ArgRole::Body));
        for name in ["hide", "show", "title"] {
            let sub = console.resolve_subcommand(name).unwrap();
            assert_eq!(sub.arg_role_at(0), None, "`console {name}` has no body arg");
        }

        let consoleinterp = specs.iter().find(|s| s.name == "consoleinterp").unwrap();
        assert_eq!(consoleinterp.min_version, None);
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
        // `Arity::at_least(1)` and no `ArgRole::Body`. Note: unlike
        // `console`/`consoleinterp`, this is a data-correctness fix only —
        // the LSP has no widget-path → widget-type tracking, so a real
        // `.w instate ...` call never resolves to this SubCommand table
        // (only a literal `ttk::treeview instate ...`/`ttk::notebook instate
        // ...` head would); it's registered for when/if that resolution is
        // added.
        use crate::{ArgRole, Arity};
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
        }
    }

    #[test]
    fn ttk_widgets_are_gated_to_tk_85() {
        let specs = tk_command_specs();
        let ttk_button = specs.iter().find(|s| s.name == "ttk::button").unwrap();
        assert_eq!(ttk_button.min_version, Some("8.5"));
        // Available floor >= 8.5 -> present; older -> absent.
        assert!(ttk_button.available_for_version(Some("8.5")));
        assert!(ttk_button.available_for_version(Some("9.0")));
        assert!(!ttk_button.available_for_version(Some("8.4")));
        // A plain widget carries no package min_version.
        let button = specs.iter().find(|s| s.name == "button").unwrap();
        assert_eq!(button.min_version, None);
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
}
