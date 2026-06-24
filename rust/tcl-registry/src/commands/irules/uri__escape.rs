//! `URI::escape` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::escape",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Percent-encodes a URI string (alias for URI::encode).",
            synopsis: &["URI::escape URI_STRING"],
            snippet: "Percent-encodes *URI_STRING* according to RFC 3986.\nThis is an alias for ``URI::encode``.",
            source: "https://clouddocs.f5.com/api/irules/URI__encode.html",
            examples: "",
            return_value: "Returns a percent-encoded URI string.",
        }),
        // URL-encodes its input (and strips CR/LF);
        // re-encoding a URL-encoded value double-encodes (T106).
        taint_transform: Some(TaintColour::URL_ENCODED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::URL_ENCODED),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "URI::escape URI_STRING",
        }],
        ..CommandSpec::DEFAULT
    }
}
