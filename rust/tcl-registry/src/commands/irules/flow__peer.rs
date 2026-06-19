//! `FLOW::peer` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::peer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the TCL flow handle for the peer flow.",
            synopsis: &["FLOW::peer ANY_CHARS"],
            snippet: "Returns the TCL flow handle for the peer flow.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__peer.html",
            examples: "when SERVER_CONNECTED {\n    # Get server side flow handle.\n    set cf [FLOW::this]\n\n    # Get client side flow handle.\n    set peer [FLOW::peer $cf]\n    log local0. \"Peer flow is $peer\"\n    unset cf peer\n}",
            return_value: "TCL handle for the peer flow. On error an exception is thrown with a message indicating the cause of failure.",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "FLOW::peer ANY_CHARS",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FlowState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
