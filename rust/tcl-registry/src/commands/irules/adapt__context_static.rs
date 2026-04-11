//! `ADAPT::context_static` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_static",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets the static context.",
            &["ADAPT::context_static (ADAPT_SIDE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
