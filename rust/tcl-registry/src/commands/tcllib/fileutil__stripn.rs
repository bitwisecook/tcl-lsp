//! `fileutil::stripN` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::stripN",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Remove N leading path components.",
            &["fileutil::stripN path n"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
