//! `HTML::encode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::encode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "HTML-encodes a string, escaping special characters.",
            synopsis: &["HTML::encode STRING"],
            snippet: "Replaces HTML-special characters (``<``, ``>``, ``&``,\n``\"``, ``'``) with their entity equivalents so the\nstring is safe to embed in an HTML text context.",
            source: "https://clouddocs.f5.com/api/irules/",
            examples: "when HTTP_REQUEST {\n  set user_input [HTTP::query]\n  HTTP::respond 200 content \"<p>[HTML::encode $user_input]</p>\"\n}",
            return_value: "Returns an HTML-escaped string.",
        }),
        // GAP-D2: HTML-escapes its input (and strips CR/LF);
        // re-encoding an HTML-escaped value double-encodes (T106).
        // Mirrors `irules/html__encode.py`.
        taint_transform: Some(TaintColour::HTML_ESCAPED.union(TaintColour::CRLF_FREE)),
        taint_double_encode_colour: Some(TaintColour::HTML_ESCAPED),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTML::encode STRING",
        }],
        ..CommandSpec::DEFAULT
    }
}
