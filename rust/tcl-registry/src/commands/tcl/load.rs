//! `load` — load a shared library extension.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load",
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Load a shared library extension.",
            &["load fileName ?prefix? ?interp?"],
            "Tcl load(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
