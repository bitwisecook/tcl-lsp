//! `iapp::upgrade_template` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::upgrade_template",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::upgrade_template`.",
            &["iapp::upgrade_template ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
