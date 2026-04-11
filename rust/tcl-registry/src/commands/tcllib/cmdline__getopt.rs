//! `cmdline::getopt` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getopt",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet::brief(
            "Parse a single command-line option.",
            &["cmdline::getopt argvVar optstring optVar valVar"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
