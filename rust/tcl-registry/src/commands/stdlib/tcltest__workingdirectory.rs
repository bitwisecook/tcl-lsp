//! `tcltest::workingDirectory` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::workingDirectory",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Get or set the working directory for tests.",
            &["tcltest::workingDirectory ?path?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
