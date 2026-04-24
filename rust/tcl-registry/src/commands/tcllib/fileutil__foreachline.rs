//! `fileutil::foreachLine` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::foreachLine",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Iterate over each line of a file.",
            &["fileutil::foreachLine var filename cmd"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
