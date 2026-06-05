//! `AM::media_playlist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AM::media_playlist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "F5 iRules command `AM::media_playlist`.",
            synopsis: &["AM::media_playlist"],
            snippet: "",
            source: "https://clouddocs.f5.com/api/irules/AM__media_playlist.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "AM::media_playlist",
        }],
        ..CommandSpec::DEFAULT
    }
}
