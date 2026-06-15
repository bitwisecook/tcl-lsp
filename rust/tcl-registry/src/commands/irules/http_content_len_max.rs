//! `http_content_len_max` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_content_len_max",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Return the HTTP Content-Length up to a maximum size (default 1024), or reject if the header is not a valid integer.",
            synopsis: &[
                "call http_content_len_max",
                "call http_content_len_max 8192",
            ],
            snippet: "Returns the `Content-Length` header value clamped to *size* (default 1024).\n\n  - `Content-Length: 512` → returns `512`\n  - `Content-Length: 2048` with default size → returns `1024`\n  - `Content-Length: 2048` with size `8192` → returns `2048`\n  - No `Content-Length` header → returns the empty string\n  - Non-integer `Content-Length` (RFC 2616 Section 14.13 violation) → calls `reject`",
            source: "https://clouddocs.f5.com/api/irules/http_content_len_max.html",
            examples: "# Collect up to the default 1024 bytes\nwhen HTTP_REQUEST priority 500 {\n    HTTP::collect [call http_content_len_max]\n}\n\n# Collect up to 8192 bytes\nwhen HTTP_REQUEST priority 500 {\n    HTTP::collect [call http_content_len_max 8192]\n}",
            return_value: "The effective Content-Length (clamped to *size*), the empty string if no header is present, or the connection is rejected.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "http_content_len_max ?max_size?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpBody,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
