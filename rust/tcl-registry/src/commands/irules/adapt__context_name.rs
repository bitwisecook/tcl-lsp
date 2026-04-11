//! `ADAPT::context_name` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the name of a dynamic adaptation context.",
            &["ADAPT::context_name ADAPT_CTX"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
