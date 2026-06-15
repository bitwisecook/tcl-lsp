//! `TCP::analytics` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::analytics",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable/disable AVR TCP stat reporting, and/or attach a user-defined string to categorize the connection for statistics collection purposes.",
            synopsis: &["TCP::analytics (enable | disable | key (KEY)?)"],
            snippet: "Enables or disables AVR TCP stat reporting (\"analytics\") for this connection and/or assigns user-defined keys.\n\nTCP::analytics enable\n    Enables analytics on this connection. AVR must be provisioned and the virtual must have a tcp-analytics profile attached. Collection will use the configuration in the profile. If the profile is configured to disable analytics by default, this gives users the ability to collect statistics by exception only.\n\nTCP::analytics disable\n    Disables analytics on this connection.",
            source: "https://clouddocs.f5.com/api/irules/TCP__analytics.html",
            examples: "rt collection for one subnet only.\n     when CLIENT_ACCEPTED {\n         if [IP::addr [IP::client_addr]/8 equals 10.0.0.0] {\n             TCP::analytics enable\n         }\n     }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::analytics (enable | disable | key (KEY)?)",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
