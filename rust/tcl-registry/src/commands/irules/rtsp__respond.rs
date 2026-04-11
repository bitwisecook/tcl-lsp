//! `RTSP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends an RTSP response to the client.",
            &["RTSP::respond STATUS_CODE STATUS_STRING (HEADERS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
