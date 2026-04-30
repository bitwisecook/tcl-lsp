//! `cmdline::typedGetopt` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::typedGetopt",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet::brief(
            "Parse a single typed command-line option.",
            &["cmdline::typedGetopt argvVar optstring optVar valVar"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
