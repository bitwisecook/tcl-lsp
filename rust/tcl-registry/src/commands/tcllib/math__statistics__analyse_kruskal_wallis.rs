//! `math::statistics::analyse-Kruskal-Wallis` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::analyse-Kruskal-Wallis args",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::analyse-Kruskal-Wallis",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Analyse Kruskal-Wallis results.",
            synopsis: &["math::statistics::analyse-Kruskal-Wallis args"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
