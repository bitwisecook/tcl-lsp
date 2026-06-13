//! `ip_tos` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip_tos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the ToS level of a packet.",
            synopsis: &["ip_tos"],
            snippet: "Returns the ToS level of a packet. The Type of Service (ToS) standard\nis a means by which network equipment can identify and treat traffic\ndifferently based on an identifier. As traffic enters the site, the\nBIG-IP system can apply a rule that sends the traffic to different\npools of servers based on the ToS level within a packet.\nThis is a BIG-IP version 4.X variable, provided for\nbackward-compatibility. You can use the equivalent 9.X command\nIP::tos instead.",
            source: "https://clouddocs.f5.com/api/irules/ip_tos.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ip_tos" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        deprecated_replacement: Some("IP::tos"),
        ..CommandSpec::DEFAULT
    }
}
