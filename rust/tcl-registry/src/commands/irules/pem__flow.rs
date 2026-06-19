//! `PEM::flow` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::flow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "PEM iRule command for flow features, including transacitonal and eval.",
            synopsis: &["PEM::flow transactional disable", "PEM::flow eval"],
            snippet: "The transciontal disable command disables the transactional feature in PEM for a flow.\nThe eval command trigers the policy evaluation for the flow immediately.",
            source: "https://clouddocs.f5.com/api/irules/PEM__flow.html",
            examples: "when HTTP_REQUEST {\n    PEM::flow transactional disable;\n    PEM::flow eval;\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PEM::flow transactional disable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
