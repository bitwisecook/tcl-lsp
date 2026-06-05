//! `TCP::rexmt_thresh` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rexmt_thresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the retransmission threshold of a TCP connection.",
            synopsis: &["TCP::rexmt_thresh (TCP_REXMT_THRESH_VALUE)?"],
            snippet: "TCP::rexmt_thresh returns the retransmission threshold of a TCP connection.\nTCP::rexmt_thresh TCP_REXMT_THRESH_VALUE sets the retransmission threshold to specified value.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rexmt_thresh.html",
            examples: "t/set the retransmission threshold of the TCP flow.\n    when CLIENT_ACCEPTED {\n        log local0. \"TCP set rtx thresh: [TCP::rexmt_thresh 100]\"\n        log local0. \"TCP get rtx thresh: [TCP::rexmt_thresh]\"\n    }",
            return_value: "TCP::rexmt_thresh returns the retransmission threshold of a TCP connection.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::rexmt_thresh (TCP_REXMT_THRESH_VALUE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
