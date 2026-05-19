//! `foreachLine` — iterate over the lines of a text file (Tcl 9.0+, TIP 670).

use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Command spec for `foreachLine`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "foreachLine",
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL90),
        arity: Arity::new(3, 3),
        arg_roles: &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        return_type: Some(TclType::String),
        lowering_hook: Some(LoweringHookId::ForeachLine),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Iterate over the lines of a text file, one line at a time.",
            &["foreachLine varName filename body"],
            "Tcl man page library.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
