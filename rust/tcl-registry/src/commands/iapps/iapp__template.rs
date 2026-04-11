//! `iapp::template` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::template",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::template`.",
            &["iapp::template ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
