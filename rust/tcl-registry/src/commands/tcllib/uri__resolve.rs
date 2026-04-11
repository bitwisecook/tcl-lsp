//! `uri::resolve` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::resolve",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Resolve a relative URI reference against a base URI.",
            &["uri::resolve base url"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
