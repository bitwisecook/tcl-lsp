//! `tcl_startOfNextWord` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl_startOfNextWord",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Return the index of the first start-of-word after *start* in *str*.",
            synopsis: &["tcl_startOfNextWord str start"],
            snippet: "",
            source: "Tcl stdlib auto-loaded utility",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
