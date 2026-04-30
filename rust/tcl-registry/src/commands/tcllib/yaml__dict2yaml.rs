//! `yaml::dict2yaml` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::dict2yaml",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Convert a Tcl dict to a YAML string.",
            &["yaml::dict2yaml dictValue ?indent? ?wordwrap?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
