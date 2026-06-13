//! `fileutil::replaceInFile` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::replaceInFile ?options? file at n data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::replaceInFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(4),
        hover: Some(HoverSnippet {
            summary: "Replace data in a file.",
            synopsis: &["fileutil::replaceInFile ?options? file at n data"],
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
