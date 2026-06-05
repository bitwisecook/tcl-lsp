//! `client_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "client_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the TCP port number/service of the specified client.",
            synopsis: &["client_port"],
            snippet: "Returns the TCP port number/service of the specified client. This is a BIG-IP version 4.X variable, provided for backward compatibility. You can use the equivalent 9.X command, TCP::client_port instead.",
            source: "https://clouddocs.f5.com/api/irules/client_port.html",
            examples: "",
            return_value: "client_port Returns the TCP port number/service of the specified client.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "client_port" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
