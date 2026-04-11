//! `ADAPT::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets or returns the internal virtual server (IVS) selection.",
            &["ADAPT::select (ADAPT_CTX)? (ADAPT_SIDE)? (NAME)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
