//! `IPFIX::template` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IPFIX::template",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "IPFIX::template Provides the ability to create and delete IPFIX message template",
            &["IPFIX::template ( (create TEMPLATE_STRING) |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
