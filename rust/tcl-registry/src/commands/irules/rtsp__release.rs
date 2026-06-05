//! `RTSP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Releases the collected data.",
            synopsis: &["RTSP::release"],
            snippet: "Releases the collected data. Unless a subsequent RTSP::collect command\nwas issued, there is no need to use the RTSP::release command inside of\nthe RTSP_REQUEST_DATA and RTSP_RESPONSE_DATA events, since in these\ncases, the data is implicitly released.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__release.html",
            examples: "when RTSP_REQUEST {\n        RTSP::collect\n    }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RTSP::release" },
        ],
        ..CommandSpec::DEFAULT
    }
}
