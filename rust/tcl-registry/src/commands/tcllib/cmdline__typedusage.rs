//! `cmdline::typedUsage` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "cmdline::typedUsage optlist ?usage?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cmdline::typedUsage",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Generate a usage string from a typed option specification.",
            synopsis: &["cmdline::typedUsage optlist ?usage?"],
            snippet: "",
            source: "tcllib cmdline package",
            examples: "",
            return_value: "A formatted usage string.",
        }),
        forms: FORMS,
        tcllib_package: Some("cmdline"),
        required_package: Some("cmdline"),
        ..CommandSpec::DEFAULT
    }
}
