//! `cmdline::getoptions` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cmdline::getoptions argvVar optlist ?usage?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::getoptions",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Parse all command-line options according to a specification.",
            synopsis: &["cmdline::getoptions argvVar optlist ?usage?"],
            snippet: "Parses the argument list against the option specification and returns a dictionary of option values.",
            source: "tcllib cmdline package",
            examples: "set options [cmdline::getoptions argv {\n    {verbose \"Turn on verbose output\"}\n    {output.arg \"\" \"Output file\"}\n}]",
            return_value: "A dictionary of parsed option values.",
        }),
        forms: FORMS,
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
