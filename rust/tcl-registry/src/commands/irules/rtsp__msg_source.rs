//! `RTSP::msg_source` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::msg_source",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Indicates whether the request or response originated from the client or the server.",
            synopsis: &["RTSP::msg_source"],
            snippet: "Indicates whether the request or response originated from the client or\nthe server. This command returns the string client or server.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__msg_source.html",
            examples: "when RTSP_REQUEST {\n        puts [RTSP::msg_source]\n    }",
            return_value: "Returns the string \"client\" or \"server\".",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RTSP::msg_source" },
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
