//! `fileutil::stripPwd` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::stripPwd",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Remove the current directory prefix from a path.",
            &["fileutil::stripPwd path"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
