//! `cmdline::getArgv0` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getArgv0",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Return the application name from the command line.",
            &["cmdline::getArgv0"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
