//! `URI::encode_component` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::encode_component",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Percent-encodes a single URI component.",
            synopsis: &["URI::encode_component STRING"],
            snippet: "Percent-encodes a single URI component (path segment, query\nparameter name or value, fragment, etc.) according to RFC 3986\nsection 2.1.  Unlike ``URI::encode`` this encodes every\nreserved delimiter (``/``, ``?``, ``&``, ``=``, …) so the\nresult is safe to embed inside a larger URI without altering\nits structure.",
            source: "https://clouddocs.f5.com/api/irules/URI__encode.html",
            examples: "when HTTP_REQUEST {\n  set value \"key=value&other\"\n  HTTP::uri \"/search?q=[URI::encode_component $value]\"\n}",
            return_value: "Returns a percent-encoded string.",
        }),
        // URL-encodes its input (and strips CR/LF);
        // re-encoding a URL-encoded value double-encodes (T106).
        taint_transform: Some(TaintColour::URL_ENCODED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::URL_ENCODED),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "URI::encode_component STRING",
        }],
        ..CommandSpec::DEFAULT
    }
}
