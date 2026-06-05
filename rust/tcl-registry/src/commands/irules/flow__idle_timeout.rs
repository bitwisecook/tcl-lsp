//! `FLOW::idle_timeout` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::idle_timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets/Gets the idle timeout on the flow",
            synopsis: &["FLOW::idle_timeout (ANY_CHARS) (NONNEGATIVE_INTEGER)?"],
            snippet: "Sets/Gets the idle timeout on the flow.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__idle_timeout.html",
            examples: "when SERVER_CONNECTED {\n    set cf [FLOW::this]\n\n    #Get flow idletimeout\n    log local0. \"Idle timeout: [FLOW::idle_timeout $cf]\"\n\n    #Set flow idletimeout\n    FLOW::idle_timeout $cf 100\n\n    unset cf\n}",
            return_value: "Set operation: Nothing is returned Get operation: Idle timeout set on the flow as number string. On error an exception is thrown with a message indicating the cause of failure.",
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
        ..CommandSpec::DEFAULT
    }
}
