//! `binary` — manipulate binary data.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary format formatString ?arg arg ...?",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary scan string formatString ?varName varName ...?",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary encode format ?-option value ...? data",
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "binary decode format ?-option value ...? data",
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "decode",
        arity: Arity::at_least(2),
        detail: "Decode binary data.",
        synopsis: "binary decode format data",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "encode",
        arity: Arity::at_least(2),
        detail: "Encode binary data.",
        synopsis: "binary encode format data",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "format",
        arity: Arity::at_least(1),
        detail: "Format values into a binary string.",
        synopsis: "binary format formatString ?arg ...?",
        pure: true,
        return_type: Some(TclType::ByteArray),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(2),
        detail: "Parse a binary string.",
        synopsis: "binary scan string formatString ?varName ...?",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "binary",
        traits: Traits::BYTE_COMPILED | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
hover: Some(HoverSnippet {
    summary: "Manipulate binary data",
    synopsis: &["binary format formatString ?arg arg ...?", "binary scan string formatString ?varName varName ...?", "binary encode format ?-option value ...? data", "binary decode format ?-option value ...? data", "binary subcommand ?arg ...?"],
    snippet: "This command provides facilities for manipulating binary data. The principal operations are inserting values into a binary string and extracting values from a binary string.",
    source: "Tcl man page binary.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
