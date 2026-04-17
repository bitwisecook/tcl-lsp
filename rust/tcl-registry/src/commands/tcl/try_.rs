//! `try` — trap and process errors and exceptions.

use crate::prelude::*;

/// Dynamic arg role resolver for `try`/`on`/`trap`/`finally`.
fn try_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut roles = Vec::new();
    if !args.is_empty() {
        roles.push((0, ArgRole::Body));
    }
    let mut i: usize = 1;
    while i < args.len() {
        let kw = args[i];
        if kw == "finally" && i + 1 < args.len() {
            if let Ok(idx) = u8::try_from(i + 1) {
                roles.push((idx, ArgRole::Body));
            }
            i += 2;
        } else if (kw == "on" || kw == "trap") && i + 3 < args.len() {
            if let Ok(idx) = u8::try_from(i + 3) {
                roles.push((idx, ArgRole::Body));
            }
            i += 4;
        } else {
            i += 1;
        }
    }
    roles
}

/// Command spec for `try`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "try",
        traits: Traits::CONTROL_FLOW | Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(try_arg_roles),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Trap and process errors and exceptions.",
            &["try body ?handler...? ?finally script?"],
            "Tcl try(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
