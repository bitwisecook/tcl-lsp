//! `CLASSIFY::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFY::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables the classification of the flow.",
            synopsis: &["CLASSIFY::disable"],
            snippet: "Disables the classification of the flow",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFY__disable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CLASSIFY::disable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ClassificationState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
