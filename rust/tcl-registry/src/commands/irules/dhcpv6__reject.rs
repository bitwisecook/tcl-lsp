//! `DHCPv6::reject` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::reject",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command drops the packet while sending ICMP packet about the drop reason.",
            synopsis: &["DHCPv6::reject"],
            snippet: "This command drops the packet while sending ICMP packet about the drop reason\n\nDetails (syntax):\nDHCPv6::reject",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__reject.html",
            examples: "when CLIENT_DATA {\n        DHCPv6::reject\n    }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv6::reject" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
