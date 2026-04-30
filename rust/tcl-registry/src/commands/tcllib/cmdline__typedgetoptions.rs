//! `cmdline::typedGetoptions` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::typedGetoptions",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Parse all typed command-line options according to a specification.",
            &["cmdline::typedGetoptions argvVar optlist ?usage?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
