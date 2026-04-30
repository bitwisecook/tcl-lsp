//! `subst` — perform Tcl substitutions on a string.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "subst",
        traits: Traits::TAINT_SINK | Traits::IS_UNESCAPE | Traits::PERFORMS_SUBSTITUTION,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Perform backslash, command, and variable substitutions.",
            &["subst ?-nobackslashes? ?-nocommands? ?-novariables? string"],
            "Tcl subst(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
