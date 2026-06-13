//! `elaborate` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "elaborate ?design_name? ?-parameters params?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "elaborate",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Elaborate the design hierarchy.",
            &["elaborate ?design_name? ?-parameters params?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
