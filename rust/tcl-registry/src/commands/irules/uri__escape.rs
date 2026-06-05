//! `URI::escape` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::escape",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Percent-encodes a URI string (alias for URI::encode).",
            &["URI::escape URI_STRING"],
            "F5 iRules",
        )),
        // GAP-D2: URL-encodes its input (and strips CR/LF);
        // re-encoding a URL-encoded value double-encodes (T106).
        // Mirrors `irules/uri__escape.py`.
        taint_transform: Some(TaintColour::URL_ENCODED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::URL_ENCODED),
        ..CommandSpec::DEFAULT
    }
}
