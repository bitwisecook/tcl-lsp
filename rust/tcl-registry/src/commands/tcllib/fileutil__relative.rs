//! `fileutil::relative` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::relative",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Compute a relative path.",
            &["fileutil::relative base dst"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
