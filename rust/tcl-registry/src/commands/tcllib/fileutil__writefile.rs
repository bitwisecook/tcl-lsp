//! `fileutil::writeFile` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::writeFile ?-encoding enc? ?--? file data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::writeFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet {
            summary: "Write data to a file, replacing any existing content.",
            synopsis: &["fileutil::writeFile ?-encoding enc? ?--? file data"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "fileutil::writeFile output.txt \"hello world\"",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
