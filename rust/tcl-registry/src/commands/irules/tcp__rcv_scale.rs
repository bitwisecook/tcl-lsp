//! `TCP::rcv_scale` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rcv_scale",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the receive window scale advertised by the remote host.",
            synopsis: &["TCP::rcv_scale"],
            snippet: "Returns the receive window scale advertised by the remote host.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rcv_scale.html",
            examples: "when CLIENT_ACCEPTED {\n    # Log rcv_scale.\n    log local0. \"rcv_scale: [TCP::rcv_scale]\"\n}",
            return_value: "The bitshift associated with the remote host window scale.",
        }),
        excluded_events: &["SERVER_INIT"],
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::rcv_scale",
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
