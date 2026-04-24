//! `fileutil::jail` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::jail",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Confine a path under a base directory.",
            &["fileutil::jail base path"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
