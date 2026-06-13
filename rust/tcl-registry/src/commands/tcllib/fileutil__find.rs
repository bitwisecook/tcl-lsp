//! `fileutil::find` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::find ?basedir? ?filtercmd?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::find",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Recursively find files matching a filter.",
            synopsis: &["fileutil::find ?basedir? ?filtercmd?"],
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
