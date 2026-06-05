//! `HTTP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        options: &[
            OptionSpec {
                name: "-version",
                takes_value: true,
                value_hint: "1.0 | 1.1",
                detail: "Protocol version on the synthesised response.",
                dialects: None,
            },
            OptionSpec {
                name: "-status",
                takes_value: true,
                value_hint: "reason",
                detail: "Override the default reason phrase for the status code.",
                dialects: None,
            },
            OptionSpec {
                name: "-noserver",
                takes_value: false,
                value_hint: "",
                detail: "Suppress the auto-injected `Server` response header.",
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Send an immediate HTTP response from an iRule.",
            &["HTTP::respond <status> ?option value ...?"],
            "F5 iRules",
        )),
        // GAP-D2: tainted data in the response body → XSS/content
        // injection (IRULE3001). Mirrors `irules/http__respond.py`.
        taint_output_sink: Some("IRULE3001"),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[
                "AUTH_ERROR",
                "AUTH_FAILURE",
                "AUTH_RESULT",
                "AUTH_SUCCESS",
                "AUTH_WANTCREDENTIAL",
                "LB_FAILED",
                "MR_EGRESS",
                "MR_FAILED",
                "NAME_RESOLVED",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
