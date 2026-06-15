//! `cmdline::getopt` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cmdline::getopt argvVar optstring optVar valVar",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getopt",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet {
            summary: "Parse a single command-line option.",
            synopsis: &["cmdline::getopt argvVar optstring optVar valVar"],
            snippet: "Processes a single option from the argument list. Returns 1 if an option was found, 0 if no more options, or -1 on error.",
            source: "tcllib cmdline package",
            examples: "",
            return_value: "1 on success, 0 when done, -1 on error.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
