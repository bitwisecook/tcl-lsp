//! `route_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "route_design ?-global_detail?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "route_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Route the design.",
            &["route_design ?-global_detail?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
