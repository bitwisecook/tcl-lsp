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
hover: Some(HoverSnippet {
            summary: "Send an immediate HTTP response from an iRule.",
            synopsis: &["HTTP::respond <status> ?option value ...?"],
            snippet: "Common options include `content`, `noserver`, `reset`, and `version`.\n\nThe response is sent when the current event completes. You cannot alter it in later HTTP events or after another response has already been sent.\n\n**Security**: When the response body contains user-supplied data\n(HTTP headers, URI, payload), HTML-encode it to prevent XSS.\nFor blocking/maintenance pages, include `Connection close` and\n`Cache-Control no-store` headers:\n```tcl\nHTTP::respond 403 content $html Connection close Cache-Control no-store\n```",
            source: "https://clouddocs.f5.com/api/irules/HTTP__respond.html",
            examples: "",
            return_value: "",
        }),
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
