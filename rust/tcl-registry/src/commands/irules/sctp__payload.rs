//! `SCTP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns or replaces SCTP data content.",
            &["SCTP::payload ( (PAYLOAD_LENGTH) |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
