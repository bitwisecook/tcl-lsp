//! `TCP::setmss` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::setmss",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the TCP max segment size.",
            synopsis: &["TCP::setmss TCP_MAX_SEGMENT_SIZE"],
            snippet: "This iRule command sets the TCP max segment size in bytes.\nThe MSS does not consider the length of any common TCP options.\nUsers should set MSS to the desired path IP packet size, minus the\nIP header length (typically 20 bytes), minus the minimum TCP header\nlength of 20 bytes.\n\nTCP will automatically apply the length of common options when\npartitioning data for delivery.",
            source: "https://clouddocs.f5.com/api/irules/TCP__setmss.html",
            examples: "# Match clientside MSS to serverside MSS\nwhen SERVER_CONNECTED {\n    set cli_mss [clientside { TCP::mss }]\n    set svr_mss [TCP::mss]\n    if { $cli_mss > $svr_mss } {\n        clientside { TCP::setmss $svr_mss }\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::setmss TCP_MAX_SEGMENT_SIZE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
