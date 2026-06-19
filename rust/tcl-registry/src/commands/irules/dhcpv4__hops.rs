//! `DHCPv4::hops` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::hops",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns hops (number of hops) field from DHCPv4 message.",
            synopsis: &["DHCPv4::hops"],
            snippet: "This command returns hops (number of hops) field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::hops",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__hops.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Hops [DHCPv4::hops]\"\n    }",
            return_value: "This command returns hlen (hardware len) field from DHCPv4 message",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DHCPv4::hops",
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
