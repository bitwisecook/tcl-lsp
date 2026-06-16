//! `tcltest::loadTestedCommands` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::loadTestedCommands",
        dialects: None,
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Evaluate the ``-load`` or ``-loadfile`` script to load commands under test.",
            synopsis: &["tcltest::loadTestedCommands"],
            snippet: "",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        ..CommandSpec::DEFAULT
    }
}
