//! `lmap` — iterate over all elements in one or more lists and collect results.

use crate::prelude::*;

/// Dynamic arg role resolver: last argument is the body script.
fn lmap_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() >= 3 {
        u8::try_from(args.len() - 1)
            .map(|last| vec![(last, ArgRole::Body)])
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Command spec for `lmap`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lmap",
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::LOOP_LIST_HEADER,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(3),
        arg_role_resolver: Some(lmap_arg_roles),
        lowering_hook: Some(crate::hooks::LoweringHookId::Lmap),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Iterate over all elements in one or more lists and collect results.",
            &[
                "lmap varname list body",
                "lmap varlist1 list1 ?varlist2 list2 ...? body",
            ],
            "Tcl lmap(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
