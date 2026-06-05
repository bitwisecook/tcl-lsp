//! `cmdline::typedGetopt` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cmdline::typedGetopt argvVar optstring optVar valVar",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::typedGetopt",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(4),
        hover: Some(HoverSnippet {
            summary: "Parse a single typed command-line option.",
            synopsis: &["cmdline::typedGetopt argvVar optstring optVar valVar"],
            snippet: "",
            source: "tcllib cmdline package",
            examples: "",
            return_value: "1 on success, 0 when done, -1 on error.",
        }),
        forms: FORMS,
        arg_roles: &[
            (0, ArgRole::VarWrite),
            (2, ArgRole::VarWrite),
            (3, ArgRole::VarWrite),
        ],
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
