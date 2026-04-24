//! `textutil::undent` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::undent",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Remove common leading whitespace from all lines.",
            &["textutil::undent text"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
