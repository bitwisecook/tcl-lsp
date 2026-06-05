//! `RTSP::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Collects the amount of data that you specify.",
            synopsis: &["RTSP::collect (LENGTH)?"],
            snippet: "Collects the amount of data that you specify.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__collect.html",
            examples: "when RTSP_REQUEST {\n        RTSP::collect 10\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "RTSP::collect (LENGTH)?",
        }],
        ..CommandSpec::DEFAULT
    }
}
