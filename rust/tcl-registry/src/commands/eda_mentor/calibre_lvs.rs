//! `calibre_lvs` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "calibre_lvs",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run Calibre LVS (layout vs schematic).",
            &["calibre_lvs ?-hier? ?-turbo? rule_file"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
