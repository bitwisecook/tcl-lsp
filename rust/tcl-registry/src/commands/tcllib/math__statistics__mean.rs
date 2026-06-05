//! `math::statistics::mean` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "math::statistics::mean data",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "math::statistics::mean",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Compute the arithmetic mean of a list of values.",
            synopsis: &["math::statistics::mean data"],
            snippet: "",
            source: "tcllib math::statistics package",
            examples: "",
            return_value: "The arithmetic mean.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
