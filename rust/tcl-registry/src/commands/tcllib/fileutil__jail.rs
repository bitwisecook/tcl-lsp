//! `fileutil::jail` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::jail base path",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::jail",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Confine a path under a base directory.",
            synopsis: &["fileutil::jail base path"],
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
