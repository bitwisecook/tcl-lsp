//! `set_host_options` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_host_options",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Configure multi-core and host settings.",
            &["set_host_options ?-max_cores n?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
