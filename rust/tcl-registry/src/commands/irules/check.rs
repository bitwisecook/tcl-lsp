//! `check` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set the iRule validation level.",
            synopsis: &["check"],
            snippet: "Set the iRule validation level: - none: disable the validation.",
            source: "https://clouddocs.f5.com/api/irules/check.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "check",
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
