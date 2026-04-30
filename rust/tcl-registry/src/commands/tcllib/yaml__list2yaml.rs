//! `yaml::list2yaml` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::list2yaml",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Convert a Tcl list to a YAML string.",
            &["yaml::list2yaml listValue ?indent? ?wordwrap?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
