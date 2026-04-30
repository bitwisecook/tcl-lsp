//! `cmdline::getKnownOpt` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getKnownOpt",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet::brief(
            "Parse a single known command-line option.",
            &["cmdline::getKnownOpt argvVar optstring optVar valVar"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
