//! `RTSP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "RTSP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Queries for or replaces content information.",
            synopsis: &["RTSP::payload (LENGTH | length)?", "RTSP::payload replace OFFSET LENGTH RTSP_PAYLOAD"],
            snippet: "Queries for or replaces content information. With this command, you can\nretrieve content, query for content size, or replace a certain amount\nof content.",
            source: "https://clouddocs.f5.com/api/irules/RTSP__payload.html",
            examples: "when RTSP_REQUEST {\n        RTSP::collect\n    }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "RTSP::payload (LENGTH | length)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
