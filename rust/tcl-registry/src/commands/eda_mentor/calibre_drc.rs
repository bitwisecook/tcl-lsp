//! `calibre_drc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "calibre_drc",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run Calibre DRC (design rule check).",
            &["calibre_drc ?-hier? ?-turbo? rule_file"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
