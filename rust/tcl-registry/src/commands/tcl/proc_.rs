//! `proc` — define a procedure.
//
// VERIFIED: Tcl 9.0.3 manpage proc(n) (man3/proc.n).

use crate::prelude::*;

/// Command spec for `proc`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "proc",
        traits: Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE
            | Traits::NEVER_INLINE_BODY
            | Traits::IRULES_TOP_LEVEL_ONLY,
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        return_type: Some(TclType::String),
        // SYNC2: a `proc` body runs in the proc's own frame on each
        // call — never the caller's frame.  Stamping `Structural`
        // here lets generic `body_indices_to_skip` consumers (SSA,
        // dead-store, def-use) treat `proc` like every other
        // structural-body command without a string-match special
        // case.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet::brief(
            "Define a procedure.",
            &["proc name args body"],
            "Tcl proc(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
