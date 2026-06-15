//! `local_addr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "local_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Deprecated: Use IP::local_addr instead.",
            synopsis: &["local_addr"],
            snippet: "Deprecated: Use IP::local_addr instead",
            source: "https://clouddocs.f5.com/api/irules/local_addr.html",
            examples: "when CLIENT_ACCEPTED {\n    log local0. \"client ip addr: [local_addr]\"\n}",
            return_value: "Returns the IP address being used in the connection. In the clientside context, this is the destination IP address from the client request (not necessarily the virtual IP address).",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "local_addr",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        deprecated_replacement: Some("IP::local_addr"),
        ..CommandSpec::DEFAULT
    }
}
