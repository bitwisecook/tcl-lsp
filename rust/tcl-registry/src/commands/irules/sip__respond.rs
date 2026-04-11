//! `SIP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Terminates a SIP response and responds with one of your creation.",
            &["SIP::respond RESPONSE_CODE (PHRASE (HEADER_NAME HEADER_VALUE)*)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
