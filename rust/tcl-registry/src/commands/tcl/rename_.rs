//! `rename` — rename or delete a command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rename",
        traits: Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD,
        arity: Arity::exact(2),
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Rename or delete a command.",
            &["rename oldName newName"],
            "Tcl rename(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
