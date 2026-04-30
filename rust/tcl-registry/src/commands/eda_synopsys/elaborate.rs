//! `elaborate` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "elaborate",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Elaborate a design from analyzed HDL.", &["elaborate design_name ?-library lib? ?-architecture arch? ?-parameters params? ?-update?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
