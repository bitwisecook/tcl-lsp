//! `URI::decode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::decode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a decoded version of a given URI.",
            synopsis: &["URI::decode URI_STRING"],
            snippet: "Returns a URI decoded version of a given URI.\nFor details on URI encoding, see RFC3986, section 2.1. Percent-Encoding.\n\nThis command is equivalent to the BIG-IP 4.X variable decode_uri.",
            source: "https://clouddocs.f5.com/api/irules/URI__decode.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"The decoded version of \\\"[HTTP::query]\\\" is \\\"[URI::decode [HTTP::query]]\\\"\"\n}",
            return_value: "Returns a decoded version of a given URI.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "URI::decode URI_STRING" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpUri,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        traits: Traits::IS_UNESCAPE,
        ..CommandSpec::DEFAULT
    }
}
