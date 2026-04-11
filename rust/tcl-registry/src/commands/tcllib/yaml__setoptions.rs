//! `yaml::setOptions` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::setOptions",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Set options for YAML processing.",
            &["yaml::setOptions optionDict"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
