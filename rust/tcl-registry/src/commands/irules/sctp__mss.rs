//! `SCTP::mss` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::mss",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the on-wire Maximum Segment Size (MSS) for an SCTP connection.",
            synopsis: &["SCTP::mss"],
            snippet: "Returns the on-wire Maximum Segment Size (MSS) for an SCTP connection.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__mss.html",
            examples: "when CLIENT_ACCEPTED {\n        SCTP::collect\n        log local0.info \"Sctp local port is [SCTP::local_port]\"\n        log local0.info \"Sctp client port is [SCTP::client_port]\"\n        log local0.info \"Sctp mss is [SCTP::mss]\"\n        log local0.info \"sctp ppi is [SCTP::ppi]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SCTP::mss" },
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
