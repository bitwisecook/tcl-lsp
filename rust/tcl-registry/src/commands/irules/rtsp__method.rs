//! `RTSP::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns a method/command from the current RTSP request.",
            &["RTSP::method"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
