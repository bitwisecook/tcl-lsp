//! `SIP::discard` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::discard",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Discards the current SIP message.",
            &["SIP::discard"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
