//! `iapp::make_safe_password` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::make_safe_password",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::make_safe_password`.",
            &["iapp::make_safe_password ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
