//! `report_net` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_net",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report net information.",
            &["report_net ?-nosplit? ?-connections? ?-verbose? ?net_list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
