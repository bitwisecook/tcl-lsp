//! `DOSL7::health` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::health",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns DOSL7 server health value for current virtual server",
            synopsis: &["DOSL7::health"],
            snippet: "Syntax\n\nDOSL7::health",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__health.html",
            examples: "",
            return_value: "Return health value as integer. Lower values are good health. Bad health is value > 1^23.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DOSL7::health",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Dosl7State,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
