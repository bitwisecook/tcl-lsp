//! `oo::objdefine` — define per-object members.
use crate::prelude::*;

use super::oo_define::oo_define_arg_roles;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::objdefine",
        traits: Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        // `oo::objdefine` has the same body-shape rules as
        // `oo::define`; share the resolver.
        arg_role_resolver: Some(oo_define_arg_roles),
        return_type: Some(TclType::String),
        // SYNC2: same structural-body rule as `oo::define` — bodies
        // run in a per-object definition context, not the caller's
        // frame.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet::brief(
            "Define per-object members.",
            &["oo::objdefine objectName ?definition?"],
            "Tcl oo::objdefine(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
