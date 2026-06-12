//! `DECOMPRESS::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DECOMPRESS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disable DECOMPRESS feature on current flow.",
            synopsis: &["DECOMPRESS::disable (request | response)?"],
            snippet: "Disable DECOMPRESS feature on current flow.",
            source: "https://clouddocs.f5.com/api/irules/DECOMPRESS__disable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DECOMPRESS::disable (request | response)?",
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
