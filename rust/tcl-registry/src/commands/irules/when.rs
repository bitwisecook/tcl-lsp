//! `when` iRules command.
use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Dynamic arg-role resolver for `when EVENT ?priority? { body }`.
///
/// The last argument is always the event-handler body.  The
/// optional `priority` token sits between `EVENT` and `BODY`.
/// Mirrors `_when_arg_roles` in
/// `core/commands/registry/irules/when.py:25-29`.
fn when_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 2 {
        if let Ok(idx) = u8::try_from(args.len() - 1) {
            return vec![(idx, ArgRole::Body)];
        }
    }
    Vec::new()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "when",
        traits: Traits::LANGUAGE_KEYWORD | Traits::IS_EVENT_HANDLER | Traits::IRULES_TOP_LEVEL_ONLY,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(2, 6),
        arg_role_resolver: Some(when_arg_roles),
        lowering_hook: Some(LoweringHookId::When),
        // SYNC2: iRules event handler bodies run in the event
        // dispatcher's frame — separate from the top-level rule
        // file's evaluation context.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet::brief(
            "Declare an iRules event handler block.",
            &["when EVENT { body }"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
