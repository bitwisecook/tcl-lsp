//! `iapp::upgrade` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::upgrade",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::upgrade`.",
            &["iapp::upgrade ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
