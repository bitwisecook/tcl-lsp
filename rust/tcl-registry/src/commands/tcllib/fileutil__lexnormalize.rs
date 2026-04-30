//! `fileutil::lexnormalize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::lexnormalize",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Lexically normalise a path.",
            &["fileutil::lexnormalize path"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
