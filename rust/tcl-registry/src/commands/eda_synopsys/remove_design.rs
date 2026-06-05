//! `remove_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "remove_design ?-all? ?-designs? ?design_list?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "remove_design",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Remove a design from memory.",
            &["remove_design ?-all? ?-designs? ?design_list?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
