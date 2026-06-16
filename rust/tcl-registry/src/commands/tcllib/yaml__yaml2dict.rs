//! `yaml::yaml2dict` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "yaml::yaml2dict ?-file? yamlText",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::yaml2dict",
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Parse a YAML string and return a Tcl dict.",
            synopsis: &["yaml::yaml2dict ?-file? yamlText"],
            snippet: "",
            source: "tcllib yaml package",
            examples: "set data [yaml::yaml2dict $yamlString]",
            return_value: "A Tcl dict representing the YAML structure.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("yaml"),
        required_package: Some("yaml"),
        ..CommandSpec::DEFAULT
    }
}
