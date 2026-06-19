//! `SCTP::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or replaces SCTP data content.",
            synopsis: &["SCTP::payload ( (PAYLOAD_LENGTH) |"],
            snippet: "Returns the accumulated SCTP data content, or replaces collected payload with the specified data.\n\nSCTP::payload [<number_of_bytes>]\n    Returns the accumulated SCTP payload data content upto size bytes if provided.\n\nSCTP::payload <offset> <number_of_bytes>\n    Returns upto number_of_bytes bytes of the accumulated SCTP payload data content starting from the offset provided.\n\nSCTP::payload replace <offset> <length> <data>\n    Replaces collected payload data with the given data, starting at offset.\n\nSCTP::payload length\n    Returns the amount of accumulated SCTP data content in bytes.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__payload.html",
            examples: "when CLIENT_ACCEPTED {\n  SCTP::collect 15\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SCTP::payload ( (PAYLOAD_LENGTH) |",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        byte_array_payload: Some(BytePayloadSpec::DEFAULT),
        ..CommandSpec::DEFAULT
    }
}
