//! `all_fanin` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "all_fanin ?-to objects? ?-flat? ?-startpoints_only? ?-only_cells?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "all_fanin",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return all fanin of a pin/port.",
            &["all_fanin ?-to objects? ?-flat? ?-startpoints_only? ?-only_cells?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
