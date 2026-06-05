//! `MR::ignore_peer_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::ignore_peer_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the ignore_peer_port mode for the current connection.",
            synopsis: &["MR::ignore_peer_port (BOOLEAN)?"],
            snippet: "The MR::ignore_peer_port command sets or resets the ignore_peer_port mode of the current connection. If ignore_peer_port mode is enabled, the remote port of the connection will be ignored when determining if the connection is usable for forwarding a message to a peer. For example, if a peer at IP 10.1.2.3 connects using a ephemeral port of 12345 and ignore_peer_port is enabled, a message routed to IP 10.1.2.3 port 2345 can be forwarded using this connection since the port will be ignored.",
            source: "https://clouddocs.f5.com/api/irules/MR__ignore_peer_port.html",
            examples: "when CLIENT_ACCEPTED {\n                MR::ignore_peer_port yes\n            }",
            return_value: "Returns the current value of the ignore_peer_port flag. This will be 'true' or 'false'.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::ignore_peer_port (BOOLEAN)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
