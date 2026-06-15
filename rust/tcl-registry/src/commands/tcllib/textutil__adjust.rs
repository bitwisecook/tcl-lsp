//! `textutil::adjust` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::adjust string ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::adjust",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Adjust a text block to a given line length.",
            synopsis: &[
                "textutil::adjust string ?-length num? ?-justify left|right|center|plain? ?-hyphenate bool? ?-full bool? ?-strictlength bool?",
            ],
            snippet: "",
            source: "tcllib textutil package",
            examples: "set wrapped [textutil::adjust $text -length 72]",
            return_value: "The adjusted text.",
        }),
        forms: FORMS,
        tcllib_package: Some("textutil"),
        required_package: Some("textutil"),
        ..CommandSpec::DEFAULT
    }
}
