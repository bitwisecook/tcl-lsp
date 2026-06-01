//! `after` iRules command.
use crate::prelude::*;

/// Dynamic arg-role resolver for `after`.
///
/// `after cancel ...` / `after info ...` take no script.  The
/// timer-scheduling form `after MILLI_SECONDS (-periodic)?
/// (NESTING_SCRIPT)?` carries the deferred body as its trailing
/// argument (never the `-periodic` flag, and never when only the delay
/// is given).  The script runs later from a timer wakeup in its own
/// dispatch context, so `body_kind` is `Structural`.  Mirrors
/// `_after_arg_roles` in `dialects/f5/irules/after.py` (#501).
fn after_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    match args {
        [] | ["cancel" | "info", ..] => Vec::new(),
        _ => {
            let last = args.len() - 1;
            if last >= 1 && args[last] != "-periodic" {
                u8::try_from(last).map_or_else(|_| Vec::new(), |idx| vec![(idx, ArgRole::Body)])
            } else {
                Vec::new()
            }
        }
    }
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "after",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        // The timer form's trailing nesting script is a deferred body
        // (runs from a timer wakeup, not the caller's frame).
        arg_role_resolver: Some(after_arg_roles),
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet::brief(
            "Execute iRules code after a set period of delay.",
            &["after MILLI_SECONDS (-periodic)? (NESTING_SCRIPT)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
