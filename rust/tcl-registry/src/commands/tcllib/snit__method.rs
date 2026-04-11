//! `snit::method` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::method",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet::brief(
            "Define an instance method outside a type definition body.",
            &["snit::method type name arglist body"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
