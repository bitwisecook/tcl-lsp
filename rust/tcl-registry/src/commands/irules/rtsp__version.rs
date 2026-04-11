//! `RTSP::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the version in the current RTSP request/response.",
            &["RTSP::version"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
