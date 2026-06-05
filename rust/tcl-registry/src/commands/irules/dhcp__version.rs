//! `DHCP::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCP::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns version number (4 or 6) of DHCP packet.",
            synopsis: &["DHCP::version"],
            snippet: "This command returns DHCP version number (4 or 6) of DHCP packet\n\nDetails (syntax):\nDHCP::version",
            source: "https://clouddocs.f5.com/api/irules/DHCP__version.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Version [DHCP::version]\"\n    }",
            return_value: "This command returns version number (4 or 6) of DHCP packet",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCP::version" },
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
