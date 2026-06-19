//! `tcl_endOfWord` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_endOfWord",
        traits: Traits::PURE | Traits::OVERRIDABLE_LIBRARY_PROC,
        dialects: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Return the index of the first end-of-word after *start* in *str*.",
            synopsis: &["tcl_endOfWord str start"],
            snippet: "",
            source: "Tcl stdlib auto-loaded utility",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
