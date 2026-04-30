//! `uri::join` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::join",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Construct a URI from component parts.",
            &["uri::join ?key value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
