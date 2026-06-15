//! `TCP::snd_cwnd` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_cwnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the TCP congestion window (cwnd).",
            synopsis: &["TCP::snd_cwnd"],
            snippet: "Returns the TCP congestion window (cwnd), the maximum\nunacknowledged data the connection can send due to the congestion\ncontrol algorithm.\n\nThe actual amount of outstanding data may be lower, due to lack of\napplication data to send, the remote host's advertised receive\nwindow, or the size of the BIG-IP send buffer.",
            source: "https://clouddocs.f5.com/api/irules/TCP__snd_cwnd.html",
            examples: "when CLIENT_CLOSED {\n    # Get BIGIP's last congestion window.\n    log local0. \"BIGIP's cwnd: [TCP::snd_cwnd]\"\n}",
            return_value: "The cwnd in bytes.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::snd_cwnd",
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
