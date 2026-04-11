//! `binary` — manipulate binary data.
use crate::prelude::*;

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
        traits: Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet::brief(
            "Manipulate binary data.",
            &["binary subcommand ?arg ...?"],
            "Tcl binary(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
