//! `lassign` — assign list elements to variables.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lassign list ?varName ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lassign",
        traits: Traits::FRAMELESS_RUNTIME,
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
hover: Some(HoverSnippet {
    summary: "Assign list elements to variables",
    synopsis: &["lassign list ?varName ...?"],
    snippet: "This command treats the value list as a list and assigns successive elements from that list to the variables given by the varName arguments in order.",
    source: "Tcl man page lassign.n",
    examples: "",
    return_value: "",
}),
        codegen_hook: Some(CodegenHookId::Lassign),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_roles: &[(1, ArgRole::VarWrite), (2, ArgRole::VarWrite), (3, ArgRole::VarWrite), (4, ArgRole::VarWrite), (5, ArgRole::VarWrite), (6, ArgRole::VarWrite), (7, ArgRole::VarWrite), (8, ArgRole::VarWrite), (9, ArgRole::VarWrite), (10, ArgRole::VarWrite), (11, ArgRole::VarWrite), (12, ArgRole::VarWrite), (13, ArgRole::VarWrite), (14, ArgRole::VarWrite), (15, ArgRole::VarWrite), (16, ArgRole::VarWrite), (17, ArgRole::VarWrite), (18, ArgRole::VarWrite), (19, ArgRole::VarWrite)],
        arg_types: &[(0, ArgTypeHint { expected: Some(TclType::List), shimmers: true })],
        ..CommandSpec::DEFAULT
    }
}
