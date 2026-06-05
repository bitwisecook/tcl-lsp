//! `get_ips` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_ips ?-regexp? ?-nocase? ?-filter expr? ?patterns?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_ips",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get IP core instances.",
            &["get_ips ?-regexp? ?-nocase? ?-filter expr? ?patterns?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
