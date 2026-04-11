//! `iapp::destination` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::destination",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::destination`.",
            &["iapp::destination ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
