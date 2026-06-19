//! `HTTP::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::payload",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Queries for or manipulates HTTP payload information.",
            synopsis: &[
                "HTTP::payload ( LENGTH | (OFFSET LENGTH) )?",
                "HTTP::payload length",
                "HTTP::payload rechunk",
                "HTTP::payload unchunk",
            ],
            snippet: "Queries for or manipulates HTTP payload (content) information. With\nthis command, you can retrieve content, query for content size, or\nreplace a certain amount of content. The content does not include the\nHTTP headers.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__payload.html",
            examples: "when HTTP_RESPONSE_DATA {\nHTTP::respond 200 content [HTTP::payload]\n}",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::payload ( LENGTH | (OFFSET LENGTH) )?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpBody,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        byte_array_payload: Some(BytePayloadSpec::DEFAULT),
        ..CommandSpec::DEFAULT
    }
}
