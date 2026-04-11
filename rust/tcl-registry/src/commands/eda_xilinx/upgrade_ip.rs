//! `upgrade_ip` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "upgrade_ip",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Upgrade IP cores to a newer version.",
            &["upgrade_ip ?-srcset srcset? ?-quiet? ?objects?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
