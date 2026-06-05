//! `math::statistics::histogram` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::histogram limits values ?weights?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::histogram",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Compute a histogram.",
            synopsis: &["math::statistics::histogram limits values ?weights?"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
