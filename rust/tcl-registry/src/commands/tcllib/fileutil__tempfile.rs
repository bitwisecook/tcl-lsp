//! `fileutil::tempfile` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::tempfile ?prefix?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::tempfile",
        dialects: None,
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Create a temporary file and return its path.",
            synopsis: &["fileutil::tempfile ?prefix?"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "The path to the new temporary file.",
        }),
        forms: FORMS,
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
