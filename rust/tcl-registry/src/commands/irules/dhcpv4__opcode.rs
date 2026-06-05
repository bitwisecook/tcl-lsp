//! `DHCPv4::opcode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::opcode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns opcode field from DHCPv4 message.",
            synopsis: &["DHCPv4::opcode"],
            snippet: "This command returns opcode field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::opcode",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__opcode.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Htype [DHCPv4::opcode]\"\n    }",
            return_value: "This command returns opcode field from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::opcode" },
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
