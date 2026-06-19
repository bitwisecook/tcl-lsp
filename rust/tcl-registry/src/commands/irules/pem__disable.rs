//! `PEM::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "PEM iRule command to disable PEM feature on current flow.",
            synopsis: &["PEM::disable"],
            snippet: "Disable PEM for the current flow. Note that the config must already contain a Policy Enforcement Profile.",
            source: "https://clouddocs.f5.com/api/irules/PEM__disable.html",
            examples: "when HTTP_REQUEST {\n    PEM::disable;\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PEM::disable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
