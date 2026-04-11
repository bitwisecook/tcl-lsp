//! `cmdline::getfiles` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getfiles",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Expand file patterns into a list of matching files.",
            &["cmdline::getfiles patterns quiet"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
