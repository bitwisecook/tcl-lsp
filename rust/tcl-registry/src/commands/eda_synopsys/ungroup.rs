//! `ungroup` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ungroup ?-all? ?-flatten? ?-start_level n? ?-simple_names? ?cells?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ungroup",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Dissolve hierarchy of specified cells.",
            &["ungroup ?-all? ?-flatten? ?-start_level n? ?-simple_names? ?cells?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
