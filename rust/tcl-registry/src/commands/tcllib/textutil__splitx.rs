//! `textutil::splitx` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::splitx string ?regexp?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::splitx",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
hover: Some(HoverSnippet {
    summary: "Split a string using a regexp pattern as delimiter.",
    synopsis: &["textutil::splitx string ?regexp?"],
    snippet: "Like the core split command but accepts a regular expression as the delimiter pattern.",
    source: "tcllib textutil package",
    examples: "set parts [textutil::splitx $text {\\s+}]",
    return_value: "A list of substrings.",
}),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
