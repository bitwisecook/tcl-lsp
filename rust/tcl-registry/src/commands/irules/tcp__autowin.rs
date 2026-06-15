//! `TCP::autowin` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::autowin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Toggles automatic window tuning.",
            synopsis: &["TCP::autowin BOOL_VALUE"],
            snippet: "Sets the send and receive buffer dynamically in accordance with measured connection parameters.",
            source: "https://clouddocs.f5.com/api/irules/TCP__autowin.html",
            examples: "when HTTP_REQUEST {\n    # Enable auto buffer tuning on HTTP request(s).\n    log local0. \"Send buffer: [TCP::sendbuf] Receive Window: [TCP::recvwnd]\"\n    log local0. \"HTTP request, auto buffer tuning enabled.\"\n    TCP::autowin enable\n    log local0. \"Send buffer: [TCP::sendbuf] Receive Window: [TCP::recvwnd]\"\n}",
            return_value: "None.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::autowin BOOL_VALUE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
