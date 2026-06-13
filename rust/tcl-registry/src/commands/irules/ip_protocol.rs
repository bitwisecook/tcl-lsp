//! `ip_protocol` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip_protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the IP protocol value.",
            synopsis: &["ip_protocol"],
            snippet: "Returns the IP protocol value. This is a BIG-IP 4.X variable, provided\nfor backward-compatibility. You can use the command IP::protocol\ninstead.",
            source: "https://clouddocs.f5.com/api/irules/ip_protocol.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ip_protocol" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        deprecated_replacement: Some("IP::protocol"),
        ..CommandSpec::DEFAULT
    }
}
