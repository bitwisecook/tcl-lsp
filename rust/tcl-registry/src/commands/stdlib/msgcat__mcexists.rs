//! `msgcat::mcexists` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mcexists",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Check whether a translation exists for the given source string.",
            &["msgcat::mcexists ?-exactnamespace? ?-exactlocale? src-string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
