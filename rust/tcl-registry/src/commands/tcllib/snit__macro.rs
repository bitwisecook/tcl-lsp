//! `snit::macro` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::macro",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet::brief(
            "Define a snit macro for use in type definitions.",
            &["snit::macro name arglist body"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
