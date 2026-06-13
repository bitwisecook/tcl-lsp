//! `PROFILE::webacceleration` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::webacceleration",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of an web acceleration profile setting.",
            synopsis: &["PROFILE::webacceleration ATTR"],
            snippet: "Returns the value of an web acceleration profile setting",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__webacceleration.html",
            examples: "",
            return_value: "Returns the value of an web acceleration profile setting",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROFILE::webacceleration ATTR",
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
