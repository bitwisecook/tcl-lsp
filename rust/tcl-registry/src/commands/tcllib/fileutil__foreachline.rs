//! `fileutil::foreachLine` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::foreachLine var filename cmd",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::foreachLine",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet {
            summary: "Iterate over each line of a file.",
            synopsis: &["fileutil::foreachLine var filename cmd"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        tcllib_package: Some("fileutil"),
        required_package: Some("fileutil"),
        ..CommandSpec::DEFAULT
    }
}
