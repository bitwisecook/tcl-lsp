//! `yaml::dict2yaml` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "yaml::dict2yaml dictValue ?indent? ?wordwrap?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::dict2yaml",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Convert a Tcl dict to a YAML string.",
            synopsis: &["yaml::dict2yaml dictValue ?indent? ?wordwrap?"],
            snippet: "",
            source: "tcllib yaml package",
            examples: "",
            return_value: "A YAML-formatted string.",
        }),
        forms: FORMS,
        tcllib_package: Some("yaml"),
        required_package: Some("yaml"),
        ..CommandSpec::DEFAULT
    }
}
