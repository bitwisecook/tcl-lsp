//! `TclOO` context.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "self",
        traits: Traits::PURE | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Introspect the current `TclOO` context.",
            &["self ?subcommand?"],
            "Tcl self(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
