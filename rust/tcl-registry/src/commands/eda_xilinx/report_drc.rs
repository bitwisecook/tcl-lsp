//! `report_drc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_drc",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run and report design rule checks.",
            &["report_drc ?-ruledecks ruledeck_list? ?-file file? ?-name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
