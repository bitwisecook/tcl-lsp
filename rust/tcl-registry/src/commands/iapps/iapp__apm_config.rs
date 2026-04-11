//! `iapp::apm_config` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "iapp::apm_config",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "F5 iApps utility command `iapp::apm_config`.",
            &["iapp::apm_config ?arg ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
