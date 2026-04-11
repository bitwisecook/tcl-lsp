//! `RTSP::uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the complete URI of the RTSP request.",
            &["RTSP::uri (URI_STRING)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
