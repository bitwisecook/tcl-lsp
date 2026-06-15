//! `CONNECTOR::profile` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CONNECTOR::profile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get connector profile name.",
            synopsis: &["CONNECTOR::profile"],
            snippet: "CONNECTOR::profile\n    Get the connector profile name in the current context.",
            source: "https://clouddocs.f5.com/api/irules/CONNECTOR__profile.html",
            examples: "when CONNECTOR_OPEN {\n                if {([CONNECTOR::profile] eq \"/Common/connector_profile_1\")} {\n                    log local0. \"CONNECTOR_OPEN raised by connector_profile_1\"\n                }\n            }",
            return_value: "CONNECTOR::profile Return the connector profile name.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "CONNECTOR::profile",
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
