//! `RTSP::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the version in the current RTSP request/response.",
            synopsis: &["RTSP::version"],
            snippet: "Returns the version (for example, RTSP/1.0) in the current RTSP\nrequest/response. You can use this command to determine if RTSP is\nbeing tunneled over HTTP on the RTSP port (the version would be an HTTP\nversion). The command is valid in the RTSP_REQUEST and RTSP_RESPONSE\nevents.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__version.html",
            examples: "when RTSP_REQUEST {\n        puts [RTSP::version]\n    }",
            return_value: "Returns the version in the current RTSP request/response.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RTSP::version" },
        ],
        ..CommandSpec::DEFAULT
    }
}
