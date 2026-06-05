//! `math::statistics::test-Wilcoxon` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::test-Wilcoxon sample_a sample_b",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::test-Wilcoxon",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Wilcoxon signed-rank test.",
            synopsis: &["math::statistics::test-Wilcoxon sample_a sample_b"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("math::statistics"),
        required_package: Some("math::statistics"),
        ..CommandSpec::DEFAULT
    }
}
