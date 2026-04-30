//! `textutil::trim` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trim",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Remove leading whitespace from each line of text.",
            &["textutil::trim text ?regexp?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
