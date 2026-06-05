//! `DHCPv4::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command drops DHCPv4 message silently.",
            synopsis: &["DHCPv4::drop"],
            snippet:
                "This command drops DHCPv4 message silently\n\nDetails (syntax):\nDHCPv4::drop",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__drop.html",
            examples: "when CLIENT_DATA {\n        DHCPv4::drop\n    }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DHCPv4::drop",
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
