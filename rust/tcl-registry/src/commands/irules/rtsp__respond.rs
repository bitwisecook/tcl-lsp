//! `RTSP::respond` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sends an RTSP response to the client.",
            synopsis: &["RTSP::respond STATUS_CODE STATUS_STRING (HEADERS)?"],
            snippet: "Sends an RTSP response to the client. The return value of the\nRTSP::msg_source command must be client. When an iRule responds to an\nRTSP request, the RTSP filter performs no further processing on the\nrequest and will not send the RTSP request to the server.\nA maximum of one response is allowed per RTSP request.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__respond.html",
            examples: "when RTSP_REQUEST {\n        RTSP::respond 401 Unauthorized \"x-header\\r\\n\\r\\n  Hey, you need a password\"\n    }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RTSP::respond STATUS_CODE STATUS_STRING (HEADERS)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
