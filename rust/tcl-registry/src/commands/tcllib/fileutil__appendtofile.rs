//! `fileutil::appendToFile` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::appendToFile ?options? file data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::appendToFile",
        dialects: None,
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Append data to a file.",
            synopsis: &["fileutil::appendToFile ?options? file data"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
