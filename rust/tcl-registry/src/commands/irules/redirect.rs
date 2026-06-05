//! `redirect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "redirect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Redirects an HTTP request to the specific location.",
            synopsis: &["redirect to HOST_URI"],
            snippet: "Redirects an HTTP request to a specific location. The location can be\neither a host name or a URI. This is a BIG-IP 4.X statement, provided\nfor backward compatibility. You can use the equivalent 9.X command\nHTTP::redirect instead.",
            source: "https://clouddocs.f5.com/api/irules/redirect.html",
            examples: "when HTTP_REQUEST {\n    # HTTP::redirect, HTTP::host and HTTP::uri should be used instead\n    redirect to \"https://[http_host][http_uri]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "redirect to HOST_URI" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ResponseCommit,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
