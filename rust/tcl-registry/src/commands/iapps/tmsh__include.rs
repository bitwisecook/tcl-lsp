//! `tmsh::include` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tmsh::include",
        dialects: Some(DialectSet::IAPPS),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Runs the Tcl command ``eval`` on the specified script.",
            &["tmsh::include <script_name>"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
