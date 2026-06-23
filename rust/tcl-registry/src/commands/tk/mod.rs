//! `tk` command specifications.

#![allow(non_snake_case)]

mod bell;
mod bind;
mod button;
mod canvas;
mod checkbutton;
mod clipboard;
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
mod winfo;
mod wm;

use crate::spec::CommandSpec;

/// Return all `tk` command specifications.
#[must_use]
pub fn tk_command_specs() -> Vec<CommandSpec> {
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
}
