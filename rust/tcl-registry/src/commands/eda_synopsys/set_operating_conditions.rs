//! `set_operating_conditions` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_operating_conditions",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Set operating conditions for timing.", &["set_operating_conditions ?-library lib? ?-min min_cond? ?-max max_cond? ?condition_name?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
