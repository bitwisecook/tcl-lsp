//! `client_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "client_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the client IP address of a connection.",
            synopsis: &["client_addr"],
            snippet: "Returns the client IP address of a connection. This is a BIG-IP version 4.X variable, provided for backward compatibility. You can use the equivalent 9.X command, IP::client_addr instead.",
            source: "https://clouddocs.f5.com/api/irules/client_addr.html",
            examples: "",
            return_value: "client_addr Returns the client IP address of a connection.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "client_addr" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        deprecated_replacement: Some("IP::client_addr"),
        ..CommandSpec::DEFAULT
    }
}
