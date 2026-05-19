//! `load` — load a shared library extension.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load",
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        // Mirrors ``core/commands/registry/tcl/load.py``.  ``--`` is
        // the option terminator that drives W304's
        // ``resolve_option_terminator`` lookup; the registry also
        // surfaces ``-global`` / ``-lazy`` for completion.
        options: &[
            OptionSpec {
                name: "-global",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-lazy",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Load a shared library extension.",
            &["load fileName ?prefix? ?interp?"],
            "Tcl load(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
