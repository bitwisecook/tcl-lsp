//! `oo::objdefine` — define per-object members.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::objdefine object defScript",
}];

use super::oo_define::oo_define_arg_roles;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::objdefine",
        traits: Traits::NOT_PROC_FACTORY | Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        // `oo::objdefine` has the same body-shape rules as
        // `oo::define`; share the resolver.
        arg_role_resolver: Some(oo_define_arg_roles),
        return_type: Some(TclType::String),
        // Same structural-body rule as `oo::define` — bodies
        // run in a per-object definition context, not the caller's
        // frame.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "define and configure classes and objects",
            synopsis: &[
                "oo::objdefine object defScript",
                "oo::objdefine object subcommand arg ?arg ...?",
                "oo::objdefine objectName ?definition?",
            ],
            snippet: "The oo::define command is used to control the configuration of classes, and the oo::objdefine command is used to control the configuration of objects (including classes as instance objects), with the configuration being applied to the entity named in the class or the object argument.",
            source: "Tcl man page define.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
