//! `fileutil::install` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::install ?-m mode? source destination",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::install",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Copy a file and optionally set permissions.",
            synopsis: &["fileutil::install ?-m mode? source destination"],
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
