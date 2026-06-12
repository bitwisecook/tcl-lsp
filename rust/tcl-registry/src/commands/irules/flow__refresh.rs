//! `FLOW::refresh` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::refresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Refresh the flow.",
            synopsis: &["FLOW::refresh ANY_CHARS"],
            snippet: "Updates the last used time on the flow to now.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__refresh.html",
            examples: "when CLIENT_DATA {\n        # Log and refresh the related flow whenever the client sends data.\n        log local0. \"Flow idle duration before refresh [FLOW::idle_duration $result]\"\n        FLOW::refresh $result\n        log local0. \"Flow idle duration after refresh [FLOW::idle_duration $result]\"\n        TCP::release\n        TCP::collect\n\n    }",
            return_value: "",
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
            FormSpec { kind: FormKind::Default, synopsis: "FLOW::refresh ANY_CHARS" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FlowState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
