//! `iapp::tmos_version` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::tmos_version",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::tmos_version`.",
            &["iapp::tmos_version ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
