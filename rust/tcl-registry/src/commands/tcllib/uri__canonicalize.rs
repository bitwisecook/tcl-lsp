//! `uri::canonicalize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::canonicalize",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Canonicalize a URI.",
            &["uri::canonicalize uri"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
