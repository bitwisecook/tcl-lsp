//! `RTSP::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Collects the amount of data that you specify.",
            &["RTSP::collect (LENGTH)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
