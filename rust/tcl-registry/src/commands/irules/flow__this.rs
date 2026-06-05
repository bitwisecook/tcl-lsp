//! `FLOW::this` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::this",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the TCL handle for the current flow.",
            synopsis: &["FLOW::this"],
            snippet: "Returns the TCL handle for the current flow.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__this.html",
            examples: "when CLIENT_ACCEPTED {\n    set cf [FLOW::this]\n    log local0. \"Current flow is $cf\"\n    unset cf\n}",
            return_value: "TCL handle for the current flow. On error an exception is thrown with a message indicating the cause of failure.",
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
            FormSpec { kind: FormKind::Default, synopsis: "FLOW::this" },
        ],
        ..CommandSpec::DEFAULT
    }
}
