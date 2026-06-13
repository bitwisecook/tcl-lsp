//! `TAP::config` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::config",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary:
                "Returns the current value of the specified setting in an assigned TAP Application.",
            synopsis: &["TAP::config APPLICATION ENTITY"],
            snippet:
                "Returns the current value of the specified setting in an assigned TAP Application.",
            source: "https://clouddocs.f5.com/api/irules/TAP__config.html",
            examples: "",
            return_value:
                "Returns the current value of the specified setting in an assigned TAP Application.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TAP::config APPLICATION ENTITY",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
