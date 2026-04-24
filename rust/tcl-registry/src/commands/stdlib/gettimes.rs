//! `gettimes` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "gettimes",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get timing information for performance testing.",
            &["gettimes"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
