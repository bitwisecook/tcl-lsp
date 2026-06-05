//! `fileutil::removeFromFile` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::removeFromFile ?options? file at n",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::removeFromFile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet {
            summary: "Remove data from a file.",
            synopsis: &["fileutil::removeFromFile ?options? file at n"],
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
