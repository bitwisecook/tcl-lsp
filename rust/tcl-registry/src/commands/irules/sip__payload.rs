//! `SIP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the accumulated SIP data content.",
            &["SIP::payload (LENGTH | (OFFSET LENGTH))?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
