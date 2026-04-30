//! `tcltest::loadTestedCommands` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::loadTestedCommands",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Evaluate the ``-load`` or ``-loadfile`` script to load commands under test.",
            &["tcltest::loadTestedCommands"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
