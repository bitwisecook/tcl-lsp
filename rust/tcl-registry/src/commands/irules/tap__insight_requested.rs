//! `TAP::insight_requested` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::insight_requested",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "TAP requests insight flag.",
            synopsis: &["TAP::insight_requested"],
            snippet: "Command for indication that TAP module wants to get insight from any module. TAP::insight must be invoked if flag is set to true.",
            source: "https://clouddocs.f5.com/api/irules/TAP__insight_requested.html",
            examples: "",
            return_value: "Return boolean value if TAP modules wants to get an insight from any module.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TAP::insight_requested",
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
