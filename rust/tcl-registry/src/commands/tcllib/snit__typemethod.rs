//! `snit::typemethod` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::typemethod",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet::brief(
            "Define a type method outside a type definition body.",
            &["snit::typemethod type name arglist body"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
