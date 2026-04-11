//! `safe::interpAddToAccessPath` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpAddToAccessPath",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Add a directory to a safe interpreter's access path.",
            &["safe::interpAddToAccessPath child directory"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
