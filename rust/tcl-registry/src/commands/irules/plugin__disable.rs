//! `PLUGIN::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PLUGIN::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::any(),
        hover: Some(HoverSnippet {
            summary: "Deprecated: removed",
            synopsis: &[],
            snippet: "",
            source: "",
            examples: "",
            return_value: "",
        }),
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        deprecated_replacement: Some("(removed)"),
        ..CommandSpec::DEFAULT
    }
}
