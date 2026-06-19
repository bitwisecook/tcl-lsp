//! `load` — load a shared library extension.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "load ?-global? ?-lazy? ?--? fileName",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::BYTE_COMPILED,
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
        hover: Some(HoverSnippet {
            summary: "Load machine code and initialize new commands",
            synopsis: &[
                "load ?-global? ?-lazy? ?--? fileName",
                "load ?-global? ?-lazy? ?--? fileName prefix",
                "load ?-global? ?-lazy? ?--? fileName prefix interp",
                "load fileName ?prefix? ?interp?",
            ],
            snippet: "This command loads binary code from a file into the application's address space and calls an initialization procedure in the library to incorporate it into an interpreter.",
            source: "Tcl man page load.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
