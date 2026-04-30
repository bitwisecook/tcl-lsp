//! `RTSP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Queries for or replaces content information.",
            &["RTSP::payload (LENGTH | length)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
