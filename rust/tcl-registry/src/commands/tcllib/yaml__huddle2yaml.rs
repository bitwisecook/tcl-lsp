//! `yaml::huddle2yaml` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "yaml::huddle2yaml huddle ?indent? ?wordwrap?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::huddle2yaml",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Convert a huddle object to a YAML string.",
            synopsis: &["yaml::huddle2yaml huddle ?indent? ?wordwrap?"],
            snippet: "",
            source: "tcllib yaml package",
            examples: "",
            return_value: "A YAML-formatted string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
