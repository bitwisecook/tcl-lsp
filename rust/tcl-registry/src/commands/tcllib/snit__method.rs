//! `snit::method` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "snit::method type name arglist body",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::method",
        dialects: None,
        arity: Arity::exact(4),
        // Snit method bodies run in a snit dispatch context,
        // not the caller's frame.  Body at index 3.
        arg_roles: &[(2, ArgRole::ParamList), (3, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Define an instance method outside a type definition body.",
            synopsis: &["snit::method type name arglist body"],
            snippet: "",
            source: "tcllib snit package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("snit"),
        required_package: Some("snit"),
        ..CommandSpec::DEFAULT
    }
}
