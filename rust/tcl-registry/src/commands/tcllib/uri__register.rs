//! `uri::register` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::register",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Register a new URI scheme handler.",
            &["uri::register schemeList script"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
