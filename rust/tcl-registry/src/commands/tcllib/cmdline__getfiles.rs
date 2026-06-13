//! `cmdline::getfiles` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cmdline::getfiles patterns quiet",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getfiles",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Expand file patterns into a list of matching files.",
            synopsis: &["cmdline::getfiles patterns quiet"],
            snippet: "",
            source: "tcllib cmdline package",
            examples: "",
            return_value: "A list of matching file paths.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
