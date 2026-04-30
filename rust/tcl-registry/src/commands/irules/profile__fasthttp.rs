//! `PROFILE::fasthttp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::fasthttp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the value of a Fast HTTP profile setting.",
            &["PROFILE::fasthttp ATTR"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
