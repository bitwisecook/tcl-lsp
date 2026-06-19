//! `STREAM::match` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::match",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the matching characters.",
            synopsis: &["STREAM::match"],
            snippet: "Returns the matching characters.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__match.html",
            examples: "when HTTP_REQUEST {\n\n   # Disable the stream filter for all requests\n   STREAM::disable\n}",
            return_value: "Returns the matching characters.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "STREAM::match",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
