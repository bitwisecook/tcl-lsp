//! `edit_pin` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "edit_pin ?-pin pin_list? ?-edge edge? ?-layer layer? ?-start coord? ?-end coord? ?-fixedPin? ?-snap?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "edit_pin",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Place or move pins.",
            &[
                "edit_pin ?-pin pin_list? ?-edge edge? ?-layer layer? ?-start coord? ?-end coord? ?-fixedPin? ?-snap?",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
