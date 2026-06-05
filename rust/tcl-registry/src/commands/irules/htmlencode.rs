//! `htmlencode` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "htmlencode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "HTML-encode a string (alias for HTML::encode).",
            synopsis: &["htmlencode STRING"],
            snippet: "Replaces HTML-special characters with their entity\nequivalents.  This is a convenience alias for\n``HTML::encode``.",
            source: "",
            examples: "",
            return_value: "Returns an HTML-escaped string.",
        }),
        // GAP-D2: HTML-escapes its input (and strips CR/LF);
        // re-encoding an HTML-escaped value double-encodes (T106).
        // Mirrors `irules/htmlencode.py`.
        taint_transform: Some(TaintColour::HTML_ESCAPED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::HTML_ESCAPED),
        ..CommandSpec::DEFAULT
    }
}
