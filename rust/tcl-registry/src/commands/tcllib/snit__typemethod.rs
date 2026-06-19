//! `snit::typemethod` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "snit::typemethod type name arglist body",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::typemethod",
        dialects: None,
        arity: Arity::exact(4),
        // SYNC2: snit typemethod bodies run in a dispatch context.
        arg_roles: &[(2, ArgRole::ParamList), (3, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Define a type method outside a type definition body.",
            synopsis: &["snit::typemethod type name arglist body"],
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
