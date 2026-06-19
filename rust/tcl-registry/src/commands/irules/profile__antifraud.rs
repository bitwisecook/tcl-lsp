//! `PROFILE::antifraud` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::antifraud",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of a ANTIFRAUD profile setting.",
            synopsis: &["PROFILE::antifraud ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned ANTIFRAUD profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__antifraud.html",
            examples: "",
            return_value: "Returns the current value of the specified setting in the assigned ANTIFRAUD profile.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROFILE::antifraud ATTR",
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
