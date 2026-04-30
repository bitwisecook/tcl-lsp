//! `ADAPT::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets or returns the adaptation result code.",
            &["ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
