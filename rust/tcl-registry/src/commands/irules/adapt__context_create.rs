//! `ADAPT::context_create` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_create",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Creates a new dynamic adaptation context.",
            &["ADAPT::context_create (ADAPT_SIDE)? NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
