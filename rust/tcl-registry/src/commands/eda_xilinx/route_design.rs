//! `route_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "route_design ?-directive directive? ?-nets net_list? ?-unroute? ?-preserve?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "route_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run routing.",
            &["route_design ?-directive directive? ?-nets net_list? ?-unroute? ?-preserve?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
