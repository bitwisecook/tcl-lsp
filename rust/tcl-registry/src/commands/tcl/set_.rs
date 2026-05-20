//! `set` — read or write a variable.

// VERIFIED: Tcl 9.0.3 manpage set(n) (man3/set.n).

use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Dynamic arg role resolver: getter (1 arg) vs setter (2 args).
fn set_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 2 {
        vec![(0, ArgRole::VarWrite)]
    } else if args.len() == 1 {
        vec![(0, ArgRole::VarRead)]
    } else {
        Vec::new()
    }
}

/// Command spec for `set`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set",
        traits: Traits::NOT_PROC_FACTORY | Traits::BYTE_COMPILED,
        arity: Arity::new(1, 2),
        arg_role_resolver: Some(set_arg_roles),
        assigns_variable_at: Some(0),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Read or write a variable.",
            &["set varName ?newValue?"],
            "Tcl set(1)",
        )),
        lowering_hook: Some(LoweringHookId::Set),
        ..CommandSpec::DEFAULT
    }
}
