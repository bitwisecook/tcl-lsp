//! `iapp::conf` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::conf",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::conf`.",
            &["iapp::conf ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
