//! `get_projects` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_projects ?-regexp? ?-nocase? ?-filter expr? ?patterns?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_projects",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get all open projects.",
            &["get_projects ?-regexp? ?-nocase? ?-filter expr? ?patterns?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
