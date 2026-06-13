//! `cmdline::typedGetoptions` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cmdline::typedGetoptions argvVar optlist ?usage?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::typedGetoptions",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Parse all typed command-line options according to a specification.",
            synopsis: &["cmdline::typedGetoptions argvVar optlist ?usage?"],
            snippet: "",
            source: "tcllib cmdline package",
            examples: "",
            return_value: "A dictionary of parsed option values.",
        }),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::VarWrite)],
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
