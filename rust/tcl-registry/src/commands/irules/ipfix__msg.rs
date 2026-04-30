//! `IPFIX::msg` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IPFIX::msg",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "IPFIX::msg Provides the ability to create, delete and set values in an IPFIX mes",
            &["IPFIX::msg ((create IPFIX_TEMPLATE) |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
