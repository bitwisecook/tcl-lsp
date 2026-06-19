//! `SCTP::rto_min` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::rto_min",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the minimum value of SCTP retransmission timeout.",
            synopsis: &["SCTP::rto_min (clientside | serverside)?"],
            snippet: "Returns the minimum value of SCTP retranmission timeout. Can specify the value on clientside or serverside.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__rto_min.html",
            examples: "when SERVER_CONNECTED {\n        log local0.info \"SCTP retransmission timeout minimum value is [SCTP::rto_min]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SCTP::rto_min (clientside | serverside)?",
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
