//! `RTSP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Manages headers in RTSP requests and responses.",
            &["RTSP::header (exists | remove | value) HEADER_NAME"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
