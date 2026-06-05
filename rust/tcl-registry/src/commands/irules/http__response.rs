//! `HTTP::response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the raw HTTP response header block as a single string.",
            synopsis: &["HTTP::response"],
            snippet: "Returns the raw HTTP response header block as a single string.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__response.html",
            examples: "when HTTP_RESPONSE {\n    # Send response header block to high speed logging\n    HSL::send $hsl [HTTP::response]\n}",
            return_value: "Returns the raw HTTP response header block as a single string.",
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
            FormSpec { kind: FormKind::Default, synopsis: "HTTP::response" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpHeader,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
