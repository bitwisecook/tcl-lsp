//! `PSC::calling_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::calling_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set calling id.",
            &["PSC::calling_id (CALLING_ID)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
