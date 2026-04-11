//! `textutil::indent` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::indent",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Indent each line of text by a given prefix.",
            &["textutil::indent text prefix ?skip?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
