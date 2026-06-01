//! `serverside` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "serverside",
        traits: Traits::IS_SIDE_SWITCH,
        dialects: Some(DialectSet::IRULES),
        // `serverside (NESTING_SCRIPT)?` — the bare query form (0 args,
        // returns 1/0) or a single optional nesting-script body (#501).
        arity: Arity::new(0, 1),
        // The optional nesting script (index 0) is a body evaluated in
        // the server-side context; it runs synchronously in the
        // caller's frame, so the default `BodyKind::Plain` applies.  The
        // 0-arg query form has no arg at index 0, so the role simply
        // does not apply there.
        arg_roles: &[(0, ArgRole::Body)],
        hover: Some(HoverSnippet::brief(
            "Causes the specified iRule command to be evaluated under the server-side context",
            &["serverside (NESTING_SCRIPT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
