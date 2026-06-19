//! `UDP::sendbuffer` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::sendbuffer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the maximum send buffer size (bytes) of a UDP connection.",
            synopsis: &["UDP::sendbuffer (UDP_SNDBUF_SIZE)?"],
            snippet: "UDP::sendbuffer returns the maximum send buffer size (bytes) of a UDP connection.\nUDP::sendbuffer BUFFERSIZE sets the maximum send buffer size (bytes) to specified value.",
            source: "https://clouddocs.f5.com/api/irules/UDP__sendbuffer.html",
            examples: "# Get/set the send buffer size of the UDP flow.\nwhen CLIENT_ACCEPTED {\n    log local0. \"UDP get send buffer: [UDP::sendbuffer]\"\n    # Set the send buffer to 2,000,000 bytes\n    log local0. \"UDP set send buffer: [UDP::sendbuffer 2000000]\"\n    log local0. \"UDP get send buffer: [UDP::sendbuffer]\"\n}",
            return_value: "UDP::sendbuffer returns the maximum send buffer size (bytes) of a UDP connection.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "UDP::sendbuffer (UDP_SNDBUF_SIZE)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::UdpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
