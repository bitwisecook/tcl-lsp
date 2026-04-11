//! `textutil::trimright` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trimright",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Trim trailing characters from each line.",
            &["textutil::trimright text ?regexp?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
