//! `HTTP::redirect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::redirect",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Redirects an HTTP request or response to the specified URL.",
            synopsis: &["HTTP::redirect REDIRECT_URL"],
            snippet: "Redirects an HTTP request or response to the specified URL. Note that\nthis command sends the response to the client immediately. Therefore,\nyou cannot specify this command multiple times in an iRule, nor can you\nspecify any other commands that modify header or content after you\nspecify this command.\nThis command will always use a 302 response code. If you wish to use a\ndifferent one (e.g. 301), you will need to craft a response using\n[HTTP::respond].\nIf the client is a typical web browser, it will reflect the new URL\nthat you specify.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__redirect.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::status] == 404} {\n    HTTP::redirect \"http://www.example.com/newlocation.html\"\n  }\n}",
            return_value: "",
        }),
        // GAP-D2: tainted redirect URL → open-redirect (IRULE3004).
        // Mirrors `irules/http__redirect.py`.
        taint_output_sink: Some("IRULE3004"),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["LB_FAILED", "NAME_RESOLVED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTTP::redirect REDIRECT_URL" },
        ],
        ..CommandSpec::DEFAULT
    }
}
