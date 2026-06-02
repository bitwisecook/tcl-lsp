//! `peer` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "peer",
        traits: Traits::IS_SIDE_SWITCH,
        dialects: Some(DialectSet::IRULES),
        // `peer NESTING_SCRIPT` — unlike clientside/serverside, peer has
        // no bare query form, so the script body is required: exactly
        // one argument at index 0 (#501).
        arity: Arity::new(1, 1),
        // The nesting script (index 0) is a body evaluated under the
        // peer-side context; it runs synchronously in the caller's
        // frame, so the default `BodyKind::Plain` applies.
        arg_roles: &[(0, ArgRole::Body)],
        hover: Some(HoverSnippet::brief(
            "Causes the specified iRule commands to be evaluated under the peer-side context.",
            &["peer NESTING_SCRIPT"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
