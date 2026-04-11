//! `load_package` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load_package",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Load a Quartus Tcl package.",
            &["load_package package_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
