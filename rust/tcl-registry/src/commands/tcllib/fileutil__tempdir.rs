//! `fileutil::tempdir` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::tempdir ?path?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::tempdir",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet {
            summary: "Return the path of the system temporary directory.",
            synopsis: &["fileutil::tempdir ?path?"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "The system temporary directory path.",
        }),
        forms: FORMS,
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
