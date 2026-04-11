//! `cmdline::getKnownOptions` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getKnownOptions",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Parse all known command-line options according to a specification.",
            &["cmdline::getKnownOptions argvVar optlist ?usage?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
