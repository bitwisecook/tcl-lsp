//! `LB::context_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::context_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Assign the current connection to a named context.",
            &["LB::context_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
