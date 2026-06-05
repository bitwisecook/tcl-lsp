//! `math::statistics::test-Kruskal-Wallis` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::test-Kruskal-Wallis confidence args",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Kruskal-Wallis",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Kruskal-Wallis rank test.",
            synopsis: &["math::statistics::test-Kruskal-Wallis confidence args"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
