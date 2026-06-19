//! `HTTP::path` iRules command.
use crate::prelude::*;
use crate::taint::SetterConstraint;

/// GAP-D2: the setter form of `HTTP::path` requires its value to start
/// with `/` (IRULE3101). Registry-driven replacement for the hardcoded
/// `SETTER_CONSTRAINTS` table in `tcl_compiler::taint`. Mirrors
/// `irules/http__path.py`.
const SETTER_CONSTRAINTS: &[SetterConstraint] = &[SetterConstraint {
    arg_index: 0,
    required_prefix: "/",
    code: "IRULE3101",
    message: "HTTP::path value must start with '/'",
}];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::path",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::DIAGRAM_ACTION)
            .union(Traits::UNNORMALISED_HTTP_GETTER),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        options: &[OptionSpec {
            name: "-normalized",
            takes_value: false,
            value_hint: "",
            detail: "Return the canonicalised path (URL evasion patterns rejected).",
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Returns or sets the path part of the HTTP request.",
            synopsis: &["HTTP::path (PATH_VALUE)?"],
            snippet: "Returns or sets the path part of the HTTP request. The path is defined\nas the path and filename in a request. It does not include the query string.\nFor the following URL: http://www.example.com:8080/main/index.jsp?user=test&login=check\n\nThe path is: /main/index.jsp\n\nNote that only ? is used as the separator for the query string.\nSo, for the following URL: http://www.example.com:8080/main/index.jsp;session_id=abcdefgh?user=test&login=check\n\nThe path is: /main/index.jsp;session_id=abcdefgh\n\n**Best practice**: Prefer `HTTP::path` over `HTTP::uri` when you\nonly need the path without the query string. Use `-normalized`\nfor consistent matching (prevents URL-encoding evasion).",
            source: "https://clouddocs.f5.com/api/irules/HTTP__path.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"\\[HTTP::path\\] original: [HTTP::path]\"\n  HTTP::path [string tolower [HTTP::path]]\n}",
            return_value: "Returns the path part of the HTTP request.",
        }),
        setter_constraints: SETTER_CONSTRAINTS,
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec {
                kind: FormKind::Getter,
                synopsis: "HTTP::path ?-normalized?",
            },
            FormSpec {
                kind: FormKind::Setter,
                synopsis: "HTTP::path <PATH_VALUE>",
            },
        ],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::PATH_PREFIXED)),
        ..CommandSpec::DEFAULT
    }
}
