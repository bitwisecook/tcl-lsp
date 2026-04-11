//! `RTSP::msg_source` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::msg_source",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Indicates whether the request or response originated from the client or the serv",
            &["RTSP::msg_source"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
