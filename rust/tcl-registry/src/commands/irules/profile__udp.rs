//! `PROFILE::udp` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::udp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of a UDP profile setting.",
            synopsis: &["PROFILE::udp ATTR"],
            snippet: "Returns the current value of the specified setting in an assigned UDP profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__udp.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in an assigned UDP profile.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROFILE::udp ATTR",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::BigipConfig,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
