//! `msgcat::mcutil` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcutil",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Utility subcommands for the message catalogue system.",
            &["msgcat::mcutil subcommand ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
