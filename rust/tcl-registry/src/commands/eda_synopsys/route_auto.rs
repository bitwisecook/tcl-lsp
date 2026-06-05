//! `route_auto` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "route_auto ?-max_detail_route_iterations n?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "route_auto",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Perform automatic routing.",
            &["route_auto ?-max_detail_route_iterations n?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
