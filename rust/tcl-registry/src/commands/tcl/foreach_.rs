//! `foreach` — iterate over one or more lists.

use crate::prelude::*;

/// Dynamic arg role resolver: last argument is always the body.
#[allow(clippy::cast_possible_truncation)]
fn foreach_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 3 {
        vec![((args.len() - 1) as u8, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

/// Command spec for `foreach`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "foreach",
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER,
        arity: Arity::at_least(3),
        arg_role_resolver: Some(foreach_arg_roles),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Iterate over one or more lists.",
            &["foreach varlist1 list1 ?varlist2 list2 ...? body"],
            "Tcl foreach(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
