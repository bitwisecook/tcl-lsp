//! `ADAPT::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enables, disables or returns the enable state.",
            &["ADAPT::enable (ADAPT_CTX)? (ADAPT_SIDE)? (BOOLEAN)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
