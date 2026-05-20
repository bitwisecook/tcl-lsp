//! `lreplace` — replace elements in a list.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lreplace",
        traits: Traits::BYTE_COMPILED | Traits::PURE,
        arity: Arity::at_least(3),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet::brief(
            "Replace elements in a list.",
            &["lreplace list first last ?element ...?"],
            "Tcl lreplace(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
