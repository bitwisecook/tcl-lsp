//! `math::statistics::samplescount` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::samplescount varname list ?expression?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::samplescount",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Count samples matching expression.",
            synopsis: &["math::statistics::samplescount varname list ?expression?"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Expr)],
        ..CommandSpec::DEFAULT
    }
}
