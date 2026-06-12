//! `UDP::max_buf_pkts` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::max_buf_pkts",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the maximum buffer packets value of a UDP connection.",
            synopsis: &["UDP::max_buf_pkts (UDP_MAX_BUF_PKTS)?"],
            snippet: "UDP::max_buf_pkts returns the maximum buffer packets value of a UDP connection.\nUDP::max_buf_pkts UDP_MAX_BUF_PKTS sets the maximum buffer packets value to specified value.",
            source: "https://clouddocs.f5.com/api/irules/UDP__max_buf_pkts.html",
            examples: "# Get/set the max buffer packets of the UDP flow.\nwhen CLIENT_ACCEPTED {\n    log local0. \"UDP get max buffer packets: [UDP::max_buf_pkts]\"\n    # Set the max buffer packets to 5,000\n    log local0. \"UDP set max buffer packets: [UPD::max_buf_pkts 5000]\"\n    log local0. \"UDP get max buffer packets: [UDP::max_buf_pkts]\"\n}",
            return_value: "UDP::max_buf_pkts returns the maximum buffer packets value of a UDP connection.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "UDP::max_buf_pkts (UDP_MAX_BUF_PKTS)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::UdpState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
