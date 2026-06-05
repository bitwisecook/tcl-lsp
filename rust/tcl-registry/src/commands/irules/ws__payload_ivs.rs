//! `WS::payload_ivs` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WS::payload_ivs",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Specifies name of the Internal Virtual Server (IVS) that will process websocket payload protocol",
            synopsis: &["WS::payload_ivs IVSNAME"],
            snippet: "WS::payload_ivs <IVS-name>\n    Sets the name of Internal Virtual Server (IVS) that will process websocket payload protocol.\n    This command takes effect only when payload processing mode of websocket profile is configured to Termination.",
            source: "https://clouddocs.f5.com/api/irules/WS__payload_ivs.html",
            examples: "when WS_REQUEST {\n    switch [WS::request protocol] {\n        mqtt {\n            WS::payload_processing enable\n            WS::payload_ivs /Common/mqtt_ivs\n        }\n    }\n}",
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
        ..CommandSpec::DEFAULT
    }
}
