//! `PROFILE::auth` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::auth",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of an authentication profile setting.",
            synopsis: &["PROFILE::auth PROFILE_AUTH ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned authentication profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__auth.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in the assigned authentication profile.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROFILE::auth PROFILE_AUTH ATTR",
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
