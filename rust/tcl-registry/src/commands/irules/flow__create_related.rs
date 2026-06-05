//! `FLOW::create_related` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::create_related",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Creates a related client side and server side flow.",
            synopsis: &["FLOW::create_related (((-translation-loose) (-hairpin))#)? (FLOW_CREATE_RELATED_SUBCMDS)+"],
            snippet: "Creates a related connection. Each related connection has two flows in it, a clientside flow and a serverside flow. The clientside flow is created using\nthe information provided in \"clientflow\" and serverside flow is created using the information provided in the \"serverflow\". Both these flows are linked\ntogether and form a connection. BIGIP excepts that the the first packet always comes from the client side of the connection for all protocols except UDP.\nThe returned TCL handle points to the clientside flow. [FLOW::peer] command can be used to get a handle to the peer flow.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__create_related.html",
            examples: "when SERVER_CONNECTED {\n            # LSN pool with prefix 4.4.4.0/30,port-range=2000-2005 and NAPT mode is configured. Parent connection is translated as follows\n            # 10.10.0.1%1:60412 -> 10.20.0.1%1:9000 TO 4.4.4.1:1084  10.20.0.1:9000  tcp\n            # Subscriber side: 10.10.0.1%1:60412 -> 10.20.0.1%1:9000\n            # Internet side: 4.4.4.1:1084  10.20.0.1:9000\n            # Below is an example of couple of related connections \n            \n            # Connection-1:",
            return_value: "TCL handle for the client side flow. On error an exception is thrown with a message indicating the cause of failure. The string representation of the TCL handle can be used to retrieve the flow details.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_DATA", "SERVER_CONNECTED", "SERVER_DATA"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "FLOW::create_related (((-translation-loose) (-hairpin))#)? (FLOW_CREATE_RELATED_SUBCMDS)+" },
        ],
        options: &[
            OptionSpec { name: "-translation-loose", takes_value: false, value_hint: "", detail: "Option -translation-loose.", dialects: None },
            OptionSpec { name: "-hairpin", takes_value: false, value_hint: "", detail: "Option -hairpin.", dialects: None },
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
