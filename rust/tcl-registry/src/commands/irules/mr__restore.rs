//! `MR::restore` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::restore",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the stored variables to the current context tcl variable store.",
            &["MR::restore (VAR)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
