//! `TCP::recvwnd` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::recvwnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the receive window size of a TCP connection.",
            synopsis: &["TCP::recvwnd ('auto' | WINDOW_SIZE)?"],
            snippet: "TCP::recvwnd returns the receive window size of a TCP connection.\nTCP::recvwnd WINDOW_SIZE sets the receive window to WINDOW_SIZE bytes.",
            source: "https://clouddocs.f5.com/api/irules/TCP__recvwnd.html",
            examples: "t the receive window size of the TCP flow.\n    when CLIENT_ACCEPTED {\n        log local0. \"TCP set receive window: [TCP::recvwnd 100000]\"\n        log local0. \"TCP get receive window: [TCP::recvwnd]\"\n    }",
            return_value: "TCP::recvwnd returns the number of bytes that can be stored at the receive window.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::recvwnd ('auto' | WINDOW_SIZE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
