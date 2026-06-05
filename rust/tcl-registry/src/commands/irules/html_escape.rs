//! `html_escape` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "html_escape",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "HTML-escape a string (alias for HTML::encode).",
            synopsis: &["html_escape STRING"],
            snippet: "Replaces HTML-special characters with their entity\nequivalents.  This is a convenience alias for\n``HTML::encode``.",
            source: "",
            examples: "",
            return_value: "Returns an HTML-escaped string.",
        }),
        // GAP-D2: HTML-escapes its input (and strips CR/LF);
        // re-encoding an HTML-escaped value double-encodes (T106).
        // Mirrors `irules/html_escape.py`.
        taint_transform: Some(TaintColour::HTML_ESCAPED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::HTML_ESCAPED),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "html_escape STRING" },
        ],
        ..CommandSpec::DEFAULT
    }
}
