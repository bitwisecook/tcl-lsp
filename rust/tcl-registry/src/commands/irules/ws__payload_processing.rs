//! `WS::payload_processing` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::payload_processing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables or disables processing of WebSocket payload via payload protocol filter",
            synopsis: &["WS::payload_processing ('enable' | 'disable')"],
            snippet: "WS::payload_processing enable\n    Enables processing of WebSocket Payload via payload protocol filter\nWS::payload_processing disable\n    Disables processing of WebSocket Payload via payload protocol filter",
            source: "https://clouddocs.f5.com/api/irules/WS__payload_processing.html",
            examples: "when WS_REQUEST {\n    switch [WS::request protocol] {\n        mqtt {\n            WS::payload_processing enable\n        }\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "WS::payload_processing ('enable' | 'disable')" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
