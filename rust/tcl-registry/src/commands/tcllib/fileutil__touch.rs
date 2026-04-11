//! `fileutil::touch` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::touch",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Update file access/modification times.",
            &["fileutil::touch ?options? filename ..."],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
