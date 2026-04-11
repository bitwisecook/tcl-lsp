//! `uri::geturl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::geturl",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Fetch the contents of a URI.",
            &["uri::geturl url ?options...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
