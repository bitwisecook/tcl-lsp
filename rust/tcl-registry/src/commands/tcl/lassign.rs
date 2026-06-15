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

/// D4-F2: `lassign list ?varName ...?` accepts variable-name args from index 1
/// onward to the end of the call.  Resolve `VarWrite` dynamically so calls with
/// arbitrarily many vars don't false-fire W210 on the unmodelled tail.
/// Mirrors `dialects/tcl/lassign.py::_lassign_arg_roles`.
fn lassign_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    (1..args.len())
        .filter_map(|i| u8::try_from(i).ok().map(|i| (i, ArgRole::VarWrite)))
        .collect()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lassign",
        traits: Traits::FRAMELESS_RUNTIME | Traits::FRAME_HASH_BUILTIN,
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
        arg_role_resolver: Some(lassign_arg_roles),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
