//! `HTTP::uri` iRules command.
use crate::prelude::*;
use crate::taint::SetterConstraint;

/// GAP-D2: the setter form of `HTTP::uri` requires its value to start
/// with `/` (IRULE3101). Registry-driven replacement for the hardcoded
/// `SETTER_CONSTRAINTS` table in `tcl_compiler::taint`. Mirrors
/// `irules/http__uri.py`.
const SETTER_CONSTRAINTS: &[SetterConstraint] = &[SetterConstraint {
    arg_index: 0,
    required_prefix: "/",
    code: "IRULE3101",
    message: "HTTP::uri value must start with '/'",
}];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::uri",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE).union(Traits::DIAGRAM_ACTION).union(Traits::UNNORMALISED_HTTP_GETTER),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        options: &[OptionSpec {
            name: "-normalized",
            takes_value: false,
            value_hint: "",
            detail: "Return the canonicalised URI (URL evasion patterns rejected).",
            dialects: None,
        }],
hover: Some(HoverSnippet {
            summary: "Returns or sets the URI part of the HTTP request.",
            synopsis: &["HTTP::uri (URI)?"],
            snippet: "Returns or sets the URI part of the HTTP request. This command replaces\nthe BIG-IP 4.X variable http_uri.\n\nFor the following URL:\nhttp://www.example.com:8080/main/index.jsp?user=test&login=check\nThe URI is: /main/index.jsp?user=test&login=check\n\nNote that in the HTTP_PROXY_REQUEST event, this command returns the complete\nproxy URI. This includes the scheme, host and port, and thus the result would be:\nhttp://www.example.com:8080/main/index.jsp?user=test&login=check",
            source: "https://clouddocs.f5.com/api/irules/HTTP__uri.html",
            examples: "when HTTP_PROXY_REQUEST {\n   log local.0 \"This proxy request is:[HTTP::uri]\"\n}",
            return_value: "Returns the URI part of the HTTP request.",
        }),
        setter_constraints: SETTER_CONSTRAINTS,
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_EGRESS", "MR_FAILED", "MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Getter, synopsis: "HTTP::uri ?-normalized?" },
            FormSpec { kind: FormKind::Setter, synopsis: "HTTP::uri <URI>" },
        ],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::PATH_PREFIXED)),
        ..CommandSpec::DEFAULT
    }
}
