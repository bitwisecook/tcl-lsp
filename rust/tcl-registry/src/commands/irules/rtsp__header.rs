//! `RTSP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Manages headers in RTSP requests and responses.",
            synopsis: &[
                "RTSP::header (exists | remove | value) HEADER_NAME",
                "RTSP::header replace HEADER_NAME HEADER_VALUE",
                "RTSP::header insert (<(HEADER_NAME HEADER_VALUE)+> |",
            ],
            snippet: "Manages headers in RTSP requests and responses.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__header.html",
            examples: "when RTSP_REQUEST {\n        puts [RTSP::header value \"x-header\"]\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "RTSP::header (exists | remove | value) HEADER_NAME",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
