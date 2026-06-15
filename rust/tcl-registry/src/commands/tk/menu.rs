//! `menu` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(1),
        detail: "Add a new entry to the bottom of the menu.",
        synopsis: "pathName add type ?option value ...?",
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "cascade",
                    detail: "A cascade entry that posts another menu.",
                },
                ArgValue {
                    value: "checkbutton",
                    detail: "A checkbutton entry with an on/off indicator.",
                },
                ArgValue {
                    value: "command",
                    detail: "A command entry that invokes a Tcl command.",
                },
                ArgValue {
                    value: "radiobutton",
                    detail: "A radiobutton entry with a mutual-exclusion indicator.",
                },
                ArgValue {
                    value: "separator",
                    detail: "A separator line between groups of entries.",
                },
            ],
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clone",
        arity: Arity::new(1, 2),
        detail: "Create a clone of this menu.",
        synopsis: "pathName clone newPathname ?cloneType?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::new(1, 2),
        detail: "Delete menu entries between index1 and index2 inclusive.",
        synopsis: "pathName delete index1 ?index2?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "entrycget",
        arity: Arity::exact(2),
        detail: "Return the value of a configuration option for a menu entry.",
        synopsis: "pathName entrycget index option",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "entryconfigure",
        arity: Arity::at_least(1),
        detail: "Query or modify options of a menu entry.",
        synopsis: "pathName entryconfigure index ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the numerical index corresponding to the given index.",
        synopsis: "pathName index index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Insert a new entry before the entry at the given index.",
        synopsis: "pathName insert index type ?option value ...?",
        arg_values: &[(
            1,
            &[
                ArgValue {
                    value: "cascade",
                    detail: "A cascade entry that posts another menu.",
                },
                ArgValue {
                    value: "checkbutton",
                    detail: "A checkbutton entry with an on/off indicator.",
                },
                ArgValue {
                    value: "command",
                    detail: "A command entry that invokes a Tcl command.",
                },
                ArgValue {
                    value: "radiobutton",
                    detail: "A radiobutton entry with a mutual-exclusion indicator.",
                },
                ArgValue {
                    value: "separator",
                    detail: "A separator line between groups of entries.",
                },
            ],
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "invoke",
        arity: Arity::exact(1),
        detail: "Invoke the action of the menu entry at the given index.",
        synopsis: "pathName invoke index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "post",
        arity: Arity::exact(2),
        detail: "Display the menu at the given screen coordinates.",
        synopsis: "pathName post x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "postcascade",
        arity: Arity::exact(1),
        detail: "Post the submenu associated with the cascade entry at the given index.",
        synopsis: "pathName postcascade index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Return the type of the menu entry at the given index.",
        synopsis: "pathName type index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unpost",
        arity: Arity::exact(0),
        detail: "Unmap the menu so it is no longer displayed.",
        synopsis: "pathName unpost",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yposition",
        arity: Arity::exact(1),
        detail: "Return the y-coordinate of the topmost pixel of the entry at the given index.",
        synopsis: "pathName yposition index",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-tearoff",
        takes_value: true,
        value_hint: "",
        detail: "Whether the menu should include a tear-off entry at the top.",
        dialects: None,
    },
    OptionSpec {
        name: "-title",
        takes_value: true,
        value_hint: "",
        detail: "Title string for the tear-off menu window.",
        dialects: None,
    },
    OptionSpec {
        name: "-type",
        takes_value: true,
        value_hint: "",
        detail: "Type of the menu: menubar, tearoff, or normal.",
        dialects: None,
    },
    OptionSpec {
        name: "-bg",
        takes_value: true,
        value_hint: "",
        detail: "Shorthand for -background.",
        dialects: None,
    },
    OptionSpec {
        name: "-background",
        takes_value: true,
        value_hint: "",
        detail: "Background colour of the menu.",
        dialects: None,
    },
    OptionSpec {
        name: "-fg",
        takes_value: true,
        value_hint: "",
        detail: "Shorthand for -foreground.",
        dialects: None,
    },
    OptionSpec {
        name: "-foreground",
        takes_value: true,
        value_hint: "",
        detail: "Foreground colour for menu entries.",
        dialects: None,
    },
    OptionSpec {
        name: "-font",
        takes_value: true,
        value_hint: "",
        detail: "Font to use for text in the menu.",
        dialects: None,
    },
    OptionSpec {
        name: "-activebackground",
        takes_value: true,
        value_hint: "",
        detail: "Background colour for the active menu entry.",
        dialects: None,
    },
    OptionSpec {
        name: "-activeforeground",
        takes_value: true,
        value_hint: "",
        detail: "Foreground colour for the active menu entry.",
        dialects: None,
    },
    OptionSpec {
        name: "-activeborderwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of the border drawn around active entries.",
        dialects: None,
    },
    OptionSpec {
        name: "-disabledforeground",
        takes_value: true,
        value_hint: "",
        detail: "Foreground colour for disabled menu entries.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the indicator for checkbutton and radiobutton entries.",
        dialects: None,
    },
    OptionSpec {
        name: "-relief",
        takes_value: true,
        value_hint: "",
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
    },
    OptionSpec {
        name: "-borderwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of the border around the menu.",
        dialects: None,
    },
    OptionSpec {
        name: "-postcommand",
        takes_value: true,
        value_hint: "",
        detail: "Tcl command to invoke just before the menu is posted.",
        dialects: None,
    },
    OptionSpec {
        name: "-tearoffcommand",
        takes_value: true,
        value_hint: "",
        detail: "Tcl command to invoke when the menu is torn off.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the menu.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the menu accepts focus during keyboard traversal.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "menu pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "menu",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a menu widget.",
            synopsis: &["menu pathName ?option value ...?"],
            snippet: "Displays a menu of commands, each of which may be a cascade, checkbutton, command, radiobutton, or separator entry.",
            source: "Tk man page menu.n",
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
