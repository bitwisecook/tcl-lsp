//! `proc` — define a procedure.
//
// VERIFIED: Tcl 9.0.3 manpage proc(n) (man3/proc.n).

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "proc name args body",
}];

/// Command spec for `proc`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "proc",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE
            | Traits::NEVER_INLINE_BODY
            | Traits::IRULES_TOP_LEVEL_ONLY
            | Traits::WASM_EMITS_NOTHING,
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        return_type: Some(TclType::String),
        lowering_hook: Some(LoweringHookId::Proc),
        // SYNC2: a `proc` body runs in the proc's own frame on each
        // call — never the caller's frame.  Stamping `Structural`
        // here lets generic `body_indices_to_skip` consumers (SSA,
        // dead-store, def-use) treat `proc` like every other
        // structural-body command without a string-match special
        // case.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Create a Tcl procedure.",
            synopsis: &["proc name args body"],
            snippet: "`args` is a formal parameter list; `body` executes in a new call frame.",
            source: "Tcl proc(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
