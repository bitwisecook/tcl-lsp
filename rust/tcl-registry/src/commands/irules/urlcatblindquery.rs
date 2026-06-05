//! `urlcatblindquery` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "urlcatblindquery",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Query the encrypted URL's hash for URL categorization.",
            synopsis: &["urlcatblindquery ENCRYPTED_URL_STRING"],
            snippet: "Query the encrypted URL's hash for URL categorization",
            source: "https://clouddocs.f5.com/api/irules/urlcatblindquery.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "urlcatblindquery ENCRYPTED_URL_STRING",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::DataGroup,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
