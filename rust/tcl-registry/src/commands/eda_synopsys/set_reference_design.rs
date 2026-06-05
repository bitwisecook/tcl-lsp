//! `set_reference_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_reference_design design_name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_reference_design",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Set the reference design for formal verification.",
            &["set_reference_design design_name"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
