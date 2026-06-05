//! `cmdline::getArgv0` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cmdline::getArgv0",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getArgv0",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Return the application name from the command line.",
            synopsis: &["cmdline::getArgv0"],
            snippet: "",
            source: "tcllib cmdline package",
            examples: "",
            return_value: "The application name.",
        }),
        forms: FORMS,
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
