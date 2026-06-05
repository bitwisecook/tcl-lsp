//! `math::statistics::spearman-rank-extended` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::spearman-rank-extended sample_a sample_b",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::spearman-rank-extended",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Extended Spearman rank correlation.",
            synopsis: &["math::statistics::spearman-rank-extended sample_a sample_b"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
