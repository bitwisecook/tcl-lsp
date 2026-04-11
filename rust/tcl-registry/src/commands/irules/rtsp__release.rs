//! `RTSP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Releases the collected data.",
            &["RTSP::release"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
