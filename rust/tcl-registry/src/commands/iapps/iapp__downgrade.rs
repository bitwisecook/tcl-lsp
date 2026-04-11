//! `iapp::downgrade` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::downgrade",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::downgrade`.",
            &["iapp::downgrade ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
