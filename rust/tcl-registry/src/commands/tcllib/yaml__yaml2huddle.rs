//! `yaml::yaml2huddle` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "yaml::yaml2huddle ?-file? yamlText",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yaml::yaml2huddle",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Parse a YAML string and return a huddle object.",
            synopsis: &["yaml::yaml2huddle ?-file? yamlText"],
            snippet: "",
            source: "tcllib yaml package",
            examples: "",
            return_value: "A huddle object representing the YAML structure.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("yaml"),
        required_package: Some("yaml"),
        ..CommandSpec::DEFAULT
    }
}
