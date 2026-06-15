//! `URI::compare` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::compare",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Compares two URI's for equality.",
            synopsis: &["URI::compare URI_STRING URI_STRING"],
            snippet: "Compares two URI's as recommended by RFC2616 section 3.2.3.\n\n3.2.3 URI Comparison\n\n   When comparing two URIs to decide if they match or not, a client\n   SHOULD use a case-sensitive octet-by-octet comparison of the entire\n   URIs, with these exceptions:\n\n      - A port that is empty or not given is equivalent to the default\n        port for that URI-reference;\n\n        - Comparisons of host names MUST be case-insensitive;\n\n        - Comparisons of scheme names MUST be case-insensitive;\n\n        - An empty abs_path is equivalent to an abs_path of \"/\".",
            source: "https://clouddocs.f5.com/api/irules/URI__compare.html",
            examples: "when HTTP_REQUEST {\n  set uri_to_check \"/dir1/somepath\"\n  if { [URI::compare [HTTP::uri] $uri_to_check] } {\n    log local0. \"URI's are equal!\"\n  }\n}",
            return_value: "Returns 1 if URIs match; 0 otherwise.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "URI::compare URI_STRING URI_STRING",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
