//! `FLOW::idle_duration` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::idle_duration",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the time in seconds when the flow was last used.",
            synopsis: &["FLOW::idle_duration ANY_CHARS"],
            snippet: "Returns the time in seconds when the flow was last used.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__idle_duration.html",
            examples: "when CLIENT_DATA {\n            # Log and refresh the related flow whenever the client sends data.\n            log local0. \"Flow idle duration before refresh [FLOW::idle_duration $result]\"\n            FLOW::refresh $result\n            log local0. \"Flow idle duration after refresh [FLOW::idle_duration $result]\"\n            TCP::release\n            TCP::collect\n\n        }",
            return_value: "Returns the time in seconds when the flow was last used.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &[
                "CLIENT_ACCEPTED",
                "CLIENT_DATA",
                "LB_SELECTED",
                "SA_PICKED",
                "SERVER_CONNECTED",
                "SERVER_DATA",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "FLOW::idle_duration ANY_CHARS" },
        ],
        ..CommandSpec::DEFAULT
    }
}
