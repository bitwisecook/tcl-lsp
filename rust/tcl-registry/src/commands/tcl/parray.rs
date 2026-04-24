//! `parray` — print an array.
use crate::prelude::*;
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
        hover: Some(HoverSnippet::brief(
            "Print array contents.",
            &["parray arrayName ?pattern?"],
            "Tcl parray(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
