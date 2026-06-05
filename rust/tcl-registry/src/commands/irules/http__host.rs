//! `HTTP::host` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::host",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
hover: Some(HoverSnippet {
            summary: "Returns the value contained in the Host header of an HTTP request.",
            synopsis: &["HTTP::host ?name?"],
            snippet: "Returns the value contained in the Host header of an HTTP request. This\ncommand replaces the BIG-IP 4.X variable http_host.\nThe Host header always contains the requested host name (which may be a\nHost Domain Name string or an IP address), and will also contain the\nrequested service port whenever a non-standard port is specified (other\nthan 80 for HTTP, other than 443 for HTTPS). When present, the\nnon-standard port is appended to the requsted name as a numeric string\nwith a colon separating the 2 values (just as it would appear in the\nbrowser's address bar):\n  * Host: host.domain.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__host.html",
            examples: "when HTTP_REQUEST {\n  if { [HTTP::uri] contains \"secure\"} {\n    HTTP::redirect \"https://[HTTP::host][HTTP::uri]\"\n }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Getter, synopsis: "HTTP::host" },
            FormSpec { kind: FormKind::Setter, synopsis: "HTTP::host <name>" },
        ],
        ..CommandSpec::DEFAULT
    }
}
