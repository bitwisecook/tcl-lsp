//! `fileutil::relative` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::relative base dst",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::relative",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Compute a relative path.",
            synopsis: &["fileutil::relative base dst"],
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
