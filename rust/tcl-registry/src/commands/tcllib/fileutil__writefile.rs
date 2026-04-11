//! `fileutil::writeFile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::writeFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet::brief(
            "Write data to a file, replacing any existing content.",
            &["fileutil::writeFile ?-encoding enc? ?--? file data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
