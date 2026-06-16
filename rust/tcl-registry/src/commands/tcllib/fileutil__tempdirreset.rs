//! `fileutil::tempdirReset` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::tempdirReset",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::tempdirReset",
        dialects: None,
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Reset the cached temporary directory path.",
            synopsis: &["fileutil::tempdirReset"],
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
