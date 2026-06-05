//! `fileutil::findByPattern` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::findByPattern basedir ?-glob|-regexp? ?--? patterns",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::findByPattern",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Find files matching glob or regexp patterns.",
            synopsis: &["fileutil::findByPattern basedir ?-glob|-regexp? ?--? patterns"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
