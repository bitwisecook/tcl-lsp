//! `yaml::yaml2dict` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::yaml2dict",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Parse a YAML string and return a Tcl dict.",
            &["yaml::yaml2dict ?-file? yamlText"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
