//! `parray` — print an array.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "parray arrayName ?pattern?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "parray",
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::VarRead)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Print an array's keys and values",
            synopsis: &["parray arrayName ?pattern?"],
            snippet: "",
            source: "Tcl man page library.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
