//! `textutil::longestCommonPrefix` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::longestCommonPrefix ?string ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::longestCommonPrefix",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Find the longest common prefix of strings.",
            synopsis: &["textutil::longestCommonPrefix ?string ...?"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("textutil"),
        required_package: Some("textutil"),
        ..CommandSpec::DEFAULT
    }
}
