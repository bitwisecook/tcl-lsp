//! `ANTIFRAUD::disable_app_layer_encryption` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::disable_app_layer_encryption",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables application layer encryption for the current transaction.",
            synopsis: &["ANTIFRAUD::disable_app_layer_encryption"],
            snippet: "Disables application layer encryption for the current transaction.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_app_layer_encryption.html",
            examples: "when HTTP_REQUEST {\n                if { [HTTP::header exists \"Antifraud-Disable-AppLayerEncryption\" ] } {\n                    ANTIFRAUD::disable_app_layer_encryption\n                    log local0. \"Application Layer Encryption disabled\"\n                }\n            }",
            return_value: "Disables application layer encryption for the current transaction.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
