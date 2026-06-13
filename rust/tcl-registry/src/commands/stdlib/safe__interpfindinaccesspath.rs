//! `safe::interpFindInAccessPath` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpFindInAccessPath",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Return the token for a directory in a safe interpreter's access path.",
            synopsis: &["safe::interpFindInAccessPath child directory"],
            snippet: "",
            source: "Tcl stdlib Safe Base",
            examples: "",
            return_value: "",
        }),
        required_package: Some("safe"),
        ..CommandSpec::DEFAULT
    }
}
