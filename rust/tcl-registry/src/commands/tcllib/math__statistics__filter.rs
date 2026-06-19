//! `math::statistics::filter` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::filter varname data expression",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::filter",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(3),
        hover: Some(HoverSnippet {
            summary: "Filter data by expression.",
            synopsis: &["math::statistics::filter varname data expression"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Expr)],
        tcllib_package: Some("math::statistics"),
        required_package: Some("math::statistics"),
        ..CommandSpec::DEFAULT
    }
}
