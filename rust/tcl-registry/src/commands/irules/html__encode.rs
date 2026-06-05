//! `HTML::encode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "HTML-encodes a string, escaping special characters.",
            &["HTML::encode STRING"],
            "F5 iRules",
        )),
        // GAP-D2: HTML-escapes its input (and strips CR/LF);
        // re-encoding an HTML-escaped value double-encodes (T106).
        // Mirrors `irules/html__encode.py`.
        taint_transform: Some(TaintColour::HTML_ESCAPED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::HTML_ESCAPED),
        ..CommandSpec::DEFAULT
    }
}
