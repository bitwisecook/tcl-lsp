//! `msgcat::mcpackageconfig` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcpackageconfig",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Get or set per-package configuration options.",
            &["msgcat::mcpackageconfig subcommand option ?value?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
