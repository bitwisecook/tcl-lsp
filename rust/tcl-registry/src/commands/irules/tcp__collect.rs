//! `TCP::collect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::collect",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Collects the specified amount of data for delivery to the iRule.",
            synopsis: &["TCP::collect (COLLECT_BYTES (SKIP_BYTES)?)?"],
            snippet: "Collects the specified amount of data before triggering a CLIENT_DATA or SERVER_DATA event. This data is not delivered to the upper layer until the event fires.\n\nAfter collecting the data in a clientside event, the CLIENT_DATA\nevent will be triggered. When collecting the data in a serverside\nevent, the SERVER_DATA event will be triggered.\n\nIt is important to note that, when an explicit length is not specified,\nthe semantics of TCP::collect and TCP::release are different than\nthose of the HTTP::collect and HTTP::release commands.\n\n**Caution**: If multiple iRules call `TCP::collect` on the same\nconnection, only the last call takes effect (last-execution-wins).\nUse a marker variable to coordinate:\n```tcl\nset tcp_collect 0\nif { !$tcp_collect } {\n    set tcp_collect 1\n    TCP::collect 43\n}\n```",
            source: "https://clouddocs.f5.com/api/irules/TCP__collect.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port].\"\n    # First collect 50000 bytes before passing data to the client.\n    TCP::collect 50000\n}",
            return_value: "None.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::collect (COLLECT_BYTES (SKIP_BYTES)?)?" },
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
