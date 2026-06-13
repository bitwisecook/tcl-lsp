//! `URI::encode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an encoded version of a given URI.",
            synopsis: &["URI::encode URI_STRING"],
            snippet: "Returns the encoded version of the given URI.\nFor details on URI encoding, see RFC3986, section 2.1. Percent-Encoding.\n\nThis command is equivalent to the BIG-IP 4.X variable encode_uri.",
            source: "https://clouddocs.f5.com/api/irules/URI__encode.html",
            examples: "when HTTP_REQUEST {\n  set my_parameter_value \"my URL encoded parameter value with metacharacters (&*@#[])\"\n  log local0. \"The encoded version of \\\"$my_parameter_value\\\" is \\\"[URI::encode $my_parameter_value]\\\"\"\n  HTTP::redirect \"/path?parameter=[URI::encode $my_parameter_value]\"\n}",
            return_value: "Returns an encoded version of a given URI.",
        }),
        // GAP-D2: URL-encodes its input (and strips CR/LF);
        // re-encoding a URL-encoded value double-encodes (T106).
        // Mirrors `irules/uri__encode.py`.
        taint_transform: Some(TaintColour::URL_ENCODED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::URL_ENCODED),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "URI::encode URI_STRING" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpUri,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
