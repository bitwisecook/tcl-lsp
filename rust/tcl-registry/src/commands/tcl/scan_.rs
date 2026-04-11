//! `scan` — parse a string using scanf-style conversion.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "scan",
        arity: Arity::at_least(2),
        arg_roles: &[
            (2, ArgRole::VarWrite),
            (3, ArgRole::VarWrite),
            (4, ArgRole::VarWrite),
            (5, ArgRole::VarWrite),
        ],
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet::brief(
            "Parse a string using scanf-style conversion.",
            &["scan string format ?varName ...?"],
            "Tcl scan(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
