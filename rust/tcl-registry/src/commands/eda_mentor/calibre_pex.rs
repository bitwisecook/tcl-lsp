//! `calibre_pex` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "calibre_pex",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run Calibre PEX (parasitic extraction).",
            &["calibre_pex ?-hier? ?-turbo? rule_file"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
