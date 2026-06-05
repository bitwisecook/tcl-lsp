//! `RTSP::method` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a method/command from the current RTSP request.",
            synopsis: &["RTSP::method"],
            snippet: "Returns the method/command (for example, DESCRIBE, PLAY) from the\ncurrent RTSP request.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__method.html",
            examples: "when RTSP_REQUEST {\n        puts [RTSP::method]\n    }",
            return_value: "Returns a method/command from the current RTSP request.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RTSP::method" },
        ],
        ..CommandSpec::DEFAULT
    }
}
