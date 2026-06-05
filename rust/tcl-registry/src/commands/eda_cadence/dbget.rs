//! `dbGet` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "dbGet object_spec.attribute ?-regexp pattern? ?-e?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dbGet",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get a design database object attribute (legacy).",
            &["dbGet object_spec.attribute ?-regexp pattern? ?-e?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
