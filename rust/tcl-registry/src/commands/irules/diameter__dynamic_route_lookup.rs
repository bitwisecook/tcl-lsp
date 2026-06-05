//! `DIAMETER::dynamic_route_lookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::dynamic_route_lookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set whether messages should be routed dynamically.",
            synopsis: &["DIAMETER::dynamic_route_lookup ( connection | message ) ( BOOLEAN )?"],
            snippet: "\"message\":\nIf status is set to \"enabled\", previously created dynamic routes will be consulted during the routing of this message.\n\n\"connection\":\nThe setting will be applied to this and all later messages on this connection.\n\nThe zero-argument form of this command returns whether the setting is enabled on the current message.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__dynamic_route_lookup.html",
            examples: "when DIAMETER_INGRESS {\n                if { ([DIAMETER::header appid] equals 666) } {\n                    DIAMETER::dynamic_route_lookup message disabled\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
