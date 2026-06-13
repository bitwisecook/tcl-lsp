//! `ECA::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables the plugin in the flow.",
            synopsis: &["ECA::enable"],
            snippet: "The ECA::enable command enables the plugin in the flow.",
            source: "https://clouddocs.f5.com/api/irules/ECA__enable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ECA::enable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
