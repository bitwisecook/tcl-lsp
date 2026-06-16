//! `fileutil::cat` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::cat ?-encoding enc? ?--? file ...",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::cat",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Return the contents of one or more files.",
            synopsis: &["fileutil::cat ?-encoding enc? ?--? file ..."],
            snippet: "Reads the contents of the named files and returns them as a single concatenated string.",
            source: "tcllib fileutil package",
            examples: "set contents [fileutil::cat myfile.txt]",
            return_value: "The concatenated file contents.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
