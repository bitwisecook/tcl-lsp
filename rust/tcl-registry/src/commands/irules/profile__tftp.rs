//! `PROFILE::tftp` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::tftp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of an TFTP profile setting.",
            synopsis: &["PROFILE::tftp ATTR"],
            snippet:
                "Returns the current value of the specified setting in the assigned TFTP profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__tftp.html",
            examples: "",
            return_value:
                "Returns the current value of the specified setting in the assigned TFTP profile.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROFILE::tftp ATTR",
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
