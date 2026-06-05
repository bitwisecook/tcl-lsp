//! `place` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Set or query the placement options for a window.",
        synopsis: "place configure window ?option? ?value option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "Cause the placer to stop managing the geometry of the window.",
        synopsis: "place forget window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(1),
        detail: "Return a list of the current configuration for the window.",
        synopsis: "place info window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "slaves",
        arity: Arity::exact(1),
        detail: "Return a list of all slaves managed by the placer for the window.",
        synopsis: "place slaves window",
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
        name: "-x",
        takes_value: true,
        value_hint: "location",
        detail: "Specifies the x-coordinate of the anchor point in the master window.",
        dialects: None,
    },
    OptionSpec {
        name: "-y",
        takes_value: true,
        value_hint: "location",
        detail: "Specifies the y-coordinate of the anchor point in the master window.",
        dialects: None,
    },
    OptionSpec {
        name: "-relx",
        takes_value: true,
        value_hint: "location",
        detail: "Specifies the x-coordinate as a fraction of the master width (0.0 to 1.0).",
        dialects: None,
    },
    OptionSpec {
        name: "-rely",
        takes_value: true,
        value_hint: "location",
        detail: "Specifies the y-coordinate as a fraction of the master height (0.0 to 1.0).",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "size",
        detail: "Specifies the width of the slave in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "size",
        detail: "Specifies the height of the slave in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-relwidth",
        takes_value: true,
        value_hint: "size",
        detail: "Specifies the width as a fraction of the master width (0.0 to 1.0).",
        dialects: None,
    },
    OptionSpec {
        name: "-relheight",
        takes_value: true,
        value_hint: "size",
        detail: "Specifies the height as a fraction of the master height (0.0 to 1.0).",
        dialects: None,
    },
    OptionSpec {
        name: "-anchor",
        takes_value: true,
        value_hint: "n|ne|e|se|s|sw|w|nw|center",
        detail: "Specifies which point of the slave is positioned at the (x,y) location.",
        dialects: None,
    },
    OptionSpec {
        name: "-bordermode",
        takes_value: true,
        value_hint: "inside|outside|ignore",
        detail: "Determines the degree to which borders within the master are used.",
        dialects: None,
    },
    OptionSpec {
        name: "-in",
        takes_value: true,
        value_hint: "master",
        detail: "Specifies the master window relative to which the slave is placed.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "place option arg ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "place",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Geometry manager for fixed or rubber-sheet placement.",
            &["place window option value ?option value ...?"],
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
