//! `ADAPT::service_down_action` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::service_down_action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets or returns the service-down-action attribute.",
            synopsis: &["ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?"],
            snippet: "The ADAPT::service_down_action command sets or returns the\nservice-down-action attribute of the ADAPT filter on the\ncurrent or specified side of the virtual server connection\nfor which the iRule is being executed.\n\nPossible service-down-actions aare:\n    * ignore - Do not send the HTTP request or response to the\n      internal virtual server (bypass). Pass it through unchanged.\n    * reset - Reset (RST) the connection.\n    * drop - Drop (FIN) the connection.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__service_down_action.html",
            examples: "when ADAPT_REQUEST_HEADERS {\n     # Cause connection to be dropped if ICAP server handling\n     # response is down for requests with a custom HTTP header\n     # (which might have been resulted from request adaptation).\n     if {[HTTP::header exists \"X-Drop-if-down\"]} {\n        ADAPT::service_down_action response drop\n     }\n}",
            return_value: "Returns the current or modified service-down-action.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ADAPT::service_down_action (ADAPT_CTX)? (ADAPT_SIDE)? ('ignore' | 'reset' | 'drop')?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
