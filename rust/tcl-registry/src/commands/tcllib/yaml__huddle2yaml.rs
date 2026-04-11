//! `yaml::huddle2yaml` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::huddle2yaml",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Convert a huddle object to a YAML string.",
            &["yaml::huddle2yaml huddle ?indent? ?wordwrap?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
