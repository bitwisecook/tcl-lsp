//! `tcl_startOfPreviousWord` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_startOfPreviousWord",
        traits: Traits::PURE | Traits::OVERRIDABLE_LIBRARY_PROC,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Return the index of the first start-of-word before *start* in *str*.",
            synopsis: &["tcl_startOfPreviousWord str start"],
            snippet: "",
            source: "Tcl stdlib auto-loaded utility",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
