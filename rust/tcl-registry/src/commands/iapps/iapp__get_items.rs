//! `iapp::get_items` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::get_items",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::get_items`.",
            &["iapp::get_items ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
