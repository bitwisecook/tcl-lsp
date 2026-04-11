//! `ADAPT::allow` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::allow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets or returns the value of a boolean property.",
            &["ADAPT::allow (ADAPT_CTX)? ('http_v1.0') (ADAPT_SIDE)? (BOOLEAN)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
