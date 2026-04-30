//! `DNS::tsig` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::tsig",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Manipulates the current DNS message and its TSIG resource record.",
            &["DNS::tsig 'remove'"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
