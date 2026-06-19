//! `get_clocks` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "get_clocks ?-regexp? ?-nocase? ?-filter expr? ?patterns?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_clocks",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get clock objects matching a pattern.",
            &["get_clocks ?-regexp? ?-nocase? ?-filter expr? ?patterns?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
