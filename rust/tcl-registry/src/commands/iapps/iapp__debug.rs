//! `iapp::debug` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::debug",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::debug`.",
            &["iapp::debug ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
