//! `pack` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Set or query the packing options for one or more slaves.",
        synopsis: "pack configure slave ?slave ...? ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::at_least(1),
        detail: "Remove each slave from the packing order for its master.",
        synopsis: "pack forget slave ?slave ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(1),
        detail: "Return a list of the current configuration state of the slave.",
        synopsis: "pack info slave",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "propagate",
        arity: Arity::new(1, 2),
        detail: "Control whether the master computes its geometry from slaves.",
        synopsis: "pack propagate master ?boolean?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "slaves",
        arity: Arity::exact(1),
        detail: "Return a list of all slaves in the packing order for the master.",
        synopsis: "pack slaves master",
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
    OptionSpec { name: "-side", takes_value: true, value_hint: "top|bottom|left|right", detail: "Specifies which side of the master the slave will be packed against.", dialects: None },
    OptionSpec { name: "-fill", takes_value: true, value_hint: "none|x|y|both", detail: "If a slave's parcel is larger than its requested dimensions, this option may be used to stretch the slave.", dialects: None },
    OptionSpec { name: "-expand", takes_value: true, value_hint: "boolean", detail: "Specifies whether the slave should be expanded to consume extra space in its master.", dialects: None },
    OptionSpec { name: "-anchor", takes_value: true, value_hint: "n|ne|e|se|s|sw|w|nw|center", detail: "Anchor must be a valid anchor position: n, ne, e, se, s, sw, w, nw, or center.", dialects: None },
    OptionSpec { name: "-padx", takes_value: true, value_hint: "amount", detail: "Specifies how much external horizontal padding to leave on each side of the slave.", dialects: None },
    OptionSpec { name: "-pady", takes_value: true, value_hint: "amount", detail: "Specifies how much external vertical padding to leave on each side of the slave.", dialects: None },
    OptionSpec { name: "-ipadx", takes_value: true, value_hint: "amount", detail: "Specifies how much internal horizontal padding to leave on each side of the slave.", dialects: None },
    OptionSpec { name: "-ipady", takes_value: true, value_hint: "amount", detail: "Specifies how much internal vertical padding to leave on each side of the slave.", dialects: None },
    OptionSpec { name: "-in", takes_value: true, value_hint: "master", detail: "Insert the slave at the end of the packing order for the master window.", dialects: None },
    OptionSpec { name: "-before", takes_value: true, value_hint: "other", detail: "Insert the slave before the window given by other in the packing order.", dialects: None },
    OptionSpec { name: "-after", takes_value: true, value_hint: "other", detail: "Insert the slave after the window given by other in the packing order.", dialects: None },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "pack option arg ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pack",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Geometry manager that packs slaves around the edges of a cavity.",
            &["pack slave ?slave ...? ?option value ...?"],
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
