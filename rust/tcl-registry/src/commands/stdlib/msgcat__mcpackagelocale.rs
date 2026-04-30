//! `msgcat::mcpackagelocale` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcpackagelocale",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Get, set, or manage the locale for the calling package.",
            &["msgcat::mcpackagelocale subcommand ?locale?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
