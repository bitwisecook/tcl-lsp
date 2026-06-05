//! `RTSP::uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the complete URI of the RTSP request.",
            synopsis: &["RTSP::uri (URI_STRING)?"],
            snippet: "Returns the complete URI of the RTSP request.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__uri.html",
            examples: "when RTSP_REQUEST {\n        puts [RTSP::uri]\n    }",
            return_value: "Returns the complete URI of the RTSP request.",
        }),
        ..CommandSpec::DEFAULT
    }
}
