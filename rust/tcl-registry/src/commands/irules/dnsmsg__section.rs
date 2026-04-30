//! `DNSMSG::section` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNSMSG::section",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a section of a dns_message.",
            &["DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
