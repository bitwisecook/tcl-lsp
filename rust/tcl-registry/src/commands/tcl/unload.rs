//! `unload` — unload a shared library extension.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unload",
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Unload a shared library extension.",
            &["unload ?-keeplibrary? ?-nocomplain? fileName ?prefix? ?interp?"],
            "Tcl unload(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
