//! `DHCPv4::xid` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::xid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns xid(transaction ID) field from DHCPv4 message.",
            synopsis: &["DHCPv4::xid"],
            snippet: "This command returns xid(transaction ID) field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::xid",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__xid.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Xid [DHCPv4::xid]\"\n    }",
            return_value: "This command returns xid(transaction ID) field from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::xid" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
