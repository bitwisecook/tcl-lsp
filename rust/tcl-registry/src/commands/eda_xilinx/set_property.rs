//! `set_property` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_property property_name value objects",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_property",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet::brief(
            "Set a property on a Vivado design object.",
            &["set_property property_name value objects"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
