//! `RTSP::status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the HTTP style status code from the current RTSP response.",
            synopsis: &["RTSP::status"],
            snippet: "Returns the HTTP style status code (for example, 200 or 401) from the\ncurrent RTSP response.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__status.html",
            examples: "when RTSP_RESPONSE {\n        puts [RTSP::status]\n    }",
            return_value: "Returns the HTTP style status code from the current RTSP response.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RTSP::status" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
