//! `LB::connect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::connect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "LB::connect",
            synopsis: &["LB::connect"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/LB__connect.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::connect",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Server,
        }],
        ..CommandSpec::DEFAULT
    }
}
