//! `HTTP::respond` iRules command.
use crate::prelude::*;

/// Bareword option tokens that follow the `<status>` positional
/// argument (`HTTP::respond 302 content|noserver|reset|version`).
/// Mirrors the form-level `arg_values[1]` in `irules/http__respond.py`.
const RESPOND_OPTION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "content",
        detail: "Inline response body.",
    },
    ArgValue {
        value: "noserver",
        detail: "Suppress Server header.",
    },
    ArgValue {
        value: "reset",
        detail: "Reset server-side connection.",
    },
    ArgValue {
        value: "version",
        detail: "Response HTTP version.",
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        // Option set mirrors `irules/http__respond.py` (the reference
        // standard): `-version`/`-content`/`-ifile`/`-noserver`/`-reset`.
        // (`-status` is the positional status arg, not an option.)
        options: &[
            OptionSpec {
                name: "-version",
                takes_value: true,
                value_hint: "1.0 | 1.1",
                detail: "Protocol version on the synthesised response.",
                dialects: None,
            },
            OptionSpec {
                name: "-content",
                takes_value: true,
                value_hint: "CONTENT",
                detail: "Response body content.",
                dialects: None,
            },
            OptionSpec {
                name: "-ifile",
                takes_value: true,
                value_hint: "IFILE_OBJ",
                detail: "Serve the response body from an iFile object.",
                dialects: None,
            },
            OptionSpec {
                name: "-noserver",
                takes_value: false,
                value_hint: "",
                detail: "Suppress the auto-injected `Server` response header.",
                dialects: None,
            },
            OptionSpec {
                name: "-reset",
                takes_value: false,
                value_hint: "",
                detail: "Reset the connection after sending the response.",
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
        // Command-level arg-value completion: the bareword option
        // tokens that follow the `<status>` positional argument
        // (`HTTP::respond 302 content|noserver|reset|version`).  Mirrors
        // the form-level `arg_values[1]` in `irules/http__respond.py`.
        arg_values: &[(1, RESPOND_OPTION_VALUES)],
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::respond <status> ?option value ...?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ResponseCommit,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
