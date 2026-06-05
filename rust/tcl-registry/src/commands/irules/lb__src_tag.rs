//! `LB::src_tag` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::src_tag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the source tag for the current request.",
            synopsis: &["LB::src_tag"],
            snippet: "Sets the source tag for the current request",
            source: "https://clouddocs.f5.com/api/irules/LB__src_tag.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::src_tag",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
        }],
        ..CommandSpec::DEFAULT
    }
}
