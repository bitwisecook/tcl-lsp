//! `safe::interpFindInAccessPath` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpFindInAccessPath",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Return the token for a directory in a safe interpreter's access path.",
            &["safe::interpFindInAccessPath child directory"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
