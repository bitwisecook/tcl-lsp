//! `encoding` — manipulate character encodings.
use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "convertfrom",
        arity: Arity::new(1, 2),
        detail: "Convert from specified encoding.",
        synopsis: "encoding convertfrom ?encoding? data",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "convertto",
        arity: Arity::new(1, 2),
        detail: "Convert to specified encoding.",
        synopsis: "encoding convertto ?encoding? string",
        pure: true,
        return_type: Some(TclType::ByteArray),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dirs",
        arity: Arity::any(),
        detail: "Manage encoding search path.",
        synopsis: "encoding dirs ?directoryList?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return list of available encodings.",
        synopsis: "encoding names",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "system",
        arity: Arity::new(0, 1),
        detail: "Get or set system encoding.",
        synopsis: "encoding system ?encoding?",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "encoding",
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet::brief(
            "Manipulate character encodings.",
            &["encoding subcommand ?arg ...?"],
            "Tcl encoding(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
