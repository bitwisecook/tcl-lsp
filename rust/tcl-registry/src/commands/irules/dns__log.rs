//! `DNS::log` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Controls log publisher in DNS log profile.",
            &["DNS::log (MESSAGE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
