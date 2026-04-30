//! `iapp::pool_members` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::pool_members",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::pool_members`.",
            &["iapp::pool_members ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
