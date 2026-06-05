//! `SSL::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Collect plaintext data after SSL offloading.",
            synopsis: &["SSL::collect (LENGTH)?"],
            snippet: "Starts the collection of plaintext data either indefinitely or for the specified amount of data. On successful collection, the corresponding data event is triggered. For clientside collection, the CLIENTSSL_DATA event is triggered. For serverside collection, the SERVERSSL_DATA event is triggered.",
            source: "https://clouddocs.f5.com/api/irules/SSL__collect.html",
            examples: "when SERVERSSL_HANDSHAKE {\n    SSL::collect\n}",
            return_value: "SSL::collect [<length>] Starts the collection of plaintext data either indefinitely or for the specified amount of data. When is specified, the data event will not be triggered until that length has been collected.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::collect (LENGTH)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
