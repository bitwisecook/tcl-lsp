//! `snit::type` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "snit::type name definition",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::type",
        traits: Traits::CREATES_BARRIER
            | Traits::NEVER_INLINE_BODY
            | Traits::CREATES_DYNAMIC_BARRIER,
        dialects: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Define a new snit type (class).",
            synopsis: &["snit::type name definition"],
            snippet: "Defines a new object type. The definition body contains option, variable, constructor, destructor, method, and typemethod declarations.",
            source: "tcllib snit package",
            examples: "snit::type Dog {\n    option -name Fido\n    method bark {} { return \"Woof!\" }\n}",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_roles: &[(1, ArgRole::Body)],
        tcllib_package: Some("snit"),
        required_package: Some("snit"),
        definition_body: Some(&crate::definer::SNIT_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
