//! `textutil::longestCommonPrefixList` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::longestCommonPrefixList list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::longestCommonPrefixList",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Find the longest common prefix of a list of strings.",
            synopsis: &["textutil::longestCommonPrefixList list"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
