//! `foreachLine` — iterate over the lines of a text file (Tcl 9.0+, TIP 670).

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "foreachLine varName filename body",
}];

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
        hover: Some(HoverSnippet {
            summary: "Iterate over the lines of a text file, one line at a time.",
            synopsis: &["foreachLine varName filename body"],
            snippet: "",
            source: "Tcl man page library.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
