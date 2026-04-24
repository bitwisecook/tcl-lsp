//! `registry` — Windows registry access (Windows-only Tcl command).
//!
//! Module name has a trailing underscore to avoid a collision with the
//! crate's [`crate::registry`] module.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "registry",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Windows registry manipulation (Windows-only).",
            &[
                "registry broadcast keyName ?-timeout ms?",
                "registry delete keyName ?valueName?",
                "registry get keyName valueName",
                "registry keys keyName ?pattern?",
                "registry set keyName ?valueName data ?type??",
                "registry type keyName valueName",
                "registry values keyName ?pattern?",
            ],
            "Tcl man page registry.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
