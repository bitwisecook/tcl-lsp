//! `fileutil::test` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::test path codes ?msgvar? ?label?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::test",
        dialects: None,
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet {
            summary: "Test file properties.",
            synopsis: &["fileutil::test path codes ?msgvar? ?label?"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_roles: &[(2, ArgRole::VarWrite)],
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
