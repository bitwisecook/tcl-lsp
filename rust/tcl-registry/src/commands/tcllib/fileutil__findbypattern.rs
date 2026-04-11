//! `fileutil::findByPattern` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::findByPattern",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Find files matching glob or regexp patterns.",
            &["fileutil::findByPattern basedir ?-glob|-regexp? ?--? patterns"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
