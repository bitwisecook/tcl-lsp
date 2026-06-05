//! `fileutil::stripN` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::stripN path n",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::stripN",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Remove N leading path components.",
            synopsis: &["fileutil::stripN path n"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
