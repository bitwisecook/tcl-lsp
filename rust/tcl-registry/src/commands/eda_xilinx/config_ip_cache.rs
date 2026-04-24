//! `config_ip_cache` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "config_ip_cache",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Configure the IP cache settings.",
            &["config_ip_cache ?-import_from_project? ?-clear_output_repo? ?-cache_location dir?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
