//! `grid` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "anchor",
        arity: Arity::new(1, 2),
        detail: "Set the anchor point for the grid within the master window.",
        synopsis: "grid anchor master ?anchor?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bbox",
        arity: Arity::new(1, 5),
        detail: "Return the bounding box of a cell or group of cells.",
        synopsis: "grid bbox master ?column row? ?column2 row2?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "columnconfigure",
        arity: Arity::at_least(2),
        detail: "Query or set column properties of the grid.",
        synopsis: "grid columnconfigure master index ?-option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Set or query the grid options for one or more slaves.",
        synopsis: "grid configure slave ?slave ...? ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::at_least(1),
        detail: "Remove each slave from the grid for its master.",
        synopsis: "grid forget slave ?slave ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(1),
        detail: "Return a list of the current grid configuration for the slave.",
        synopsis: "grid info slave",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "location",
        arity: Arity::exact(3),
        detail: "Return the column and row containing the screen point x, y.",
        synopsis: "grid location master x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "propagate",
        arity: Arity::new(1, 2),
        detail: "Control whether the master computes its geometry from slaves.",
        synopsis: "grid propagate master ?boolean?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::at_least(1),
        detail: "Remove each slave from the grid, but remember its configuration.",
        synopsis: "grid remove slave ?slave ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rowconfigure",
        arity: Arity::at_least(2),
        detail: "Query or set row properties of the grid.",
        synopsis: "grid rowconfigure master index ?-option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(1),
        detail: "Return the size of the grid as a list of two elements (columns, rows).",
        synopsis: "grid size master",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "slaves",
        arity: Arity::at_least(1),
        detail: "Return a list of all slaves in the grid for the master.",
        synopsis: "grid slaves master ?-option value?",
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
        name: "-row",
        takes_value: true,
        value_hint: "n",
        detail: "Insert the slave so that it occupies the nth row in the grid.",
        dialects: None,
    },
    OptionSpec {
        name: "-column",
        takes_value: true,
        value_hint: "n",
        detail: "Insert the slave so that it occupies the nth column in the grid.",
        dialects: None,
    },
    OptionSpec {
        name: "-rowspan",
        takes_value: true,
        value_hint: "n",
        detail: "Insert the slave so that it occupies n rows in the grid.",
        dialects: None,
    },
    OptionSpec {
        name: "-columnspan",
        takes_value: true,
        value_hint: "n",
        detail: "Insert the slave so that it occupies n columns in the grid.",
        dialects: None,
    },
    OptionSpec {
        name: "-sticky",
        takes_value: true,
        value_hint: "nsew",
        detail:
            "Specifies which edges of the cell the slave sticks to (combination of n, s, e, w).",
        dialects: None,
    },
    OptionSpec {
        name: "-padx",
        takes_value: true,
        value_hint: "amount",
        detail: "Specifies external horizontal padding for the slave.",
        dialects: None,
    },
    OptionSpec {
        name: "-pady",
        takes_value: true,
        value_hint: "amount",
        detail: "Specifies external vertical padding for the slave.",
        dialects: None,
    },
    OptionSpec {
        name: "-ipadx",
        takes_value: true,
        value_hint: "amount",
        detail: "Specifies internal horizontal padding for the slave.",
        dialects: None,
    },
    OptionSpec {
        name: "-ipady",
        takes_value: true,
        value_hint: "amount",
        detail: "Specifies internal vertical padding for the slave.",
        dialects: None,
    },
    OptionSpec {
        name: "-in",
        takes_value: true,
        value_hint: "master",
        detail: "Insert the slave into the specified master window.",
        dialects: None,
    },
    OptionSpec {
        name: "-weight",
        takes_value: true,
        value_hint: "int",
        detail: "Relative weight for apportioning extra space (columnconfigure/rowconfigure).",
        dialects: None,
    },
    OptionSpec {
        name: "-minsize",
        takes_value: true,
        value_hint: "amount",
        detail: "Minimum size of the column or row (columnconfigure/rowconfigure).",
        dialects: None,
    },
    OptionSpec {
        name: "-pad",
        takes_value: true,
        value_hint: "amount",
        detail: "Extra padding for the largest slave (columnconfigure/rowconfigure).",
        dialects: None,
    },
    OptionSpec {
        name: "-uniform",
        takes_value: true,
        value_hint: "group",
        detail: "Group columns/rows for uniform sizing (columnconfigure/rowconfigure).",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "grid option arg ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "grid",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Geometry manager that arranges widgets in a grid.",
            &["grid slave ?slave ...? ?option value ...?"],
            "F5",
        )),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
