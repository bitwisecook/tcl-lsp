//! `yaml::yaml2huddle` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::yaml2huddle",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Parse a YAML string and return a huddle object.",
            &["yaml::yaml2huddle ?-file? yamlText"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
