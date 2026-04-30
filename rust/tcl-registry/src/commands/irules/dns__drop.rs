//! `DNS::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Drops the current DNS packet after the execution of the event.",
            &["DNS::drop"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
