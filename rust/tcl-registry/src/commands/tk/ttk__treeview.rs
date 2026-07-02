//! `ttk::treeview` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-columns",
        takes_value: true,
        value_hint: "columnList",
        detail: "List of column identifiers.",
        dialects: None,
    },
    OptionSpec {
        name: "-displaycolumns",
        takes_value: true,
        value_hint: "columnList",
        detail: "List of columns to display, or #all.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "rows",
        detail: "Number of rows to display.",
        dialects: None,
    },
    OptionSpec {
        name: "-padding",
        takes_value: true,
        value_hint: "padSpec",
        detail: "Internal padding around the widget content.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectmode",
        takes_value: true,
        value_hint: "mode",
        detail: "Selection mode (extended, browse, or none).",
        dialects: None,
    },
    OptionSpec {
        name: "-show",
        takes_value: true,
        value_hint: "components",
        detail: "Which parts of the treeview to display (tree, headings, or both).",
        dialects: None,
    },
    OptionSpec {
        name: "-xscrollcommand",
        takes_value: true,
        value_hint: "script",
        detail: "Command prefix for horizontal scroll communication.",
        dialects: None,
    },
    OptionSpec {
        name: "-yscrollcommand",
        takes_value: true,
        value_hint: "script",
        detail: "Command prefix for vertical scroll communication.",
        dialects: None,
    },
    OptionSpec {
        name: "-style",
        takes_value: true,
        value_hint: "style",
        detail: "Style to use for the widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-class",
        takes_value: true,
        value_hint: "className",
        detail: "Widget class name for option-database lookups.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "cursor",
        detail: "Cursor to display when the pointer is over the widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "focusSpec",
        detail: "Whether the widget accepts focus during keyboard traversal.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ttk::treeview pathName ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::treeview",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed hierarchical multicolumn data display widget.",
            synopsis: &["ttk::treeview pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_treeview.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
