//! `cmdline::getoptions` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getoptions",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Parse all command-line options according to a specification.",
            &["cmdline::getoptions argvVar optlist ?usage?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
