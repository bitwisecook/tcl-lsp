//! `calibre` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "calibre",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run Calibre physical verification.",
            &["calibre ?-drc | -lvs | -pex? ?-hier? ?-turbo? ?-hyper? rule_file"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
