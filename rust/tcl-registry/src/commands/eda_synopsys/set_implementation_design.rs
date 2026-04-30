//! `set_implementation_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_implementation_design",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Set the implementation design for formal verification.",
            &["set_implementation_design design_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
