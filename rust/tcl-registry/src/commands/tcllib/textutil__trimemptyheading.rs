//! `textutil::trimEmptyHeading` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trimEmptyHeading",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Remove leading empty lines.",
            &["textutil::trimEmptyHeading text"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
