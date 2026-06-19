//! `TCP::sendbuf` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::sendbuf",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the send buffer size of a TCP connection.",
            synopsis: &["TCP::sendbuf ('auto' | BUFFER_SIZE)?"],
            snippet: "TCP::sendbuf returns the send buffer size of a TCP connection.\nTCP::sendbuf BUFFER_SIZE sets the send buffer size to BUFFER_SIZE bytes.",
            source: "https://clouddocs.f5.com/api/irules/TCP__sendbuf.html",
            examples: "t the send buffer size of the TCP flow.\n    when CLIENT_ACCEPTED {\n        log local0. \"TCP set send buffer: [TCP::sendbuf 100000]\"\n        log local0. \"TCP get send buffer: [TCP::sendbuf]\"\n    }",
            return_value: "TCP::sendbuf returns the number of bytes that can be stored at the send buffer.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::sendbuf ('auto' | BUFFER_SIZE)?",
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
