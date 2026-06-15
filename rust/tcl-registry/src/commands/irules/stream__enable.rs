//! `STREAM::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables the stream filter for the life of the current TCP connection or until disabled.",
            synopsis: &["STREAM::enable"],
            snippet: "Enables the stream filter for the life of the current TCP connection or\nuntil disabled with STREAM::disable.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__enable.html",
            examples: "# This section only logs matches, and should be removed before using the rule in production.\nwhen STREAM_MATCHED {\n    log local0. \"Matched: [STREAM::match]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "STREAM::enable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
