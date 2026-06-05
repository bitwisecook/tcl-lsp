//! `yaml::list2yaml` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "yaml::list2yaml listValue ?indent? ?wordwrap?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::list2yaml",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Convert a Tcl list to a YAML string.",
            synopsis: &["yaml::list2yaml listValue ?indent? ?wordwrap?"],
            snippet: "",
            source: "tcllib yaml package",
            examples: "",
            return_value: "A YAML-formatted string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
